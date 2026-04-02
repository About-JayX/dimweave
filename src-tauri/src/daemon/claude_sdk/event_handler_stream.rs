use crate::daemon::gui::{self, ClaudeStreamPayload};
use serde_json::Value;
use tauri::AppHandle;

/// Parse `stream_event` and emit `claude_stream` for real-time UI updates.
///
/// stream_event.event contains raw Anthropic API events:
/// - content_block_start {content_block: {type: "text"|"tool_use"|...}}
/// - content_block_delta {delta: {type: "text_delta", text: "..."}}
/// - message_start, message_delta, message_stop
pub(super) fn handle_stream_event(event: &Value, app: &AppHandle) {
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
        _ => {}
    }
}

pub(super) fn extract_assistant_text(event: &Value) -> String {
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
