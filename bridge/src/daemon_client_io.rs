use crate::types::{BridgeMessage, BridgeMsg, BridgeOutbound, DaemonInbound, DaemonMsg, MessageTarget, ParsedReply};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, warn};

static MSG_SEQ: AtomicU64 = AtomicU64::new(0);

pub type OnlineAgentsReply = tokio::sync::oneshot::Sender<serde_json::Value>;
pub type BridgeSink = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    Message,
>;

pub fn serialize_outbound(agent_id: &str, outbound: &BridgeOutbound) -> Result<String, ()> {
    let result = match outbound {
        BridgeOutbound::AgentReply(reply) => {
            let legacy = to_wire_message(agent_id, reply);
            serde_json::to_string(&BridgeMsg::AgentReply { message: &legacy })
        }
        BridgeOutbound::PermissionRequest(request) => {
            serde_json::to_string(&BridgeMsg::PermissionRequest { request })
        }
        BridgeOutbound::GetOnlineAgents(_) => return Err(()),
    };
    result.map_err(|err| {
        error!(agent_id = %agent_id, error = %err, "failed to serialize outbound message");
    })
}

fn target_label(t: &MessageTarget) -> &str {
    match t {
        MessageTarget::User => "user",
        MessageTarget::Role { role } => role,
        MessageTarget::Agent { agent_id } => agent_id,
    }
}

/// Convert structured `ParsedReply` to legacy `BridgeMessage` at the wire boundary.
/// This temporary shim keeps the daemon wire envelope compatible until Task 7.
fn to_wire_message(from: &str, reply: &ParsedReply) -> BridgeMessage {
    let seq = MSG_SEQ.fetch_add(1, Ordering::Relaxed);
    let ts = chrono::Utc::now().timestamp_millis();
    BridgeMessage {
        id: format!("claude_{ts}_{seq}"), from: from.into(), display_source: None,
        to: target_label(&reply.target).into(), content: reply.content.clone(),
        timestamp: ts as u64, reply_to: None, priority: None,
        status: Some(reply.status), sender_agent_id: None, attachments: None,
    }
}

pub async fn handle_inbound(
    agent_id: &str,
    txt: &str,
    push_tx: &tokio::sync::mpsc::Sender<DaemonInbound>,
    pending_query: &mut Option<OnlineAgentsReply>,
) {
    match serde_json::from_str::<DaemonMsg>(txt) {
        Ok(dm) => match dm {
            DaemonMsg::RoutedMessage { message } => {
                if push_tx
                    .send(DaemonInbound::RoutedMessage(message))
                    .await
                    .is_err()
                {
                    warn!(agent_id = %agent_id, "push channel closed");
                }
            }
            DaemonMsg::PermissionVerdict { verdict } => {
                if push_tx
                    .send(DaemonInbound::PermissionVerdict(verdict))
                    .await
                    .is_err()
                {
                    warn!(agent_id = %agent_id, "push channel closed");
                }
            }
            DaemonMsg::OnlineAgentsResponse { online_agents } => {
                if let Some(tx) = pending_query.take() {
                    let _ = tx.send(online_agents);
                }
            }
            DaemonMsg::Status { .. } => {}
        },
        Err(err) => {
            warn!(
                agent_id = %agent_id,
                error = %err,
                "failed to parse daemon message"
            );
        }
    }
}
