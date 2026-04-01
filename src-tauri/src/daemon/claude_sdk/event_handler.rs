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
pub async fn handle_events(
    events: Vec<Value>,
    role: &str,
    state: SharedState,
    app: AppHandle,
) {
    for event in events {
        let Some(event_type) = event["type"].as_str() else {
            continue;
        };
        match event_type {
            "assistant" => handle_assistant(&event, role, &state, &app).await,
            "control_request" => handle_control_request(&event, &state, &app).await,
            "system" => handle_system(&event, &app),
            "result" => handle_result(&event, role, &state, &app).await,
            "user" | "keep_alive" => { /* echo / heartbeat — ignore */ }
            "rate_limit_event" => {
                let detail = event["message"].as_str().unwrap_or("rate limited");
                gui::emit_system_log(&app, "warn", &format!("[Claude SDK] {detail}"));
            }
            other => {
                eprintln!("[Claude SDK] unhandled event type: {other}");
            }
        }
    }
}

async fn handle_assistant(event: &Value, role: &str, state: &SharedState, app: &AppHandle) {
    let text = extract_assistant_text(event);
    if text.is_empty() {
        return;
    }
    let msg = BridgeMessage {
        id: format!("claude_sdk_{}", chrono::Utc::now().timestamp_millis()),
        from: role.to_string(),
        display_source: Some("claude".to_string()),
        to: "user".to_string(),
        content: text,
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::InProgress),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("claude".to_string()),
    };
    routing::route_message(state, app, msg).await;
}

async fn handle_control_request(event: &Value, state: &SharedState, app: &AppHandle) {
    let request_obj = &event["request"];
    let subtype = request_obj["subtype"].as_str().unwrap_or("");
    if subtype != "can_use_tool" {
        eprintln!("[Claude SDK] unknown control_request subtype: {subtype}");
        return;
    }
    let request_id = match event["request_id"].as_str() {
        Some(id) => id.to_string(),
        None => return,
    };
    let tool_name = request_obj["tool_name"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let description = request_obj["description"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let input_preview = request_obj["input"]
        .as_object()
        .map(|obj| serde_json::to_string_pretty(obj).unwrap_or_default())
        .or_else(|| request_obj["input"].as_str().map(ToOwned::to_owned));

    let perm = PermissionRequest {
        request_id: request_id.clone(),
        tool_name: tool_name.clone(),
        description: description.clone(),
        input_preview,
    };
    let created_at = chrono::Utc::now().timestamp_millis() as u64;
    state
        .write()
        .await
        .store_permission_request("claude", perm.clone(), created_at);
    gui::emit_permission_prompt(app, "claude", &perm, created_at);
    gui::emit_system_log(
        app,
        "info",
        &format!("[Claude SDK] permission request {request_id} for {tool_name}"),
    );
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
        .or_else(|| extract_assistant_text(event).into());
    if let Some(text) = text {
        if !text.is_empty() {
            let msg = BridgeMessage {
                id: format!("claude_sdk_result_{}", chrono::Utc::now().timestamp_millis()),
                from: role.to_string(),
                display_source: Some("claude".to_string()),
                to: "user".to_string(),
                content: text,
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                reply_to: None,
                priority: None,
                status: Some(MessageStatus::Done),
                task_id: None,
                session_id: None,
                sender_agent_id: Some("claude".to_string()),
            };
            routing::route_message(state, app, msg).await;
        }
    }
    gui::emit_system_log(app, "info", "[Claude SDK] turn completed");
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
