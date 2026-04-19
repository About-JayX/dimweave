use crate::daemon::gui::{self, ClaudeStreamPayload};
use crate::daemon::SharedState;
use serde_json::Value;
use std::time::Duration;
use tauri::AppHandle;

const CLAUDE_PREVIEW_BATCH_WINDOW_MS: u64 = 50;

/// Parse `stream_event` and emit `claude_stream` for real-time UI updates.
///
/// stream_event.event contains raw Anthropic API events:
/// - content_block_start {content_block: {type: "thinking"|"text"|"tool_use"}}
/// - content_block_delta {delta: {type: "thinking_delta"|"text_delta"|"input_json_delta"}}
/// - content_block_stop, message_start, message_delta, message_stop
pub(super) async fn handle_stream_event(
    event: &Value,
    task_id: &str,
    agent_id: &str,
    state: &SharedState,
    app: &AppHandle,
) {
    let inner = &event["event"];
    let event_type = inner["type"].as_str().unwrap_or("");
    let tid = Some(task_id);
    let aid = Some(agent_id);

    match event_type {
        "content_block_start" => {
            let block_type = inner["content_block"]["type"].as_str().unwrap_or("");
            match block_type {
                "thinking" => {
                    state.write().await.clear_claude_preview_batch();
                    gui::emit_claude_stream(app, tid, aid, ClaudeStreamPayload::ThinkingStarted);
                }
                "text" => {
                    flush_pending_preview_batch(task_id, agent_id, state, app).await;
                    gui::emit_claude_stream(app, tid, aid, ClaudeStreamPayload::TextStarted);
                }
                "tool_use" => {
                    flush_pending_preview_batch(task_id, agent_id, state, app).await;
                    let name = inner["content_block"]["name"]
                        .as_str()
                        .unwrap_or("tool")
                        .to_string();
                    gui::emit_claude_stream(app, tid, aid, ClaudeStreamPayload::ToolStarted { name });
                }
                _ => {}
            }
        }
        "content_block_delta" => {
            let delta_type = inner["delta"]["type"].as_str().unwrap_or("");
            match delta_type {
                "thinking_delta" => {
                    if let Some(text) = inner["delta"]["thinking"].as_str() {
                        gui::emit_claude_stream(
                            app,
                            tid,
                            aid,
                            ClaudeStreamPayload::ThinkingDelta {
                                text: text.to_string(),
                            },
                        );
                    }
                }
                "text_delta" => {
                    if let Some(text) = inner["delta"]["text"].as_str() {
                        // Batch text deltas for preview compat + direct emit for live UI
                        let should_schedule =
                            state.write().await.append_claude_preview_delta(text);
                        gui::emit_claude_stream(
                            app,
                            tid,
                            aid,
                            ClaudeStreamPayload::TextDelta {
                                text: text.to_string(),
                            },
                        );
                        if should_schedule {
                            let state = state.clone();
                            let app = app.clone();
                            let task_id = task_id.to_string();
                            let agent_id = agent_id.to_string();
                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_millis(
                                    CLAUDE_PREVIEW_BATCH_WINDOW_MS,
                                ))
                                .await;
                                flush_pending_preview_batch(&task_id, &agent_id, &state, &app).await;
                            });
                        }
                    }
                }
                // input_json_delta for tool_use — not rendered as text
                _ => {}
            }
        }
        _ => {}
    }
}

pub(super) async fn flush_pending_preview_batch(
    task_id: &str,
    agent_id: &str,
    state: &SharedState,
    app: &AppHandle,
) {
    let Some(text) = state.write().await.take_claude_preview_batch() else {
        return;
    };

    gui::emit_claude_stream(app, Some(task_id), Some(agent_id), ClaudeStreamPayload::Preview { text });
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
