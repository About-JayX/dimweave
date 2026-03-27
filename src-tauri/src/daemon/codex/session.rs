use crate::daemon::codex::handler;
use crate::daemon::codex::handshake::{WsStream, WsTx};
use crate::daemon::codex::structured_output::{
    StreamPreviewState, parse_structured_output, should_emit_final_message,
};
use crate::daemon::gui::{self, CodexStreamPayload};
use crate::daemon::types::{BridgeMessage, MessageStatus};
use crate::daemon::{routing, SharedState};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::AppHandle;
use tokio::sync::mpsc;
pub struct SessionOpts {
    pub role_id: String,
    pub cwd: String,
    pub model: Option<String>,
    pub sandbox_mode: Option<String>,
    pub base_instructions: Option<String>,
}

pub async fn run(
    port: u16,
    opts: SessionOpts,
    state: SharedState,
    app: AppHandle,
    mut inject_rx: mpsc::Receiver<(String, bool)>,
    ready_tx: tokio::sync::oneshot::Sender<String>,
) {
    match super::handshake::handshake(port, &opts, &app).await {
        Some((tid, ws_tx, stream)) => {
            let _ = ready_tx.send(tid.clone());
            event_loop(tid, &opts.role_id, &state, &app, &mut inject_rx, ws_tx, stream).await;
        }
        None => {
            let _ = ready_tx.send(String::new());
        }
    }
}

async fn event_loop(
    thread_id: String,
    role_id: &str,
    state: &SharedState,
    app: &AppHandle,
    inject_rx: &mut mpsc::Receiver<(String, bool)>,
    ws_tx: WsTx,
    mut stream: WsStream,
) {
    let mut next_id: u64 = 100;
    let mut stream_preview = StreamPreviewState::default();
    let mut req_from_user: std::collections::HashMap<u64, bool> = std::collections::HashMap::new();
    let mut user_turn_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    loop {
        tokio::select! {
            msg_opt = stream.next() => {
                let Some(Ok(msg)) = msg_opt else { break };
                let Ok(v) = serde_json::from_str::<Value>(msg.to_text().unwrap_or("")) else {
                    continue;
                };
                if let Some(rpc_id) = v["id"].as_u64() {
                    if let Some(tid) = v["result"]["turn"]["id"].as_str() {
                        if req_from_user.remove(&rpc_id) == Some(true) {
                            user_turn_ids.insert(tid.to_string());
                        }
                    }
                }
                let turn_id = v["params"]["turnId"].as_str()
                    .or_else(|| v["params"]["turn"]["id"].as_str())
                    .unwrap_or("");
                let route_ok = user_turn_ids.contains(turn_id);
                handle_codex_event(
                    &v,
                    role_id,
                    route_ok,
                    state,
                    app,
                    &ws_tx,
                    &mut stream_preview,
                ).await;
                if v["method"].as_str() == Some("turn/completed") {
                    user_turn_ids.remove(turn_id);
                }
            }
            inject = inject_rx.recv() => {
                let Some((text, from_user)) = inject else { break };
                let id = next_id; next_id += 1;
                req_from_user.insert(id, from_user);
                let mut turn_params = json!({
                    "threadId": &thread_id,
                    "input": [{"type":"text","text":text}],
                    "outputSchema": crate::daemon::role_config::output_schema()
                });
                if turn_params["outputSchema"].is_null() {
                    turn_params.as_object_mut().map(|m| m.remove("outputSchema"));
                }
                if ws_tx.send(json!({
                    "method": "turn/start", "id": id,
                    "params": turn_params
                }).to_string()).await.is_err() {
                    eprintln!("[Codex] failed to inject turn/start");
                    break;
                }
            }
        }
    }
    state.write().await.codex_inject_tx = None;
    gui::emit_agent_status(app, "codex", false, None);
    gui::emit_system_log(app, "info", "[Codex] session ended");
}

async fn handle_codex_event(
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
        "item/tool/call" => {
            let name = v["params"]["tool"]
                .as_str()
                .or_else(|| v["params"]["name"].as_str());
            if let (Some(id), Some(name)) = (v["id"].as_u64(), name) {
                let args = v["params"]["arguments"].clone();
                handler::handle_dynamic_tool(id, name, &args, role_id, state, app, ws_tx).await;
            }
        }
        "turn/started" => {
            stream_preview.reset();
            gui::emit_codex_stream(app, CodexStreamPayload::Thinking);
        }
        "item/agentMessage/delta" => {
            if let Some(text) = v["params"]["delta"].as_str() {
                if !text.is_empty() {
                    if let Some(preview) = stream_preview.ingest_delta(text) {
                        gui::emit_codex_stream(app, CodexStreamPayload::Delta { text: preview });
                    }
                }
            }
        }
        "item/completed" => {
            if v["params"]["item"]["type"].as_str() == Some("agentMessage") {
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
                        let error_msg =
                            build_msg_with_status(role_id, "user", &hint, MessageStatus::Error);
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
                    parsed.send_to.as_deref().filter(|t| {
                        matches!(*t, "lead" | "coder" | "reviewer")
                    })
                } else {
                    None
                };
                if let Some(target) = valid_target {
                    let msg =
                        build_msg_with_status(role_id, target, &parsed.message, parsed.status);
                    eprintln!("[Codex] schema-route {} → {}", role_id, target);
                    routing::route_message(state, app, msg).await;
                } else {
                    let msg =
                        build_msg_with_status(role_id, "user", &parsed.message, parsed.status);
                    gui::emit_agent_message(app, &msg);
                }
            }
        }
        "turn/completed" => {
            stream_preview.reset();
            let status = v["params"]["turn"]["status"].as_str().unwrap_or("unknown");
            gui::emit_codex_stream(app, CodexStreamPayload::TurnDone { status: status.into() });
        }
        _ => {}
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
        to: to.to_string(),
        content: content.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(status),
    }
}
