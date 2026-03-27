use crate::types::{BridgeMsg, BridgeOutbound, DaemonInbound, DaemonMsg};
use futures_util::{SinkExt, StreamExt};
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

const MAX_RETRIES: u32 = 20;
const BACKOFF_BUF_CAP: usize = 64;

pub async fn run(
    port: u16,
    agent_id: String,
    push_tx: tokio::sync::mpsc::Sender<DaemonInbound>,
    mut reply_rx: tokio::sync::mpsc::Receiver<BridgeOutbound>,
) {
    let url = format!("ws://127.0.0.1:{port}/ws");
    let mut attempt = 0u32;
    let mut pending: Vec<BridgeOutbound> = Vec::new();

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
                if sink.send(Message::Text(connect_msg)).await.is_err() {
                    continue;
                }

                // Replay messages buffered during backoff — on send failure,
                // put remaining back into pending for next reconnect attempt
                {
                    let mut replay_failed = false;
                    let mut remaining: Vec<BridgeOutbound> = Vec::new();
                    for m in pending.drain(..) {
                        if replay_failed {
                            remaining.push(m);
                            continue;
                        }
                        if let Ok(s) = serialize_outbound(&agent_id, &m) {
                            if sink.send(Message::Text(s)).await.is_err() {
                                remaining.push(m);
                                replay_failed = true;
                            }
                        }
                    }
                    if !remaining.is_empty() {
                        pending = remaining;
                        continue; // reconnect — pending preserved
                    }
                }

                loop {
                    tokio::select! {
                        msg = stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(txt))) => {
                                    handle_inbound(&agent_id, &txt, &push_tx).await;
                                }
                                Some(Ok(_)) => {}
                                _ => break,
                            }
                        }
                        Some(outbound) = reply_rx.recv() => {
                            if let Ok(s) = serialize_outbound(&agent_id, &outbound) {
                                if sink.send(Message::Text(s)).await.is_err() {
                                    // Send failed — buffer for next reconnect
                                    pending.push(outbound);
                                    break;
                                }
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
                // Buffer outbound during backoff — replayed after reconnect
                let deadline = tokio::time::Instant::now() + delay;
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep_until(deadline) => break,
                        msg = reply_rx.recv() => {
                            match msg {
                                Some(m) => {
                                    if pending.len() < BACKOFF_BUF_CAP {
                                        pending.push(m);
                                    } else {
                                        eprintln!("[Bridge/{agent_id}] backoff buffer full, dropping");
                                    }
                                }
                                None => {
                                    eprintln!("[Bridge/{agent_id}] reply channel closed");
                                    return;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn serialize_outbound(agent_id: &str, outbound: &BridgeOutbound) -> Result<String, ()> {
    let result = match outbound {
        BridgeOutbound::AgentReply(reply) => {
            serde_json::to_string(&BridgeMsg::AgentReply { message: reply })
        }
        BridgeOutbound::PermissionRequest(request) => {
            serde_json::to_string(&BridgeMsg::PermissionRequest { request })
        }
    };
    result.map_err(|e| {
        eprintln!("[Bridge/{agent_id}] failed to serialize outbound: {e}");
    })
}

async fn handle_inbound(
    agent_id: &str,
    txt: &str,
    push_tx: &tokio::sync::mpsc::Sender<DaemonInbound>,
) {
    match serde_json::from_str::<DaemonMsg>(txt) {
        Ok(dm) => match dm {
            DaemonMsg::RoutedMessage { message } => {
                if push_tx.send(DaemonInbound::RoutedMessage(message)).await.is_err() {
                    eprintln!("[Bridge/{agent_id}] push channel closed");
                }
            }
            DaemonMsg::PermissionVerdict { verdict } => {
                if push_tx.send(DaemonInbound::PermissionVerdict(verdict)).await.is_err() {
                    eprintln!("[Bridge/{agent_id}] push channel closed");
                }
            }
            DaemonMsg::Status { .. } => {}
        },
        Err(e) => {
            eprintln!("[Bridge/{agent_id}] failed to parse daemon msg: {e}");
        }
    }
}
