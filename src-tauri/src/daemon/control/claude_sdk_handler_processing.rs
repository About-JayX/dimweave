use crate::daemon::{gui, state::DaemonState, SharedState};
use tauri::AppHandle;

/// Resolve Claude (role, agent_id, display_source) from task_agents[] first,
/// then legacy provider bindings, then global `claude_role`.
fn resolve_claude_identity_for_task(
    s: &DaemonState,
    task_id: &str,
) -> (String, String, &'static str) {
    let agents = s.task_graph.agents_for_task(task_id);
    if let Some(claude_agent) = agents.iter().find(|a| {
        a.provider == crate::daemon::task_graph::types::Provider::Claude
    }) {
        return (claude_agent.role.clone(), claude_agent.agent_id.clone(), "claude");
    }
    // Legacy fallback: derive role from task provider slots
    let role = if let Some(task) = s.task_graph.get_task(task_id) {
        if task.lead_provider == crate::daemon::task_graph::types::Provider::Claude {
            "lead".into()
        } else if task.coder_provider == crate::daemon::task_graph::types::Provider::Claude {
            "coder".into()
        } else {
            s.claude_role.clone()
        }
    } else {
        s.claude_role.clone()
    };
    (role, "claude".into(), "claude")
}

pub(crate) async fn process_sdk_events(
    state: &SharedState,
    app: &AppHandle,
    events: Vec<serde_json::Value>,
    task_id: &str,
) {
    for event in events {
        process_sdk_event(state, app, event, task_id).await;
    }
}

async fn process_sdk_event(
    state: &SharedState,
    app: &AppHandle,
    event: serde_json::Value,
    task_id: &str,
) {
    let (role, agent_id, display_source) = {
        let s = state.read().await;
        resolve_claude_identity_for_task(&s, task_id)
    };
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
        &agent_id,
        display_source,
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
