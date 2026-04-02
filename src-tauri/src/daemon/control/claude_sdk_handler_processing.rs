use crate::daemon::{gui, SharedState};
use tauri::AppHandle;

pub(crate) async fn process_sdk_events(
    state: &SharedState,
    app: &AppHandle,
    events: Vec<serde_json::Value>,
) {
    for event in events {
        process_sdk_event(state, app, event).await;
    }
}

async fn process_sdk_event(
    state: &SharedState,
    app: &AppHandle,
    event: serde_json::Value,
) {
    let role = state.read().await.claude_role.clone();
    gui::emit_system_log(
        app,
        "info",
        &format!(
            "[Claude Trace] chain=event_dispatch role={} {}",
            role,
            summarize_event_shape(&event)
        ),
    );
    crate::daemon::claude_sdk::event_handler::handle_events(
        vec![event],
        &role,
        state.clone(),
        app.clone(),
    )
    .await;
}

pub(crate) fn summarize_events_batch(
    body: &crate::daemon::claude_sdk::protocol::PostEventsBody,
) -> String {
    let events = body
        .events
        .iter()
        .map(summarize_event_shape)
        .collect::<Vec<_>>()
        .join("; ");
    format!("count={} events=[{}]", body.events.len(), events)
}

pub(crate) fn summarize_event_shape(event: &serde_json::Value) -> String {
    let event_type = event["type"].as_str().unwrap_or("unknown");
    let session = event["session_id"]
        .as_str()
        .or_else(|| event["sessionId"].as_str())
        .unwrap_or("-");
    match event_type {
        "assistant" => summarize_assistant_shape(session, &event["message"]["content"]),
        "result" => summarize_result_shape(session, event),
        "system" => format!("system session={} shape={{type,session_id}}", session),
        "control_request" => {
            let tool_name = event["request"]["tool_name"].as_str().unwrap_or("-");
            format!(
                "control_request session={} shape={{type,session_id,request_id,request{{subtype,tool_name,description,input}}}} tool_name={}",
                session, tool_name
            )
        }
        other => format!("{other} session={session} shape={{type,session_id,...}}"),
    }
}

fn summarize_assistant_shape(session: &str, content: &serde_json::Value) -> String {
    let content_items = content.as_array().map_or(0, Vec::len);
    let text_len = extract_event_text_len(content);
    format!(
        "assistant session={} shape={{type,session_id,message{{content[]}}}} content_items={} text_len={}",
        session, content_items, text_len
    )
}

fn summarize_result_shape(session: &str, event: &serde_json::Value) -> String {
    let result_len = event["result"].as_str().map_or(0, str::len);
    format!(
        "result session={} shape={{type,session_id,result}} result_len={}",
        session, result_len
    )
}

fn extract_event_text_len(content: &serde_json::Value) -> usize {
    match content {
        serde_json::Value::String(text) => text.len(),
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                if item["type"].as_str() == Some("text") {
                    item["text"].as_str()
                } else {
                    None
                }
            })
            .map(str::len)
            .sum(),
        _ => 0,
    }
}
