//! Process NDJSON events POSTed by Claude to `/claude/events`.

use crate::daemon::{
    gui::{self, ClaudeStreamPayload},
    routing,
    types::{BridgeMessage, MessageStatus, PermissionRequest},
    SharedState,
};
use serde_json::Value;
use tauri::AppHandle;

/// Dispatch a batch of events from Claude's HTTP POST.
pub async fn handle_events(events: Vec<Value>, role: &str, state: SharedState, app: AppHandle) {
    for event in events {
        let Some(event_type) = event["type"].as_str() else {
            continue;
        };
        match event_type {
            "assistant" => handle_assistant(&event, role, &state, &app).await,
            "control_request" => handle_control_request(&event, &state, &app).await,
            "system" => handle_system(&event, &app),
            "result" => handle_result(&event, role, &state, &app).await,
            "user" | "keep_alive" | "control_cancel_request" => { /* echo / heartbeat / cancel — ignore */ }
            "stream_event" => handle_stream_event(&event, &app),
            "rate_limit_event" => {
                let status = event["rate_limit_info"]["status"].as_str().unwrap_or("?");
                gui::emit_system_log(&app, "info", &format!("[Claude SDK] rate_limit: {status}"));
            }
            "prompt_suggestion" => {
                let suggestion = event["suggestion"].as_str().unwrap_or("");
                if !suggestion.is_empty() {
                    gui::emit_system_log(&app, "info", &format!("[Claude SDK] suggestion: {suggestion}"));
                }
            }
            "auth_status" => {
                let is_auth = event["isAuthenticating"].as_bool().unwrap_or(false);
                gui::emit_system_log(&app, "info", &format!("[Claude SDK] auth_status: authenticating={is_auth}"));
            }
            other => {
                gui::emit_system_log(&app, "info", &format!("[Claude SDK] unhandled event: {other}"));
            }
        }
    }
}

async fn handle_assistant(event: &Value, role: &str, state: &SharedState, app: &AppHandle) {
    let text = extract_assistant_text(event);
    if text.is_empty() || !begin_sdk_direct_text_turn_if_allowed(state).await {
        return;
    }
    // SDK fallback intentionally keeps in-progress text out of chat bubbles.
    // Assistant chunks only lock in direct-routing ownership for this turn so a
    // late bridge attach cannot steal the final visible result mid-turn.
    if let Some(msg) = build_direct_sdk_gui_message(role, &text, MessageStatus::InProgress) {
        routing::route_message(state, app, msg).await;
    }
}

