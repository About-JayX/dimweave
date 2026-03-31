use crate::daemon::codex::handler;
use crate::daemon::codex::ws_client::WsTx;
use crate::daemon::codex::structured_output::{
    parse_structured_output, should_emit_final_message, StreamPreviewState,
};
use crate::daemon::gui::{self, CodexStreamPayload};
use crate::daemon::types::{BridgeMessage, MessageStatus};
use crate::daemon::{routing, SharedState};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;

pub(super) async fn handle_codex_event(
    v: &Value,
    role_id: &str,
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
        "item/tool/call" => handle_tool_call(v, role_id, state, app, ws_tx).await,
        "turn/started" => {
            stream_preview.reset();
            gui::emit_codex_stream(app, CodexStreamPayload::Thinking);
        }
        "item/agentMessage/delta" => {
            if let Some(text) = v["params"]["delta"].as_str().filter(|text| !text.is_empty()) {
                if let Some(preview) = stream_preview.ingest_delta(text) {
                    gui::emit_codex_stream(app, CodexStreamPayload::Delta { text: preview });
                }
            }
        }
        "item/completed" => {
            handle_completed_agent_message(v, role_id, schema_route_enabled, state, app, stream_preview)
                .await;
        }
        "turn/completed" => {
            stream_preview.reset();
            let status = v["params"]["turn"]["status"].as_str().unwrap_or("unknown");
            gui::emit_codex_stream(app, CodexStreamPayload::TurnDone { status: status.into() });
        }
        _ => {}
    }
}

async fn handle_tool_call(
    v: &Value,
    role_id: &str,
    state: &SharedState,
    app: &AppHandle,
    ws_tx: &WsTx,
) {
    let name = v["params"]["tool"]
        .as_str()
        .or_else(|| v["params"]["name"].as_str());
    if let (Some(id), Some(name)) = (v["id"].as_u64(), name) {
        let args = v["params"]["arguments"].clone();
        handler::handle_dynamic_tool(id, name, &args, role_id, state, app, ws_tx).await;
    }
}

async fn handle_completed_agent_message(
    v: &Value,
    role_id: &str,
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
            let error_msg = build_msg_with_status(role_id, "user", &hint, MessageStatus::Error);
            gui::emit_agent_message(app, &error_msg);
            return;
        }
    };
    if !should_emit_final_message(&parsed.message) {
        return;
    }
    gui::emit_codex_stream(app, CodexStreamPayload::Message {
        text: parsed.message.clone(),
    });
    let valid_target = if schema_route_enabled {
        parsed
            .send_to
            .as_deref()
            .filter(|target| matches!(*target, "lead" | "coder" | "reviewer"))
    } else {
        None
    };
    if let Some(target) = valid_target {
        let mut msg = build_msg_with_status(role_id, target, &parsed.message, parsed.status);
        state.read().await.stamp_message_context(role_id, &mut msg);
        eprintln!("[Codex] schema-route {} → {}", role_id, target);
        routing::route_message(state, app, msg).await;
    } else {
        let mut msg = build_msg_with_status(role_id, "user", &parsed.message, parsed.status);
        state.read().await.stamp_message_context(role_id, &mut msg);
        gui::emit_agent_message(app, &msg);
    }
}

static MSG_SEQ: AtomicU64 = AtomicU64::new(0);

fn build_msg_with_status(
    from: &str,
    to: &str,
    content: &str,
    status: MessageStatus,
) -> BridgeMessage {
    let seq = MSG_SEQ.fetch_add(1, Ordering::Relaxed);
    BridgeMessage {
        id: format!("codex_{}_{seq}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        display_source: Some("codex".into()),
        to: to.to_string(),
        content: content.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(status),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("codex".into()),
    }
}
