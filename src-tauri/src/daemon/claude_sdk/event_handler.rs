//! Process NDJSON events POSTed by Claude to `/claude/events`.

use crate::daemon::{
    gui::{self, ClaudeStreamPayload},
    routing,
    types::MessageStatus,
    SharedState,
};
use delivery::{
    begin_sdk_direct_text_turn_if_allowed, build_direct_sdk_gui_message,
    claim_sdk_terminal_delivery, finish_sdk_direct_text_turn,
};
use serde_json::Value;
use stream::{extract_assistant_text, flush_pending_preview_batch, handle_stream_event};
use tauri::AppHandle;

#[path = "event_handler_delivery.rs"]
mod delivery;

#[path = "event_handler_stream.rs"]
mod stream;

#[cfg(test)]
#[path = "event_handler_tests.rs"]
mod tests;

/// Dispatch a batch of events from Claude's HTTP POST.
pub async fn handle_events(
    events: Vec<Value>,
    task_id: &str,
    role: &str,
    agent_id: &str,
    display_source: &str,
    state: SharedState,
    app: AppHandle,
) {
    for event in events {
        let Some(event_type) = event["type"].as_str() else {
            continue;
        };
        match event_type {
            "assistant" => handle_assistant(&event, role, agent_id, display_source, &state, &app).await,
            "control_request" => handle_control_request(&event, &state, &app).await,
            "system" => handle_system(&event, &app),
            "result" => handle_result(&event, task_id, role, agent_id, display_source, &state, &app).await,
            "user" | "keep_alive" | "control_cancel_request" => { /* echo / heartbeat / cancel — ignore */
            }
            "stream_event" => handle_stream_event(&event, task_id, agent_id, &state, &app).await,
            "rate_limit_event" => {
                let status = event["rate_limit_info"]["status"].as_str().unwrap_or("?");
                gui::emit_system_log(&app, "info", &format!("[Claude SDK] rate_limit: {status}"));
            }
            "prompt_suggestion" => {
                let suggestion = event["suggestion"].as_str().unwrap_or("");
                if !suggestion.is_empty() {
                    gui::emit_system_log(
                        &app,
                        "info",
                        &format!("[Claude SDK] suggestion: {suggestion}"),
                    );
                }
            }
            "auth_status" => {
                let is_auth = event["isAuthenticating"].as_bool().unwrap_or(false);
                gui::emit_system_log(
                    &app,
                    "info",
                    &format!("[Claude SDK] auth_status: authenticating={is_auth}"),
                );
            }
            other => {
                gui::emit_system_log(
                    &app,
                    "info",
                    &format!("[Claude SDK] unhandled event: {other}"),
                );
            }
        }
    }
}

async fn handle_assistant(
    event: &Value,
    role: &str,
    agent_id: &str,
    display_source: &str,
    state: &SharedState,
    app: &AppHandle,
) {
    let text = extract_assistant_text(event);
    if text.is_empty() || !begin_sdk_direct_text_turn_if_allowed(state).await {
        return;
    }
    if let Some(msg) = build_direct_sdk_gui_message(role, &text, MessageStatus::InProgress, agent_id, display_source) {
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
            // SDK mode intentionally runs with Claude's local permission bypass
            // enabled. If Claude still emits a can_use_tool request, we answer
            // allow here so the transport stays self-consistent instead of
            // stalling on a GUI permission flow that SDK mode no longer uses.
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
            crate::daemon::claude_sdk::protocol::format_generic_ack(&request_id)
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

async fn handle_result(
    event: &Value,
    task_id: &str,
    role: &str,
    agent_id: &str,
    display_source: &str,
    state: &SharedState,
    app: &AppHandle,
) {
    flush_pending_preview_batch(task_id, agent_id, state, app).await;
    let tid = Some(task_id);
    let aid = Some(agent_id);
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
            gui::emit_claude_stream(app, tid, aid, ClaudeStreamPayload::Done);
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
        if let Some(msg) = build_direct_sdk_gui_message(role, &text, MessageStatus::Done, agent_id, display_source) {
            routing::route_message(state, app, msg).await;
        }
    }
    // Done is emitted after route_message so the durable bubble arrives
    // before the frontend draft clears.
    gui::emit_claude_stream(app, tid, aid, ClaudeStreamPayload::Done);
    gui::emit_system_log(app, "info", "[Claude SDK] turn completed");
}
