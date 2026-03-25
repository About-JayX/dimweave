use crate::channel_state::ChannelState;
use crate::mcp_protocol::id_to_value;
use crate::tools::handle_tool_call;
use crate::types::{BridgeOutbound, DaemonInbound};
use tokio::io::AsyncWriteExt;

/// Handle tool/call and produce a JSON-RPC response.
pub(crate) async fn tool_call_response(
    agent_id: &str,
    channel_state: &mut ChannelState,
    reply_tx: &tokio::sync::mpsc::Sender<BridgeOutbound>,
    msg: &crate::mcp_protocol::RpcMessage,
) -> serde_json::Value {
    match msg
        .params
        .as_ref()
        .and_then(|params| handle_tool_call(params, agent_id))
        .and_then(|bridge_msg| channel_state.rewrite_reply(bridge_msg))
    {
        Some(bridge_msg) => {
            eprintln!(
                "[Bridge/{agent_id}] reply tool -> {} (reply_to={:?})",
                bridge_msg.to, bridge_msg.reply_to
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
        None => serde_json::json!({
            "jsonrpc": "2.0",
            "id": id_to_value(&msg.id),
            "error": { "code": -32000, "message": "unknown chat_id or unsupported tool call" }
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
                    "[Bridge/{agent_id}] permission verdict {} -> {:?}",
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
