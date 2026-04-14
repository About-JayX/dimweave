use crate::daemon::{
    types::{BridgeMessage, MessageStatus},
    SharedState,
};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc;

type WsSend = mpsc::Sender<String>;

/// Dispatch a `item/tool/call` dynamic-tool invocation from Codex.
/// Sends a JSON response back via `ws_tx`.
pub async fn handle_dynamic_tool(
    id: u64,
    tool_name: &str,
    args: &Value,
    role_id: &str,
    task_id: &str,
    state: &SharedState,
    app: &AppHandle,
    ws_tx: &WsSend,
) {
    let result_text = match tool_name {
        "reply" => handle_reply(args, role_id, task_id, state, app).await,
        "check_messages" => handle_check_messages(role_id, task_id, state).await,
        "get_status" => handle_get_status(task_id, state).await,
        other => format!("Unknown tool: {other}"),
    };

    let response = json!({
        "id": id,
        "result": {
            "contentItems": [{ "type": "inputText", "text": result_text }],
            "success": true
        }
    });
    if ws_tx.send(response.to_string()).await.is_err() {
        eprintln!("[Codex] failed to send tool response for id={id}");
    }
}

fn build_reply_message(
    args: &Value,
    from: &str,
    agent_id: &str,
    display_source: &str,
) -> Option<BridgeMessage> {
    let to = args["to"].as_str().unwrap_or("user");
    let text = args["text"].as_str().unwrap_or("");
    if text.trim().is_empty() {
        return None;
    }

    let status = args["status"]
        .as_str()
        .and_then(MessageStatus::parse)
        .unwrap_or(MessageStatus::Done);
    Some(BridgeMessage {
        id: format!("codex_{}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        display_source: Some(display_source.to_string()),
        to: to.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(status),
        task_id: None,
        session_id: None,
        sender_agent_id: Some(agent_id.to_string()),
        attachments: None,
    })
}

async fn handle_reply(
    args: &Value,
    from: &str,
    task_id: &str,
    state: &SharedState,
    app: &AppHandle,
) -> String {
    let to = args["to"].as_str().unwrap_or("user");
    // Resolve actual Codex identity from task_agents (AC1)
    let (agent_id, display_source) = {
        let s = state.read().await;
        resolve_codex_identity(&s, task_id)
    };
    let Some(mut msg) = build_reply_message(args, from, &agent_id, display_source) else {
        return format!("Ignored empty message to {to}");
    };
    {
        let s = state.read().await;
        s.stamp_message_context_for_task(task_id, from, &mut msg);
    }

    crate::daemon::routing::route_message(state, app, msg).await;
    format!("Message sent to {to}")
}

/// Resolve Codex (agent_id, display_source) from task_agents.
/// Falls back to ("codex", "codex") for legacy tasks.
fn resolve_codex_identity(
    s: &crate::daemon::state::DaemonState,
    task_id: &str,
) -> (String, &'static str) {
    let agents = s.task_graph.agents_for_task(task_id);
    agents
        .iter()
        .find(|a| a.provider == crate::daemon::task_graph::types::Provider::Codex)
        .map(|a| (a.agent_id.clone(), "codex"))
        .unwrap_or_else(|| ("codex".into(), "codex"))
}

async fn handle_check_messages(role_id: &str, task_id: &str, state: &SharedState) -> String {
    let msgs = state
        .write()
        .await
        .take_buffered_for_task(role_id, Some(task_id));
    if msgs.is_empty() {
        return "No new messages.".to_string();
    }

    msgs.iter()
        .map(|m| format!("[{}] {}: {}", m.timestamp, m.from, m.content))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn handle_get_status(task_id: &str, state: &SharedState) -> String {
    let s = state.read().await;
    let snapshot = s.task_scoped_online_agents(task_id);
    serde_json::to_string(&json!({ "online_agents": snapshot }))
        .unwrap_or_else(|_| r#"{"online_agents":[]}"#.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::state::DaemonState;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};

    #[test]
    fn reply_builder_preserves_status() {
        let args = serde_json::json!({
            "to": "user",
            "text": "final review result",
            "status": "error"
        });

        let msg = build_reply_message(&args, "lead", "codex-agent-1", "codex").expect("message");
        assert_eq!(msg.to, "user");
        assert_eq!(msg.status, Some(MessageStatus::Error));
    }

    #[test]
    fn reply_builder_defaults_status_to_done() {
        let args = serde_json::json!({
            "to": "coder",
            "text": "take task 2"
        });

        let msg = build_reply_message(&args, "lead", "codex-agent-1", "codex").expect("message");
        assert_eq!(msg.status, Some(MessageStatus::Done));
    }

    #[test]
    fn reply_builder_rejects_empty_text() {
        let args = serde_json::json!({
            "to": "user",
            "text": "   "
        });

        assert!(build_reply_message(&args, "lead", "codex-agent-1", "codex").is_none());
    }

    #[tokio::test]
    async fn get_status_returns_valid_json() {
        let state: SharedState = Arc::new(RwLock::new(DaemonState::new()));
        let status = handle_get_status("", &state).await;
        let v: serde_json::Value = serde_json::from_str(&status).expect("must be valid JSON");
        assert!(
            v["online_agents"].is_array(),
            "top-level online_agents must be array"
        );
    }

    #[tokio::test]
    async fn get_status_includes_wired_codex_session() {
        let state: SharedState = Arc::new(RwLock::new(DaemonState::new()));
        let (tx, _rx) = mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
        state.write().await.codex_inject_tx = Some(tx);

        let status = handle_get_status("", &state).await;
        let v: serde_json::Value = serde_json::from_str(&status).expect("must be valid JSON");
        let agents = v["online_agents"].as_array().expect("must be array");
        assert_eq!(agents.len(), 1);
        let agent = &agents[0];
        assert_eq!(agent["agentId"], "codex");
        assert!(agent["role"].is_string(), "role must be string");
        assert!(
            agent["modelSource"].is_string(),
            "modelSource must be string"
        );
    }

    #[tokio::test]
    async fn get_status_empty_when_no_agents_online() {
        let state: SharedState = Arc::new(RwLock::new(DaemonState::new()));
        let status = handle_get_status("", &state).await;
        let v: serde_json::Value = serde_json::from_str(&status).expect("must be valid JSON");
        let agents = v["online_agents"].as_array().expect("must be array");
        assert!(agents.is_empty(), "no agents should be online by default");
    }
}