async fn handle_control_request(event: &Value, state: &SharedState, app: &AppHandle) {
    let request_obj = &event["request"];
    let subtype = request_obj["subtype"].as_str().unwrap_or("");
    let request_id = match event["request_id"].as_str() {
        Some(id) => id.to_string(),
        None => return,
    };

    let ndjson = match subtype {
        "can_use_tool" => {
            let tool_name = request_obj["tool_name"].as_str().unwrap_or("unknown");
            gui::emit_system_log(
                app,
                "info",
                &format!("[Claude SDK] auto-approving {tool_name} ({request_id})"),
            );
            crate::daemon::claude_sdk::protocol::format_control_response(&request_id, true)
        }
        "initialize" => {
            gui::emit_system_log(app, "info", "[Claude SDK] responding to initialize");
            crate::daemon::claude_sdk::protocol::format_initialize_response(&request_id)
        }
        // set_permission_mode, set_model, interrupt, etc. — ack with empty success
        _ => {
            gui::emit_system_log(
                app,
                "info",
                &format!("[Claude SDK] acking control_request subtype={subtype} ({request_id})"),
            );
            let msg = serde_json::json!({
                "type": "control_response",
                "response": {
                    "subtype": "success",
                    "request_id": request_id,
                    "response": {}
                }
            });
            format!("{msg}\n")
        }
    };

    // Send response via WS. Retry up to 3s in case WS isn't attached yet.
    let mut sent = false;
    for attempt in 0..30 {
        let sdk_tx = state.read().await.claude_sdk_ws_tx.clone();
        if let Some(tx) = sdk_tx {
            if tx.send(ndjson.clone()).await.is_ok() {
                sent = true;
                break;
            }
        }
        if attempt < 29 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    if !sent {
        gui::emit_system_log(
            app,
            "error",
            &format!("[Claude SDK] FAILED to respond to {subtype} ({request_id}) — WS not ready"),
        );
    }
}

fn handle_system(event: &Value, app: &AppHandle) {
    let session_id = event["session_id"]
        .as_str()
        .or_else(|| event["sessionId"].as_str())
        .unwrap_or("unknown");
    gui::emit_system_log(
        app,
        "info",
        &format!("[Claude SDK] session init: {session_id}"),
    );
}

async fn handle_result(event: &Value, role: &str, state: &SharedState, app: &AppHandle) {
    gui::emit_claude_stream(app, ClaudeStreamPayload::Done);
    // Extract final text if present in result
    let text = event["result"]
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| Some(extract_assistant_text(event)));
    if let Some(text) = text.filter(|text| !text.is_empty()) {
        if !claim_sdk_terminal_delivery(state).await {
            gui::emit_system_log(
                app,
                "info",
                "[Claude SDK] suppressed duplicate terminal text; bridge owns visible result",
            );
            gui::emit_system_log(
                app,
                "info",
                &format!(
                    "[Claude Trace] chain=sdk_result delivery=bridge_owned text_len={}",
                    text.len()
                ),
            );
            finish_sdk_direct_text_turn(state).await;
            gui::emit_system_log(app, "info", "[Claude SDK] turn completed");
            return;
        }
        gui::emit_system_log(
            app,
            "info",
            &format!(
                "[Claude Trace] chain=sdk_result delivery=direct_sdk text_len={} role={}",
                text.len(),
                role
            ),
        );
        if let Some(msg) = build_direct_sdk_gui_message(role, &text, MessageStatus::Done) {
            routing::route_message(state, app, msg).await;
        }
    }
    gui::emit_system_log(app, "info", "[Claude SDK] turn completed");
}

async fn begin_sdk_direct_text_turn_if_allowed(state: &SharedState) -> bool {
    state.write().await.begin_claude_sdk_direct_text_turn()
}

async fn claim_sdk_terminal_delivery(state: &SharedState) -> bool {
    state.write().await.claim_claude_sdk_terminal_delivery()
}

async fn finish_sdk_direct_text_turn(state: &SharedState) {
    state.write().await.finish_claude_sdk_direct_text_turn();
}

fn build_direct_sdk_gui_message(
    role: &str,
    text: &str,
    status: MessageStatus,
) -> Option<BridgeMessage> {
    // Direct SDK fallback only renders terminal text. UI already exposes a
    // single Claude thinking indicator, so surfacing partial assistant chunks
    // here would reintroduce the duplicate/preview noise we removed.
    if !status.is_terminal() || text.is_empty() {
        return None;
    }
    let prefix = match status {
        MessageStatus::Done => "claude_sdk_result",
        MessageStatus::Error => "claude_sdk_error",
        MessageStatus::InProgress => "claude_sdk",
    };
    Some(BridgeMessage {
        id: format!("{prefix}_{}", chrono::Utc::now().timestamp_millis()),
        from: role.to_string(),
        display_source: Some("claude".to_string()),
        to: "user".to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(status),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("claude".to_string()),
    })
}

/// Parse `stream_event` and emit `claude_stream` for real-time UI updates.
///
/// stream_event.event contains raw Anthropic API events:
/// - content_block_start {content_block: {type: "text"|"tool_use"|...}}
/// - content_block_delta {delta: {type: "text_delta", text: "..."}}
/// - message_start, message_delta, message_stop
fn handle_stream_event(event: &Value, app: &AppHandle) {
    let inner = &event["event"];
    let event_type = inner["type"].as_str().unwrap_or("");

    match event_type {
        "content_block_start" => {
            let block_type = inner["content_block"]["type"].as_str().unwrap_or("");
            if block_type == "text" {
                gui::emit_claude_stream(app, ClaudeStreamPayload::ThinkingStarted);
            }
        }
        "content_block_delta" => {
            let delta_type = inner["delta"]["type"].as_str().unwrap_or("");
            if delta_type == "text_delta" {
                if let Some(text) = inner["delta"]["text"].as_str() {
                    if !text.is_empty() {
                        gui::emit_claude_stream(
                            app,
                            ClaudeStreamPayload::Preview {
                                text: text.to_string(),
                            },
                        );
                    }
                }
            }
        }
        // message_start, message_delta, message_stop — no UI action needed
        _ => {}
    }
}

