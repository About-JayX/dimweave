use crate::daemon::codex::handler;
use crate::daemon::codex::structured_output::{
    parse_structured_output, should_emit_final_message, ParsedOutput, StreamPreviewState,
};
use crate::daemon::codex::ws_client::WsTx;
use crate::daemon::gui::{self, CodexStreamPayload};
use crate::daemon::gui_task::TaskUiEvent;
use crate::daemon::task_graph::types::{Provider, SessionStatus};
use crate::daemon::types::{BridgeMessage, MessageStatus};
use crate::daemon::{routing, SharedState};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;

pub(super) async fn handle_codex_event(
    v: &Value,
    role_id: &str,
    task_id: &str,
    agent_id: &str,
    schema_route_enabled: bool,
    state: &SharedState,
    app: &AppHandle,
    ws_tx: &WsTx,
    stream_preview: &mut StreamPreviewState,
) {
    let Some(method) = v["method"].as_str() else {
        return;
    };
    match method {
        "item/tool/call" => handle_tool_call(v, role_id, task_id, agent_id, state, app, ws_tx).await,
        "turn/started" => {
            stream_preview.reset();
            gui::emit_codex_stream(app, CodexStreamPayload::Thinking);
        }
        "thread/status/changed" => sync_thread_status_change(v, state, app).await,
        "thread/archived" => sync_thread_archive(v, state, app).await,
        "thread/unarchived" => sync_thread_unarchive(v, state, app).await,
        "item/started" => emit_activity_from_item(v, app),
        "item/reasoning/summaryTextDelta" => {
            if let Some(delta) = v["params"]["delta"].as_str().filter(|s| !s.is_empty()) {
                stream_preview.append_reasoning(delta);
                gui::emit_codex_stream(
                    app,
                    CodexStreamPayload::Reasoning {
                        text: stream_preview.reasoning_text().to_string(),
                    },
                );
            }
        }
        "item/reasoning/summaryPartAdded" => {
            stream_preview.append_reasoning_boundary();
            if !stream_preview.reasoning_text().is_empty() {
                gui::emit_codex_stream(
                    app,
                    CodexStreamPayload::Reasoning {
                        text: stream_preview.reasoning_text().to_string(),
                    },
                );
            }
        }
        "item/commandExecution/outputDelta" => {
            if let Some(delta) = v["params"]["delta"].as_str().filter(|s| !s.is_empty()) {
                gui::emit_codex_stream(
                    app,
                    CodexStreamPayload::CommandOutput {
                        text: delta.to_string(),
                    },
                );
            }
        }
        "item/agentMessage/delta" => {
            if let Some(text) = v["params"]["delta"]
                .as_str()
                .filter(|text| !text.is_empty())
            {
                if let Some(preview) = stream_preview.ingest_delta(text) {
                    gui::emit_codex_stream(app, CodexStreamPayload::Delta { text: preview });
                }
            }
        }
        "item/completed" => {
            handle_completed_agent_message(
                v,
                role_id,
                task_id,
                agent_id,
                schema_route_enabled,
                state,
                app,
                stream_preview,
            )
            .await;
        }
        "turn/completed" => {
            stream_preview.reset();
            let status = v["params"]["turn"]["status"].as_str().unwrap_or("unknown");
            gui::emit_codex_stream(
                app,
                CodexStreamPayload::TurnDone {
                    status: status.into(),
                },
            );
        }
        _ => {}
    }
}

async fn handle_tool_call(
    v: &Value,
    role_id: &str,
    task_id: &str,
    agent_id: &str,
    state: &SharedState,
    app: &AppHandle,
    ws_tx: &WsTx,
) {
    let name = v["params"]["tool"]
        .as_str()
        .or_else(|| v["params"]["name"].as_str());
    if let (Some(id), Some(name)) = (v["id"].as_u64(), name) {
        let args = v["params"]["arguments"].clone();
        handler::handle_dynamic_tool(id, name, &args, role_id, task_id, agent_id, state, app, ws_tx).await;
    }
}

fn build_completed_output_message(
    role_id: &str,
    parsed: &ParsedOutput,
    schema_route_enabled: bool,
    agent_id: &str,
    display_source: &str,
) -> Option<BridgeMessage> {
    if !should_emit_final_message(&parsed.message) {
        return None;
    }

    let target = if schema_route_enabled {
        parsed
            .send_to
            .as_deref()
            .filter(|t| matches!(*t, "user" | "lead" | "coder"))
            .unwrap_or("user")
    } else {
        "user"
    };

    Some(build_msg_with_status(role_id, target, &parsed.message, parsed.status, agent_id, display_source))
}

