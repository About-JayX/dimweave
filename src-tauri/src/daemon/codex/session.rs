use crate::daemon::codex::handler;
use crate::daemon::codex::handshake::{WsStream, WsTx};
use crate::daemon::gui::{self, CodexStreamPayload};
use crate::daemon::types::BridgeMessage;
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
    // request_id → from_user queue: maps turn/start request id to whether user initiated
    let mut req_from_user: std::collections::HashMap<u64, bool> = std::collections::HashMap::new();
    // turn_id → from_user: resolved when turn/start response arrives
    let mut user_turn_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    loop {
        tokio::select! {
            msg_opt = stream.next() => {
                let Some(Ok(msg)) = msg_opt else { break };
                let Ok(v) = serde_json::from_str::<Value>(msg.to_text().unwrap_or("")) else {
                    continue;
                };
                // turn/start response: map request_id → turn_id
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
                handle_codex_event(&v, role_id, route_ok, state, app, &ws_tx).await;
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
                // Remove outputSchema if null (shouldn't happen but be safe)
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
            gui::emit_codex_stream(app, CodexStreamPayload::Thinking);
        }
        "item/agentMessage/delta" => {
            if let Some(text) = v["params"]["delta"].as_str() {
                if !text.is_empty() {
                    gui::emit_codex_stream(app, CodexStreamPayload::Delta { text: text.into() });
                }
            }
        }
        "item/completed" => {
            if v["params"]["item"]["type"].as_str() == Some("agentMessage") {
                let raw = v["params"]["item"]["text"].as_str().unwrap_or("");
                if raw.is_empty() {
                    return;
                }
                let (display_text, send_to) = parse_structured_output(raw);
                gui::emit_codex_stream(app, CodexStreamPayload::Message {
                    text: display_text.clone(),
                });
                // Only schema-route on user-initiated turns (prevents feedback loops)
                let valid_target = if schema_route_enabled {
                    send_to.as_deref().filter(|t| {
                        matches!(*t, "lead" | "coder" | "reviewer" | "tester")
                    })
                } else {
                    None
                };
                if let Some(target) = valid_target {
                    // Route to another agent — route_message emits to GUI internally
                    let msg = build_msg(role_id, target, &display_text);
                    eprintln!("[Codex] schema-route {} → {}", role_id, target);
                    routing::route_message(state, app, msg).await;
                } else {
                    // No routing — show as local message to user
                    let msg = build_msg(role_id, "user", &display_text);
                    gui::emit_agent_message(app, &msg);
                }
            }
        }
        "turn/completed" => {
            let status = v["params"]["turn"]["status"].as_str().unwrap_or("unknown");
            gui::emit_codex_stream(app, CodexStreamPayload::TurnDone { status: status.into() });
        }
        _ => {}
    }
}

/// Parse Codex structured output `{ "message": "...", "send_to": "..." }`.
/// Falls back to raw text if not valid JSON.
fn parse_structured_output(raw: &str) -> (String, Option<String>) {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) {
        let message = v["message"].as_str().unwrap_or(raw).to_string();
        let send_to = v["send_to"].as_str().map(|s| s.to_string());
        (message, send_to)
    } else {
        (raw.to_string(), None)
    }
}

static MSG_SEQ: AtomicU64 = AtomicU64::new(0);

fn build_msg(from: &str, to: &str, content: &str) -> BridgeMessage {
    let seq = MSG_SEQ.fetch_add(1, Ordering::Relaxed);
    BridgeMessage {
        id: format!("codex_{}_{seq}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        to: to.to_string(),
        content: content.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
    }
}
