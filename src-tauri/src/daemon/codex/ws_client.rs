use crate::daemon::codex::session::SessionOpts;
use crate::daemon::codex::ws_helpers::{self, FullWs};
use crate::daemon::gui;
use futures_util::SinkExt;
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Sender for outbound text messages (turn/start etc.)
pub(crate) type WsTx = mpsc::Sender<String>;
/// Receiver for inbound JSON-RPC messages (notifications + responses)
pub(crate) type WsRx = mpsc::Receiver<Value>;

/// Encapsulates a single WS connection to Codex app-server.
pub(crate) struct CodexWsClient {
    thread_id: String,
    ws_tx: WsTx,
}

impl CodexWsClient {
    /// Connect, initialize, start a new thread, and spawn pump loop.
    pub async fn connect(port: u16, opts: &SessionOpts, app: &AppHandle) -> Option<(Self, WsRx)> {
        let mut ws = open_ws(port, app).await?;
        do_initialize(&mut ws, app).await?;
        let thread_id = do_thread_start(&mut ws, opts, app).await?;
        gui::emit_system_log(app, "info", &format!("[Codex] thread={thread_id}"));

        let (out_tx, out_rx) = mpsc::channel::<String>(64);
        let (in_tx, in_rx) = mpsc::channel::<Value>(128);
        ws_helpers::spawn_pump(ws, out_rx, in_tx);
        Some((
            Self {
                thread_id,
                ws_tx: out_tx,
            },
            in_rx,
        ))
    }

    /// Reconnect to an existing thread after WS drop.
    pub async fn reconnect(port: u16, thread_id: &str, app: &AppHandle) -> Option<(Self, WsRx)> {
        let mut ws = open_ws(port, app).await?;
        do_initialize(&mut ws, app).await?;
        do_thread_resume(&mut ws, thread_id, app).await?;

        let (out_tx, out_rx) = mpsc::channel::<String>(64);
        let (in_tx, in_rx) = mpsc::channel::<Value>(128);
        ws_helpers::spawn_pump(ws, out_rx, in_tx);
        Some((
            Self {
                thread_id: thread_id.to_string(),
                ws_tx: out_tx,
            },
            in_rx,
        ))
    }

    pub fn thread_id(&self) -> &str {
        &self.thread_id
    }
    pub fn sender(&self) -> &WsTx {
        &self.ws_tx
    }
}

pub(crate) async fn thread_list(
    port: u16,
    params: Value,
    app: &AppHandle,
) -> Result<Value, String> {
    rpc_call(port, 20, "thread/list", params, app).await
}

pub(crate) async fn thread_fork(
    port: u16,
    thread_id: &str,
    app: &AppHandle,
) -> Result<String, String> {
    let result = rpc_call(
        port,
        21,
        "thread/fork",
        json!({ "threadId": thread_id }),
        app,
    )
    .await?;
    result["thread"]["id"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| "invalid thread/fork response: missing thread.id".to_string())
}

pub(crate) async fn thread_archive(
    port: u16,
    thread_id: &str,
    app: &AppHandle,
) -> Result<(), String> {
    rpc_call(
        port,
        22,
        "thread/archive",
        json!({ "threadId": thread_id }),
        app,
    )
    .await
    .map(|_| ())
}

async fn open_ws(port: u16, app: &AppHandle) -> Option<FullWs> {
    let url = format!("ws://127.0.0.1:{port}");
    match connect_async(&url).await {
        Ok((ws, _)) => Some(ws),
        Err(e) => {
            gui::emit_system_log(app, "error", &format!("[Codex] connect failed: {e}"));
            None
        }
    }
}

async fn rpc_call(
    port: u16,
    id: u64,
    method: &str,
    params: Value,
    app: &AppHandle,
) -> Result<Value, String> {
    let mut ws = open_ws(port, app)
        .await
        .ok_or_else(|| format!("[Codex] connect failed for {method}"))?;
    do_initialize(&mut ws, app)
        .await
        .ok_or_else(|| format!("[Codex] initialize failed for {method}"))?;
    let msg = json!({
        "method": method,
        "id": id,
        "params": params,
    });
    ws.send(Message::Text(msg.to_string()))
        .await
        .map_err(|e| format!("[Codex] failed to send {method}: {e}"))?;
    let response = ws_helpers::wait_for_rpc_response(&mut ws, id, 30)
        .await
        .ok_or_else(|| format!("[Codex] {method} timed out"))?;
    if response.get("error").is_some() {
        return Err(format!(
            "[Codex] {method} failed: {}",
            serde_json::to_string(&response["error"]).unwrap_or_default()
        ));
    }
    Ok(response["result"].clone())
}

async fn do_initialize(ws: &mut FullWs, app: &AppHandle) -> Option<()> {
    let msg = json!({
        "method": "initialize", "id": 1,
        "params": {
            "clientInfo": {"name":"dimweave","version":"0.1.0"},
            "capabilities": {"experimentalApi": true}
        }
    });
    if ws.send(Message::Text(msg.to_string())).await.is_err() {
        gui::emit_system_log(app, "error", "[Codex] failed to send initialize");
        return None;
    }
    if !ws_helpers::wait_for_rpc_id(ws, 1, 30).await {
        gui::emit_system_log(app, "error", "[Codex] initialize timed out");
        return None;
    }
    let ack = json!({"method":"initialized","params":{}});
    ws.send(Message::Text(ack.to_string())).await.ok()?;
    Some(())
}

async fn do_thread_start(ws: &mut FullWs, opts: &SessionOpts, app: &AppHandle) -> Option<String> {
    let params = super::handshake::build_thread_start_params(opts);
    let msg = json!({"method":"thread/start","id":2,"params":params});
    if ws.send(Message::Text(msg.to_string())).await.is_err() {
        gui::emit_system_log(app, "error", "[Codex] failed to send thread/start");
        return None;
    }
    match ws_helpers::wait_for_thread_id(ws, 30).await {
        Some(tid) if !tid.is_empty() => Some(tid),
        _ => {
            gui::emit_system_log(app, "error", "[Codex] thread/start failed or timed out");
            None
        }
    }
}

async fn do_thread_resume(ws: &mut FullWs, thread_id: &str, app: &AppHandle) -> Option<()> {
    let msg = json!({
        "method": "thread/resume", "id": 3,
        "params": { "threadId": thread_id }
    });
    if ws.send(Message::Text(msg.to_string())).await.is_err() {
        gui::emit_system_log(app, "error", "[Codex] failed to send thread/resume");
        return None;
    }
    if !ws_helpers::wait_for_rpc_id(ws, 3, 30).await {
        gui::emit_system_log(app, "error", "[Codex] thread/resume timed out");
        return None;
    }
    Some(())
}
