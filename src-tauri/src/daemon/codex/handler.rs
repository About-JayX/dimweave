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

    let msg = BridgeMessage {
        id: format!("codex_{}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        display_source: Some("codex".into()),
        to: to.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::Done),
    };

    crate::daemon::routing::route_message(state, app, msg).await;
    format!("Message sent to {to}")
}

async fn handle_check_messages(role_id: &str, state: &SharedState) -> String {
    let msgs = state.write().await.take_buffered_for(role_id);
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
    let mut online: Vec<String> = Vec::new();
    if s.attached_agents.contains_key("claude") {
        online.push("claude".to_string());
    }
    if s.codex_inject_tx.is_some() {
        online.push("codex".to_string());
    }
    for agent in s
        .attached_agents
        .keys()
        .filter(|agent| agent.as_str() != "claude" && agent.as_str() != "codex")
    {
        online.push(agent.clone());
    }
    format!(
        "Claude role: {}, Codex role: {}, Online agents: [{}]",
        s.claude_role,
        s.codex_role,
        online.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::state::DaemonState;
    use std::sync::Arc;
    use tokio::sync::{mpsc, RwLock};

    #[tokio::test]
    async fn get_status_reports_wired_codex_session() {
        let state: SharedState = Arc::new(RwLock::new(DaemonState::new()));
        let (tx, _rx) = mpsc::channel::<(String, bool)>(1);
        state.write().await.codex_inject_tx = Some(tx);

        let status = handle_get_status(&state).await;
        assert!(status.contains("Online agents: [codex]"));
    }
}
