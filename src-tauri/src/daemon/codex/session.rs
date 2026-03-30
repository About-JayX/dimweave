use crate::daemon::codex::structured_output::StreamPreviewState;
use crate::daemon::codex::ws_client::{CodexWsClient, WsRx, WsTx};
use crate::daemon::gui;
use crate::daemon::SharedState;
use serde_json::json;
use self::session_event::handle_codex_event;
use tauri::AppHandle;
use tokio::sync::mpsc;
#[path = "session_event.rs"]
mod session_event;
pub struct SessionOpts {
    pub role_id: String,
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub sandbox_mode: Option<String>,
    pub network_access: bool,
    pub base_instructions: Option<String>,
}

struct EventLoopCtx<'a> {
    thread_id: String,
    session_epoch: u64,
    role_id: &'a str,
    state: &'a SharedState,
    app: &'a AppHandle,
}

pub async fn run(
    port: u16,
    session_epoch: u64,
    opts: SessionOpts,
    state: SharedState,
    app: AppHandle,
    mut inject_rx: mpsc::Receiver<(String, bool)>,
    ready_tx: tokio::sync::oneshot::Sender<String>,
) {
    match CodexWsClient::connect(port, &opts, &app).await {
        Some((client, ws_rx)) => {
            let tid = client.thread_id().to_string();
            let ws_tx = client.sender().clone();
            let _ = ready_tx.send(tid.clone());
            let ctx = EventLoopCtx {
                thread_id: tid,
                session_epoch,
                role_id: &opts.role_id,
                state: &state,
                app: &app,
            };
            event_loop(ctx, &mut inject_rx, ws_tx, ws_rx).await;
        }
        None => {
            let _ = ready_tx.send(String::new());
        }
    }
}

async fn event_loop(
    ctx: EventLoopCtx<'_>,
    inject_rx: &mut mpsc::Receiver<(String, bool)>,
    ws_tx: WsTx,
    mut ws_rx: WsRx,
) {
    let EventLoopCtx {
        thread_id,
        session_epoch,
        role_id,
        state,
        app,
    } = ctx;
    let mut next_id: u64 = 100;
    let mut stream_preview = StreamPreviewState::default();
    let mut req_from_user: std::collections::HashMap<u64, bool> = std::collections::HashMap::new();
    let mut user_turn_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    loop {
        tokio::select! {
            msg_opt = ws_rx.recv() => {
                let Some(v) = msg_opt else { break };
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
    let cleared_current = {
        let mut daemon = state.write().await;
        daemon.clear_codex_session_if_current(session_epoch)
    };
    if cleared_current {
        gui::emit_agent_status(app, "codex", false, None);
        gui::emit_system_log(app, "info", "[Codex] session ended");
    }
}