fn extract_assistant_text(event: &Value) -> String {
    let content = &event["message"]["content"];
    match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                if item["type"].as_str() == Some("text") {
                    item["text"].as_str().map(ToOwned::to_owned)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::types::MessageStatus;
    use serde_json::json;

    #[test]
    fn in_progress_sdk_text_does_not_create_visible_gui_message() {
        let msg = build_direct_sdk_gui_message("lead", "partial reply", MessageStatus::InProgress);
        assert!(msg.is_none());
    }

    #[test]
    fn terminal_sdk_text_creates_visible_gui_message() {
        let msg = build_direct_sdk_gui_message("lead", "final reply", MessageStatus::Done)
            .expect("done messages should be visible");

        assert_eq!(msg.from, "lead");
        assert_eq!(msg.display_source.as_deref(), Some("claude"));
        assert_eq!(msg.to, "user");
        assert_eq!(msg.content, "final reply");
        assert_eq!(msg.status, Some(MessageStatus::Done));
    }

    // ── extract_assistant_text ──────────────────────────

    #[test]
    fn extract_text_from_content_array() {
        let event = json!({
            "message": {
                "content": [
                    {"type": "text", "text": "Hello "},
                    {"type": "tool_use", "name": "Bash"},
                    {"type": "text", "text": "world"}
                ]
            }
        });
        assert_eq!(extract_assistant_text(&event), "Hello world");
    }

    #[test]
    fn extract_text_from_string_content() {
        let event = json!({"message": {"content": "plain text"}});
        assert_eq!(extract_assistant_text(&event), "plain text");
    }

    #[test]
    fn extract_text_returns_empty_for_missing_content() {
        let event = json!({"message": {}});
        assert_eq!(extract_assistant_text(&event), "");
    }

    #[test]
    fn extract_text_returns_empty_for_only_tool_use() {
        let event = json!({
            "message": {
                "content": [{"type": "tool_use", "name": "Edit"}]
            }
        });
        assert_eq!(extract_assistant_text(&event), "");
    }

    // ── stream event parsing (unit-testable parts) ─────

    #[test]
    fn stream_event_text_delta_extracts_text() {
        let event = json!({
            "type": "stream_event",
            "event": {
                "type": "content_block_delta",
                "delta": {"type": "text_delta", "text": "Hello"}
            }
        });
        let inner = &event["event"];
        let delta_type = inner["delta"]["type"].as_str().unwrap();
        let text = inner["delta"]["text"].as_str().unwrap();
        assert_eq!(delta_type, "text_delta");
        assert_eq!(text, "Hello");
    }

    #[test]
    fn stream_event_non_text_delta_has_no_text() {
        let event = json!({
            "type": "stream_event",
            "event": {
                "type": "content_block_delta",
                "delta": {"type": "input_json_delta", "partial_json": "{\"cmd\""}
            }
        });
        let text = event["event"]["delta"]["text"].as_str();
        assert!(text.is_none());
    }

    #[test]
    fn stream_event_content_block_start_text_type() {
        let event = json!({
            "type": "stream_event",
            "event": {
                "type": "content_block_start",
                "content_block": {"type": "text"}
            }
        });
        let block_type = event["event"]["content_block"]["type"].as_str().unwrap();
        assert_eq!(block_type, "text");
    }

    #[test]
    fn stream_event_content_block_start_tool_use_type() {
        let event = json!({
            "type": "stream_event",
            "event": {
                "type": "content_block_start",
                "content_block": {"type": "tool_use", "name": "Bash"}
            }
        });
        let block_type = event["event"]["content_block"]["type"].as_str().unwrap();
        assert_eq!(block_type, "tool_use");
    }
}
