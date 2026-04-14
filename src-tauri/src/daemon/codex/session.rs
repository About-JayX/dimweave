use self::session_event::handle_codex_event;
use crate::daemon::codex::structured_output::StreamPreviewState;
use crate::daemon::codex::ws_client::{CodexWsClient, WsRx, WsTx};
use crate::daemon::gui;
use crate::daemon::SharedState;
use serde_json::json;
use tauri::AppHandle;
use tokio::sync::mpsc;
#[path = "session_event.rs"]
mod session_event;

const MAX_RECONNECT_ATTEMPTS: u32 = 5;
const RECONNECT_BASE_DELAY_MS: u64 = 500;

pub struct SessionOpts {
    pub role_id: String,
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub sandbox_mode: Option<String>,
    pub network_access: bool,
    pub base_instructions: Option<String>,
}

pub async fn run(
    port: u16,
    session_epoch: u64,
    task_id: String,
    agent_id: String,
    opts: SessionOpts,
    state: SharedState,
    app: AppHandle,
    mut inject_rx: mpsc::Receiver<(Vec<serde_json::Value>, bool)>,
    ready_tx: tokio::sync::oneshot::Sender<String>,
) {
    match CodexWsClient::connect(port, &opts, &app).await {
        Some((client, ws_rx)) => {
            let thread_id = client.thread_id().to_string();
            let ws_tx = client.sender().clone();
            let _ = ready_tx.send(thread_id.clone());
            run_with_reconnect(
                port,
                session_epoch,
                &task_id,
                &agent_id,
                &opts.role_id,
                &state,
                &app,
                &mut inject_rx,
                thread_id,
                ws_tx,
                ws_rx,
            )
            .await;
        }
        None => {
            let _ = ready_tx.send(String::new());
        }
    }
}

pub async fn resume(
    port: u16,
    session_epoch: u64,
    task_id: String,
    agent_id: String,
    role_id: String,
    thread_id: String,
    state: SharedState,
    app: AppHandle,
    mut inject_rx: mpsc::Receiver<(Vec<serde_json::Value>, bool)>,
    ready_tx: tokio::sync::oneshot::Sender<String>,
) {
    match CodexWsClient::reconnect(port, &thread_id, &app).await {
        Some((client, ws_rx)) => {
            let ws_tx = client.sender().clone();
            let _ = ready_tx.send(thread_id.clone());
            run_with_reconnect(
                port,
                session_epoch,
                &task_id,
                &agent_id,
                &role_id,
                &state,
                &app,
                &mut inject_rx,
                thread_id,
                ws_tx,
                ws_rx,
            )
            .await;
        }
        None => {
            let _ = ready_tx.send(String::new());
        }
    }
}

async fn run_with_reconnect(
    port: u16,
    session_epoch: u64,
    task_id: &str,
    agent_id: &str,
    role_id: &str,
    state: &SharedState,
    app: &AppHandle,
    inject_rx: &mut mpsc::Receiver<(Vec<serde_json::Value>, bool)>,
    thread_id: String,
    mut ws_tx: WsTx,
    mut ws_rx: WsRx,
) {
    let mut reconnect_count: u32 = 0;
    loop {
        let reason = event_loop(
            &thread_id,
            session_epoch,
            role_id,
            task_id,
            agent_id,
            state,
            app,
            inject_rx,
            &ws_tx,
            &mut ws_rx,
        )
        .await;

        if reason != LoopExit::WsClosed {
            break;
        }
        // WS closed — check if process is still alive before reconnecting
        if !is_port_alive(port).await {
            eprintln!("[Codex] app-server unreachable, not reconnecting");
            break;
        }
        reconnect_count += 1;
        if reconnect_count > MAX_RECONNECT_ATTEMPTS {
            eprintln!("[Codex] max reconnect attempts ({MAX_RECONNECT_ATTEMPTS}) reached");
            break;
        }
        let delay = RECONNECT_BASE_DELAY_MS * 2u64.pow(reconnect_count - 1);
        eprintln!("[Codex] WS lost, reconnecting ({reconnect_count}/{MAX_RECONNECT_ATTEMPTS}) in {delay}ms");
        gui::emit_system_log(
            app,
            "warn",
            &format!("[Codex] reconnecting ({reconnect_count}/{MAX_RECONNECT_ATTEMPTS})…"),
        );
        tokio::time::sleep(std::time::Duration::from_millis(delay)).await;

        match CodexWsClient::reconnect(port, &thread_id, app).await {
            Some((client, new_rx)) => {
                ws_tx = client.sender().clone();
                ws_rx = new_rx;
                reconnect_count = 0; // reset on success
                gui::emit_system_log(app, "info", "[Codex] reconnected");
                eprintln!("[Codex] reconnected to thread={thread_id}");
            }
            None => {
                eprintln!("[Codex] reconnect handshake failed");
                break;
            }
        }
    }
    let cleanup = {
        let mut s = state.write().await;
        let cleared = s.clear_codex_task_session_for_agent(task_id, agent_id, session_epoch);
        let any_online = s.is_codex_online();
        (cleared, any_online)
    };
    if cleanup.0.is_some() && !cleanup.1 {
        gui::emit_agent_status(app, "codex", false, None, None);
    }
    if cleanup.0.is_some() {
        gui::emit_system_log(app, "info", "[Codex] session ended");
    }
    if let Some(tid) = cleanup.0 {
        crate::daemon::gui_task::emit_task_context_events(state, app, &tid).await;
    }
}

