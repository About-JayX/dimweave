use crate::daemon::{types::BridgeMessage, SharedState};
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
    ws_tx.send(response.to_string()).await.ok();
}

async fn handle_reply(args: &Value, from: &str, state: &SharedState, app: &AppHandle) -> String {
    let to = args["to"].as_str().unwrap_or("user");
    let text = args["text"].as_str().unwrap_or("");

    let msg = BridgeMessage {
        id: format!("codex_{}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        to: to.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
    };

    crate::daemon::routing::route_message(state, app, msg).await;
    format!("Message sent to {to}")
}

async fn handle_check_messages(role_id: &str, state: &SharedState) -> String {
    let mut s = state.write().await;
    let msgs: Vec<BridgeMessage> = s
        .buffered_messages
        .iter()
        .filter(|m| m.to == role_id)
        .cloned()
        .collect();

    // Remove delivered messages
    s.buffered_messages.retain(|m| m.to != role_id);

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
    let online: Vec<&str> = s.attached_agents.keys().map(|k| k.as_str()).collect();
    format!(
        "Claude role: {}, Codex role: {}, Online agents: [{}]",
        s.claude_role,
        s.codex_role,
        online.join(", ")
    )
}