async fn handle_completed_agent_message(
    v: &Value,
    role_id: &str,
    task_id: &str,
    agent_id: &str,
    schema_route_enabled: bool,
    state: &SharedState,
    app: &AppHandle,
    stream_preview: &mut StreamPreviewState,
) {
    if v["params"]["item"]["type"].as_str() != Some("agentMessage") {
        return;
    }
    let raw = v["params"]["item"]["text"].as_str().unwrap_or("");
    if raw.is_empty() {
        return;
    }
    stream_preview.sync_final_raw(raw);
    let parsed = match parse_structured_output(raw) {
        Ok(parsed) => parsed,
        Err(err) => {
            let hint = err.to_string();
            gui::emit_system_log(app, "error", &format!("[Codex] {hint}"));
            let error_msg = build_msg_with_status(
                role_id, "user", &hint, MessageStatus::Error, agent_id, "codex",
            );
            gui::emit_agent_message(app, &error_msg);
            return;
        }
    };
    let display_source = "codex";
    let Some(mut msg) = build_completed_output_message(
        role_id, &parsed, schema_route_enabled, agent_id, display_source,
    ) else {
        return;
    };
    gui::emit_codex_stream(
        app,
        CodexStreamPayload::Message {
            text: parsed.message.clone(),
        },
    );
    {
        let s = state.read().await;
        s.stamp_message_context_for_task(task_id, role_id, &mut msg);
    }
    eprintln!("[Codex] route {} → {}", role_id, msg.to);
    routing::route_message(state, app, msg).await;
}

fn emit_activity_from_item(v: &Value, app: &AppHandle) {
    let item = &v["params"]["item"];
    if let Some(label) = activity_label_from_item(item) {
        gui::emit_codex_stream(app, CodexStreamPayload::Activity { label });
    }
}

fn map_thread_runtime_status(raw: &str) -> SessionStatus {
    match raw {
        "active" => SessionStatus::Active,
        "systemError" => SessionStatus::Error,
        _ => SessionStatus::Paused,
    }
}

async fn sync_thread_status_change(v: &Value, state: &SharedState, app: &AppHandle) {
    let thread_id = v["params"]["threadId"].as_str().unwrap_or("");
    let status_type = v["params"]["status"]["type"]
        .as_str()
        .unwrap_or("notLoaded");
    sync_thread_session_status(
        thread_id,
        map_thread_runtime_status(status_type),
        state,
        app,
    )
    .await;
}

async fn sync_thread_archive(v: &Value, state: &SharedState, app: &AppHandle) {
    let thread_id = v["params"]["threadId"].as_str().unwrap_or("");
    sync_thread_session_status(thread_id, SessionStatus::Completed, state, app).await;
}

async fn sync_thread_unarchive(v: &Value, state: &SharedState, app: &AppHandle) {
    let thread_id = v["params"]["threadId"].as_str().unwrap_or("");
    sync_thread_session_status(thread_id, SessionStatus::Paused, state, app).await;
}

async fn sync_thread_session_status(
    thread_id: &str,
    new_status: SessionStatus,
    state: &SharedState,
    app: &AppHandle,
) {
    if thread_id.is_empty() {
        return;
    }
    let payload = {
        let mut s = state.write().await;
        let Some(session) = s
            .task_graph
            .find_session_by_external_id(Provider::Codex, thread_id)
            .cloned()
        else {
            return;
        };
        if !s
            .task_graph
            .update_session_status(&session.session_id, new_status)
        {
            return;
        }
        let sessions = s
            .task_graph
            .sessions_for_task(&session.task_id)
            .into_iter()
            .cloned()
            .collect();
        s.auto_save_task_graph();
        (session.task_id, sessions)
    };
    TaskUiEvent::SessionTreeChanged {
        task_id: payload.0,
        sessions: payload.1,
    }
    .emit(app);
}

fn truncate_label(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        s.chars().take(max - 1).collect::<String>() + "…"
    }
}

fn activity_label_from_item(item: &Value) -> Option<String> {
    match item["type"].as_str() {
        Some("commandExecution") => {
            let cmd = item["command"].as_str().unwrap_or("…");
            Some(format!("Running: {}", truncate_label(cmd, 80)))
        }
        Some("fileChange") => {
            let change = item["changes"]
                .as_array()
                .and_then(|changes| changes.first());
            let path = change
                .and_then(|entry| entry["path"].as_str())
                .unwrap_or("…");
            let kind = change
                .and_then(|entry| entry["kind"].as_str())
                .unwrap_or("edit");
            Some(format!("File {kind}: {}", truncate_label(path, 80)))
        }
        Some("mcpToolCall") => {
            let tool = item["tool"].as_str().unwrap_or("…");
            Some(format!("MCP tool: {tool}"))
        }
        Some("reasoning") => Some("Reasoning…".into()),
        Some("webSearch") => match item["action"]["type"].as_str() {
            Some("openPage") => {
                let url = item["action"]["url"].as_str().unwrap_or("…");
                Some(format!("Opening: {}", truncate_label(url, 60)))
            }
            Some("findInPage") => {
                let pattern = item["action"]["pattern"].as_str().unwrap_or("…");
                Some(format!("Finding: {}", truncate_label(pattern, 60)))
            }
            _ => {
                let query = item["query"]
                    .as_str()
                    .or_else(|| item["action"]["query"].as_str())
                    .or_else(|| {
                        item["action"]["queries"]
                            .as_array()
                            .and_then(|queries| queries.first())
                            .and_then(|query| query.as_str())
                    })
                    .unwrap_or("…");
                Some(format!("Searching: {}", truncate_label(query, 60)))
            }
        },
        _ => None,
    }
}