#[derive(PartialEq)]
enum LoopExit {
    WsClosed,
    InjectClosed,
    SendFailed,
}

#[derive(Default)]
struct RoutedTurnTracker {
    pending_injected_requests: std::collections::HashSet<u64>,
    routed_turn_ids: std::collections::HashSet<String>,
}

impl RoutedTurnTracker {
    fn track_injected_request(&mut self, rpc_id: u64, _from_user: bool) {
        self.pending_injected_requests.insert(rpc_id);
    }

    fn bind_turn_from_response(&mut self, response: &serde_json::Value) {
        let Some(rpc_id) = response["id"].as_u64() else {
            return;
        };
        let Some(turn_id) = response["result"]["turn"]["id"].as_str() else {
            return;
        };
        if self.pending_injected_requests.remove(&rpc_id) {
            self.routed_turn_ids.insert(turn_id.to_string());
        }
    }

    fn is_routed_turn(&self, turn_id: &str) -> bool {
        self.routed_turn_ids.contains(turn_id)
    }

    fn complete_turn(&mut self, turn_id: &str) {
        self.routed_turn_ids.remove(turn_id);
    }
}

async fn event_loop(
    thread_id: &str,
    session_epoch: u64,
    role_id: &str,
    task_id: &str,
    agent_id: &str,
    state: &SharedState,
    app: &AppHandle,
    inject_rx: &mut mpsc::Receiver<(Vec<serde_json::Value>, bool)>,
    ws_tx: &WsTx,
    ws_rx: &mut WsRx,
) -> LoopExit {
    let mut next_id: u64 = 100;
    let mut stream_preview = StreamPreviewState::default();
    let mut routed_turns = RoutedTurnTracker::default();
    let _ = (session_epoch, thread_id); // used by callers for context
    loop {
        tokio::select! {
            msg_opt = ws_rx.recv() => {
                let Some(v) = msg_opt else {
                    eprintln!("[Codex] event_loop: ws_rx closed");
                    return LoopExit::WsClosed;
                };
                routed_turns.bind_turn_from_response(&v);
                let turn_id = v["params"]["turnId"].as_str()
                    .or_else(|| v["params"]["turn"]["id"].as_str())
                    .unwrap_or("");
                let route_ok = routed_turns.is_routed_turn(turn_id);
                handle_codex_event(
                    &v, role_id, task_id, agent_id, route_ok, state, app, ws_tx, &mut stream_preview,
                ).await;
                if v["method"].as_str() == Some("turn/completed") {
                    routed_turns.complete_turn(turn_id);
                }
            }
            inject = inject_rx.recv() => {
                let Some((items, from_user)) = inject else {
                    eprintln!("[Codex] event_loop: inject_rx closed");
                    return LoopExit::InjectClosed;
                };
                let id = next_id; next_id += 1;
                routed_turns.track_injected_request(id, from_user);
                let mut turn_params = json!({
                    "threadId": thread_id,
                    "input": items,
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
                    return LoopExit::SendFailed;
                }
            }
        }
    }
}

async fn is_port_alive(port: u16) -> bool {
    tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
        .await
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn injected_turns_are_routable_even_when_not_from_user() {
        let mut tracker = RoutedTurnTracker::default();

        tracker.track_injected_request(101, false);
        tracker.bind_turn_from_response(&json!({
            "id": 101,
            "result": { "turn": { "id": "turn_agent_reply" } }
        }));

        assert!(tracker.is_routed_turn("turn_agent_reply"));
    }

    #[test]
    fn completed_turns_are_removed_from_routable_set() {
        let mut tracker = RoutedTurnTracker::default();

        tracker.track_injected_request(202, true);
        tracker.bind_turn_from_response(&json!({
            "id": 202,
            "result": { "turn": { "id": "turn_user" } }
        }));
        tracker.complete_turn("turn_user");

        assert!(!tracker.is_routed_turn("turn_user"));
    }
}
