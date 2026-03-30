use crate::daemon::codex::session::SessionOpts;
use crate::daemon::gui;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

type FullWs = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

/// Sender for outbound text messages (turn/start etc.)
pub(crate) type WsTx = mpsc::Sender<String>;
/// Receiver for inbound JSON-RPC messages (notifications + responses)
pub(crate) type WsRx = mpsc::Receiver<Value>;

/// Encapsulates a single WS connection to Codex app-server.
/// Each instance owns exactly one thread — switching threads requires
/// dropping this client and calling `connect()` again.
pub(crate) struct CodexWsClient {
    thread_id: String,
    ws_tx: WsTx,
}

impl CodexWsClient {
    /// Connect, handshake, then spawn a pump loop that:
    /// - forwards outbound text from `WsTx` to the WS sink,
    /// - forwards inbound JSON text from WS to `WsRx`,
    /// - auto-replies Pong to Ping (no split needed).
    pub async fn connect(
        port: u16,
        opts: &SessionOpts,
        app: &AppHandle,
    ) -> Option<(Self, WsRx)> {
        let url = format!("ws://127.0.0.1:{port}");
        let mut ws = match connect_async(&url).await {
            Ok((ws, _)) => ws,
            Err(e) => {
                gui::emit_system_log(app, "error", &format!("[Codex] connect failed: {e}"));
                return None;
            }
        };

        if !Self::do_initialize(&mut ws, app).await {
            return None;
        }
        let thread_id = Self::do_thread_start(&mut ws, opts, app).await?;
        gui::emit_system_log(app, "info", &format!("[Codex] thread={thread_id}"));

        // Channel pair: outbound text, inbound parsed JSON
        let (out_tx, mut out_rx) = mpsc::channel::<String>(64);
        let (in_tx, in_rx) = mpsc::channel::<Value>(128);

        // Single pump loop — owns the unsplit WS, handles Ping/Pong internally.
        // Using unsplit avoids the borrow-split lifetime issues that previously caused
        // first-message loss and post-reconnect non-delivery.
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(text) = out_rx.recv() => {
                        if ws.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    msg = ws.next() => {
                        match msg {
                            Some(Ok(ref m)) if m.is_text() => {
                                let t = m.to_text().unwrap_or("");
                                if let Ok(v) = serde_json::from_str::<Value>(t) {
                                    if in_tx.send(v).await.is_err() {
                                        break;
                                    }
                                }
                            }
                            Some(Ok(ref m)) if m.is_ping() => {
                                // tungstenite auto-replies Pong; nothing to do
                            }
                            Some(Ok(_)) => {}
                            Some(Err(_)) => break,
                            None => break,
                        }
                    }
                }
            }
        });

        Some((Self { thread_id, ws_tx: out_tx }, in_rx))
    }

    pub fn thread_id(&self) -> &str { &self.thread_id }
    pub fn sender(&self) -> &WsTx { &self.ws_tx }

    async fn do_initialize(ws: &mut FullWs, app: &AppHandle) -> bool {
        let msg = json!({
            "method": "initialize", "id": 1,
            "params": {
                "clientInfo": {"name":"agentnexus","version":"0.1.0"},
                "capabilities": {"experimentalApi": true}
            }
        });
        if ws.send(Message::Text(msg.to_string())).await.is_err() {
            gui::emit_system_log(app, "error", "[Codex] failed to send initialize");
            return false;
        }
        if !wait_for_rpc_id(ws, 1, 30).await {
            gui::emit_system_log(app, "error", "[Codex] initialize timed out");
            return false;
        }
        let ack = json!({"method":"initialized","params":{}});
        ws.send(Message::Text(ack.to_string())).await.is_ok()
    }

    async fn do_thread_start(
        ws: &mut FullWs,
        opts: &SessionOpts,
        app: &AppHandle,
    ) -> Option<String> {
        let params = super::handshake::build_thread_start_params(opts);
        let msg = json!({"method":"thread/start","id":2,"params":params});
        if ws.send(Message::Text(msg.to_string())).await.is_err() {
            gui::emit_system_log(app, "error", "[Codex] failed to send thread/start");
            return None;
        }
        match wait_for_thread_id(ws, 30).await {
            Some(tid) if !tid.is_empty() => Some(tid),
            _ => {
                gui::emit_system_log(app, "error", "[Codex] thread/start failed or timed out");
                None
            }
        }
    }
}

async fn wait_for_rpc_id(ws: &mut FullWs, expected_id: u64, secs: u64) -> bool {
    timeout(Duration::from_secs(secs), async {
        while let Some(Ok(msg)) = ws.next().await {
            let Ok(text) = msg.to_text() else { continue };
            let Ok(v) = serde_json::from_str::<Value>(text) else { continue };
            if v["id"].as_u64() == Some(expected_id) {
                return true;
            }
        }
        false
    })
    .await
    .unwrap_or(false)
}

async fn wait_for_thread_id(ws: &mut FullWs, secs: u64) -> Option<String> {
    timeout(Duration::from_secs(secs), async {
        while let Some(Ok(msg)) = ws.next().await {
            let Ok(text) = msg.to_text() else { continue };
            let Ok(v) = serde_json::from_str::<Value>(text) else { continue };
            if v["id"].as_u64() == Some(2) {
                if v.get("error").is_some() {
                    let err = serde_json::to_string(&v["error"]).unwrap_or_default();
                    eprintln!("[Codex] thread/start error: {err}");
                }
                return v["result"]["thread"]["id"]
                    .as_str().unwrap_or("").to_string();
            }
            if v["method"].as_str() == Some("thread/started") {
                if let Some(tid) = v["params"]["thread"]["id"].as_str() {
                    return tid.to_string();
                }
            }
        }
        String::new()
    })
    .await
    .ok()
}
