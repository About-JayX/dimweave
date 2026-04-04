use crate::types::{BridgeMsg, BridgeOutbound, DaemonInbound, DaemonMsg};
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, warn};

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
            serde_json::to_string(&BridgeMsg::AgentReply { message: reply })
        }
        BridgeOutbound::PermissionRequest(request) => {
            serde_json::to_string(&BridgeMsg::PermissionRequest { request })
        }
        BridgeOutbound::GetOnlineAgents(_) => return Err(()),
    };
    result.map_err(|err| {
        error!(
            agent_id = %agent_id,
            error = %err,
            "failed to serialize outbound message"
        );
    })
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
