use crate::daemon::codex::handler;
use crate::daemon::codex::handshake::{WsStream, WsTx};
use crate::daemon::{gui, SharedState};
use futures_util::StreamExt;
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc;

pub struct SessionOpts {
    pub role_id: String,
    pub cwd: String,
    pub model: Option<String>,
    pub sandbox_mode: Option<String>,
    pub developer_instructions: Option<String>,
}

/// Connect to a running Codex app-server, initialize the session, and enter
/// the event loop.  Sends the thread ID on `ready_tx` after successful handshake,
/// or empty string on failure.
pub async fn run(
    port: u16,
    opts: SessionOpts,
    state: SharedState,
    app: AppHandle,
    mut inject_rx: mpsc::Receiver<String>,
    ready_tx: tokio::sync::oneshot::Sender<String>,
) {
    match super::handshake::handshake(port, &opts, &app).await {
        Some((tid, ws_tx, stream)) => {
            let _ = ready_tx.send(tid.clone());
            event_loop(
                tid,
                &opts.role_id,
                &state,
                &app,
                &mut inject_rx,
                ws_tx,
                stream,
            )
            .await;
        }
        None => {
            let _ = ready_tx.send(String::new());
        }
    }
}

/// Main event loop after successful handshake.
async fn event_loop(
    thread_id: String,
    role_id: &str,
    state: &SharedState,
    app: &AppHandle,
    inject_rx: &mut mpsc::Receiver<String>,
    ws_tx: WsTx,
    mut stream: WsStream,
) {
    let mut next_id: u64 = 100;
    loop {
        tokio::select! {
            msg_opt = stream.next() => {
                let Some(Ok(msg)) = msg_opt else { break };
                let Ok(v) = serde_json::from_str::<Value>(&msg.to_text().unwrap_or("")) else { continue };
                if v["method"].as_str() == Some("item/tool/call") {
                    if let (Some(id), Some(name)) = (v["id"].as_u64(), v["params"]["name"].as_str()) {
                        let args = v["params"]["arguments"].clone();
                        handler::handle_dynamic_tool(id, name, &args, role_id, state, app, &ws_tx).await;
                    }
                }
            }
            inject = inject_rx.recv() => {
                let Some(text) = inject else { break };
                let id = next_id; next_id += 1;
                if ws_tx.send(json!({
                    "method": "turn/start", "id": id,
                    "params": {"threadId": &thread_id, "input": [{"type":"text","text":text}]}
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
