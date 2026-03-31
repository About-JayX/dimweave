use futures_util::StreamExt;
use serde_json::Value;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::tungstenite::Message;

pub(super) type FullWs = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

pub(super) async fn wait_for_rpc_id(ws: &mut FullWs, expected_id: u64, secs: u64) -> bool {
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

pub(super) async fn wait_for_thread_id(ws: &mut FullWs, secs: u64) -> Option<String> {
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

/// Spawn a bidirectional pump loop for a WS connection.
/// Forwards outbound text from `out_rx` and inbound JSON to `in_tx`.
pub(super) fn spawn_pump(
    mut ws: FullWs,
    mut out_rx: tokio::sync::mpsc::Receiver<String>,
    in_tx: tokio::sync::mpsc::Sender<Value>,
) {
    use futures_util::SinkExt;
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(text) = out_rx.recv() => {
                    if ws.send(Message::Text(text)).await.is_err() {
                        eprintln!("[Codex-WS] pump: outbound send failed");
                        break;
                    }
                }
                msg = ws.next() => {
                    match msg {
                        Some(Ok(ref m)) if m.is_text() => {
                            let t = m.to_text().unwrap_or("");
                            if let Ok(v) = serde_json::from_str::<Value>(t) {
                                if in_tx.send(v).await.is_err() {
                                    eprintln!("[Codex-WS] pump: in_tx closed");
                                    break;
                                }
                            }
                        }
                        Some(Ok(ref m)) if m.is_ping() => {}
                        Some(Ok(ref m)) if m.is_close() => {
                            eprintln!("[Codex-WS] pump: received Close frame");
                            break;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(e)) => {
                            eprintln!("[Codex-WS] pump: read error: {e}");
                            break;
                        }
                        None => {
                            eprintln!("[Codex-WS] pump: stream ended");
                            break;
                        }
                    }
                }
            }
        }
        eprintln!("[Codex-WS] pump loop exited");
    });
}
