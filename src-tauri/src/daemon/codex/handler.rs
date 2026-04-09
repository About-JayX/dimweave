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
    state: &SharedState,
    app: &AppHandle,
    ws_tx: &WsSend,
) {
    let result_text = match tool_name {
        "reply" => handle_reply(args, role_id, state, app).await,
        "check_messages" => handle_check_messages(role_id, state).await,
        "get_status" => handle_get_status(state).await,
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

async fn handle_reply(args: &Value, from: &str, state: &SharedState, app: &AppHandle) -> String {
    let to = args["to"].as_str().unwrap_or("user");
    let text = args["text"].as_str().unwrap_or("");
    if text.trim().is_empty() {
        return format!("Ignored empty message to {to}");
    }

    let mut msg = BridgeMessage {
        id: format!("codex_{}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        display_source: Some("codex".into()),
        to: to.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::Done),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("codex".into()),
        attachments: None, report_telegram: None,
    };
    state.read().await.stamp_message_context(from, &mut msg);

    crate::daemon::routing::route_message(state, app, msg).await;
    format!("Message sent to {to}")
}

async fn handle_check_messages(role_id: &str, state: &SharedState) -> String {
    let task_id = state.read().await.active_task_id.clone();
    let msgs = state
        .write()
        .await
        .take_buffered_for_task(role_id, task_id.as_deref());
    if msgs.is_empty() {
        return "No new messages.".to_string();
    }

    msgs.iter()
        .map(|m| format!("[{}] {}: {}", m.timestamp, m.from, m.content))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn handle_get_status(state: &SharedState) -> String {
    let s = state.read().await;
    let snapshot = s.online_agents_snapshot();
    serde_json::to_string(&json!({ "online_agents": snapshot }))
        .unwrap_or_else(|_| r#"{"online_agents":[]}"#.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::state::DaemonState;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};

    #[tokio::test]
    async fn get_status_returns_valid_json() {
        let state: SharedState = Arc::new(RwLock::new(DaemonState::new()));
        let status = handle_get_status(&state).await;
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

        let status = handle_get_status(&state).await;
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
        let status = handle_get_status(&state).await;
        let v: serde_json::Value = serde_json::from_str(&status).expect("must be valid JSON");
        let agents = v["online_agents"].as_array().expect("must be array");
        assert!(agents.is_empty(), "no agents should be online by default");
    }
}
