use crate::types::{BridgeMessage, BridgeMsg, DaemonMsg};
use futures_util::{SinkExt, StreamExt};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const MAX_RETRIES: u32 = 20;

pub async fn run(
    port: u16,
    agent_id: String,
    push_tx: tokio::sync::mpsc::Sender<BridgeMessage>,
    mut reply_rx: tokio::sync::mpsc::Receiver<BridgeMessage>,
) {
    let url = format!("ws://127.0.0.1:{port}/ws");
    let mut attempt = 0u32;

    loop {
        match connect_async(&url).await {
            Ok((ws, _)) => {
                eprintln!("[Bridge/{agent_id}] connected to daemon");
                attempt = 0;
                let (mut sink, mut stream) = ws.split();

                let connect_msg = serde_json::to_string(&BridgeMsg::AgentConnect {
                    agent_id: &agent_id,
                })
                .unwrap();
                if sink.send(Message::Text(connect_msg.into())).await.is_err() {
                    continue;
                }

                loop {
                    tokio::select! {
                        msg = stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(txt))) => {
                                    if let Ok(dm) = serde_json::from_str::<DaemonMsg>(&txt) {
                                        match dm {
                                            DaemonMsg::RoutedMessage { message } => {
                                                let _ = push_tx.send(message).await;
                                            }
                                            DaemonMsg::Status { .. } => {}
                                        }
                                    }
                                }
                                Some(Ok(_)) => {}
                                _ => break,
                            }
                        }
                        Some(reply) = reply_rx.recv() => {
                            let msg = serde_json::to_string(&BridgeMsg::AgentReply {
                                message: &reply,
                            })
                            .unwrap();
                            if sink.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                    }
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
                sleep(delay).await;
            }
        }
    }
}
