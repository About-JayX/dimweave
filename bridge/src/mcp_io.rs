use crate::channel_state::ChannelState;
use crate::mcp_protocol::id_to_value;
use crate::tools::handle_tool_call;
use crate::types::{BridgeOutbound, DaemonInbound};
use tokio::io::AsyncWriteExt;

/// Handle tool/call and produce a JSON-RPC response.
pub(crate) async fn tool_call_response(
    agent_id: &str,
    reply_tx: &tokio::sync::mpsc::Sender<BridgeOutbound>,
    msg: &crate::mcp_protocol::RpcMessage,
) -> serde_json::Value {
    match msg
        .params
        .as_ref()
        .map(|params| handle_tool_call(params, agent_id))
    {
        Some(Ok(Some(bridge_msg))) => {
            eprintln!(
                "[Bridge/{agent_id}] reply tool → {}",
                bridge_msg.to
            );
            match reply_tx.send(BridgeOutbound::AgentReply(bridge_msg)).await {
                Ok(()) => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id_to_value(&msg.id),
                    "result": { "content": [{ "type": "text", "text": "sent" }] }
                }),
                Err(_) => serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id_to_value(&msg.id),
                    "error": { "code": -32001, "message": "bridge outbound channel is closed" }
                }),
            }
        }
        Some(Err(err)) => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id_to_value(&msg.id),
            "error": { "code": -32002, "message": err.to_string() }
        }),
        _ => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id_to_value(&msg.id),
            "error": { "code": -32000, "message": "unsupported tool call" }
        }),
    }
}

/// Handle a daemon inbound message. Returns false if stdout write failed.
pub(crate) async fn handle_daemon_inbound_checked(
    agent_id: &str,
    channel_state: &mut ChannelState,
    writer: &mut tokio::io::BufWriter<tokio::io::Stdout>,
    inbound: DaemonInbound,
) -> bool {
    let payload = match inbound {
        DaemonInbound::RoutedMessage(msg) => {
            let notif = channel_state.prepare_channel_message(&msg);
            if notif.is_some() {
                eprintln!(
                    "[Bridge/{agent_id}] channel event {} from {}",
                    msg.id, msg.from
                );
            }
            notif
        }
        DaemonInbound::PermissionVerdict(verdict) => {
            let notif = channel_state.permission_notification(verdict.clone());
            if notif.is_some() {
                eprintln!(
                    "[Bridge/{agent_id}] permission verdict {} → {:?}",
                    verdict.request_id, verdict.behavior
                );
            }
            notif
        }
    };

    if let Some(notif) = payload {
        if !write_line(writer, &notif).await {
            eprintln!("[Bridge/{agent_id}] stdout write failed, exiting MCP loop");
            return false;
        }
    }
    true
}

/// Write a JSON value as a newline-delimited line. Returns false on error.
pub(crate) async fn write_line(
    w: &mut tokio::io::BufWriter<tokio::io::Stdout>,
    val: &serde_json::Value,
) -> bool {
    let Ok(mut line) = serde_json::to_string(val) else {
        return false;
    };
    line.push('\n');
    if w.write_all(line.as_bytes()).await.is_err() {
        return false;
    }
    if w.flush().await.is_err() {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_protocol::{RpcId, RpcMessage};

    #[tokio::test]
    async fn invalid_status_tool_call_returns_explicit_error() {
        let (reply_tx, _reply_rx) = tokio::sync::mpsc::channel(1);
        let msg = RpcMessage {
            id: Some(RpcId::Number(1)),
            method: Some("tools/call".into()),
            params: Some(serde_json::json!({
                "name": "reply",
                "arguments": {
                    "to": "lead",
                    "text": "hello",
                    "status": "waiting"
                }
            })),
        };

        let response = tool_call_response("claude", &reply_tx, &msg).await;
        assert_eq!(response["error"]["code"], -32002);
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap_or_default()
                .contains("Invalid status: \"waiting\"")
        );
    }
}
