use crate::types::{BridgeMsg, BridgeOutbound, DaemonInbound, DaemonMsg};
use futures_util::{SinkExt, StreamExt};
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const MAX_RETRIES: u32 = 20;

pub async fn run(
    port: u16,
    agent_id: String,
    push_tx: tokio::sync::mpsc::Sender<DaemonInbound>,
    mut reply_rx: tokio::sync::mpsc::Receiver<BridgeOutbound>,
) {
    let url = format!("ws://127.0.0.1:{port}/ws");
    let mut attempt = 0u32;

    loop {
        match connect_async(&url).await {
            Ok((ws, _)) => {
                eprintln!("[Bridge/{agent_id}] connected to daemon");
                let connected_at = tokio::time::Instant::now();
                let (mut sink, mut stream) = ws.split();

                let connect_msg = serde_json::to_string(&BridgeMsg::AgentConnect {
                    agent_id: &agent_id,
                })
                .unwrap_or_else(|e| {
                    eprintln!("[Bridge/{agent_id}] failed to serialize connect msg: {e}");
                    "{}".to_string()
                });
                if sink.send(Message::Text(connect_msg.into())).await.is_err() {
                    continue;
                }

                loop {
                    tokio::select! {
                        msg = stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(txt))) => {
                                    match serde_json::from_str::<DaemonMsg>(&txt) {
                                        Ok(dm) => {
                                            match dm {
                                                DaemonMsg::RoutedMessage { message } => {
                                                    if push_tx.send(DaemonInbound::RoutedMessage(message)).await.is_err() {
                                                        eprintln!("[Bridge/{agent_id}] push channel closed, exiting");
                                                        return;
                                                    }
                                                }
                                                DaemonMsg::PermissionVerdict { verdict } => {
                                                    if push_tx.send(DaemonInbound::PermissionVerdict(verdict)).await.is_err() {
                                                        eprintln!("[Bridge/{agent_id}] push channel closed, exiting");
                                                        return;
                                                    }
                                                }
                                                DaemonMsg::Status { .. } => {}
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("[Bridge/{agent_id}] failed to parse daemon msg: {e}");
                                        }
                                    }
                                }
                                Some(Ok(_)) => {}
                                _ => break,
                            }
                        }
                        Some(outbound) = reply_rx.recv() => {
                            let serialized = match outbound {
                                BridgeOutbound::AgentReply(reply) => serde_json::to_string(&BridgeMsg::AgentReply {
                                    message: &reply,
                                }),
                                BridgeOutbound::PermissionRequest(request) => serde_json::to_string(&BridgeMsg::PermissionRequest {
                                    request: &request,
                                }),
                            };
                            let Ok(msg) = serialized else {
                                eprintln!("[Bridge/{agent_id}] failed to serialize outbound message");
                                continue;
                            };
                            if sink.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                if connected_at.elapsed() > Duration::from_secs(2) {
                    attempt = 0;
                }
                eprintln!("[Bridge/{agent_id}] daemon connection dropped, reconnecting...");
            }
            Err(e) => {
                attempt += 1;
                if attempt >= MAX_RETRIES {
                    eprintln!("[Bridge/{agent_id}] max retries reached: {e}");
                    return;
                }
                let delay = Duration::from_millis(100 * (1u64 << attempt.min(6)));
                eprintln!(
                    "[Bridge/{agent_id}] connect failed (attempt {attempt}): {e}, retry in {delay:?}"
                );
                // Drain outbound queue during backoff to prevent MCP stdin blocking
                let deadline = tokio::time::Instant::now() + delay;
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep_until(deadline) => break,
                        msg = reply_rx.recv() => {
                            if msg.is_none() {
                                eprintln!("[Bridge/{agent_id}] reply channel closed during backoff");
                                return;
                            }
                            // Discard — we have no connection to send on
                        }
                    }
                }
            }
        }
    }
}