static MSG_SEQ: AtomicU64 = AtomicU64::new(0);

fn build_msg_with_status(
    from: &str,
    to: &str,
    content: &str,
    status: MessageStatus,
    agent_id: &str,
    display_source: &str,
) -> BridgeMessage {
    let seq = MSG_SEQ.fetch_add(1, Ordering::Relaxed);
    BridgeMessage {
        id: format!("codex_{}_{seq}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        display_source: Some(display_source.to_string()),
        to: to.to_string(),
        content: content.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(status),
        task_id: None,
        session_id: None,
        sender_agent_id: Some(agent_id.to_string()),
        attachments: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn completed_output_builder_routes_to_schema_target() {
        let parsed = ParsedOutput {
            message: "final review result".into(),
            send_to: Some("user".into()),
            status: MessageStatus::Done,
        };

        let msg = build_completed_output_message("lead", &parsed, true, "codex-agent-1", "codex").expect("message");
        assert_eq!(msg.to, "user");
        assert_eq!(msg.status, Some(MessageStatus::Done));
    }

    #[test]
    fn completed_output_builder_restricts_schema_routes_to_known_roles() {
        let parsed = ParsedOutput {
            message: "final review result".into(),
            send_to: Some("reviewer".into()),
            status: MessageStatus::Done,
        };

        let msg = build_completed_output_message("lead", &parsed, true, "codex-agent-1", "codex").expect("message");
        assert_eq!(msg.to, "user");
    }

    #[test]
    fn completed_output_builder_defaults_to_user_when_schema_disabled() {
        let parsed = ParsedOutput {
            message: "status update".into(),
            send_to: Some("lead".into()),
            status: MessageStatus::InProgress,
        };

        let msg = build_completed_output_message("coder", &parsed, false, "codex-agent-1", "codex").expect("message");
        assert_eq!(msg.to, "user");
    }

    #[test]
    fn completed_output_builder_rejects_empty_message() {
        let parsed = ParsedOutput {
            message: "   ".into(),
            send_to: Some("user".into()),
            status: MessageStatus::Done,
        };

        assert!(build_completed_output_message("lead", &parsed, true, "codex-agent-1", "codex").is_none());
    }

    #[test]
    fn activity_label_formats_command_execution() {
        let item = json!({
            "type": "commandExecution",
            "command": "ls -la src-tauri/src/daemon"
        });

        assert_eq!(
            activity_label_from_item(&item).as_deref(),
            Some("Running: ls -la src-tauri/src/daemon")
        );
    }

    #[test]
    fn activity_label_formats_reasoning_state() {
        let item = json!({ "type": "reasoning" });

        assert_eq!(
            activity_label_from_item(&item).as_deref(),
            Some("Reasoning…")
        );
    }

    #[test]
    fn activity_label_truncates_long_file_change_paths() {
        let item = json!({
            "type": "fileChange",
            "changes": [{
                "kind": "edit",
                "path": format!("src/{}", "very-long-path/".repeat(12))
            }]
        });

        let label = activity_label_from_item(&item).expect("label");
        assert!(label.starts_with("File edit: src/"));
        assert!(label.ends_with('…'));
    }

    #[test]
    fn activity_label_formats_web_open_page_action() {
        let item = json!({
            "type": "webSearch",
            "action": {
                "type": "openPage",
                "url": "https://example.com/docs"
            }
        });

        assert_eq!(
            activity_label_from_item(&item).as_deref(),
            Some("Opening: https://example.com/docs")
        );
    }

    #[test]
    fn thread_runtime_status_maps_to_normalized_session_status() {
        assert_eq!(
            map_thread_runtime_status("active"),
            crate::daemon::task_graph::types::SessionStatus::Active
        );
        assert_eq!(
            map_thread_runtime_status("systemError"),
            crate::daemon::task_graph::types::SessionStatus::Error
        );
        assert_eq!(
            map_thread_runtime_status("notLoaded"),
            crate::daemon::task_graph::types::SessionStatus::Paused
        );
    }
}
