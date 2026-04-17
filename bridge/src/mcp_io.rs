use crate::channel_state::ChannelState;
use crate::mcp_protocol::id_to_value;
use crate::tools::handle_tool_call;
use crate::types::{BridgeOutbound, DaemonInbound, MessageTarget};
use tokio::io::AsyncWriteExt;

/// Handle tool/call and produce a JSON-RPC response.
pub(crate) async fn tool_call_response(
    agent_id: &str,
    reply_tx: &tokio::sync::mpsc::Sender<BridgeOutbound>,
    msg: &crate::mcp_protocol::RpcMessage,
) -> serde_json::Value {
    let params = match msg.params.as_ref() {
        Some(p) => p,
        None => return tool_error(&msg.id, -32000, "unsupported tool call"),
    };
    if crate::tools::is_get_online_agents(params) {
        return handle_get_online_agents(agent_id, reply_tx, &msg.id).await;
    }
    match handle_tool_call(params) {
        Ok(Some(parsed)) => {
            eprintln!("[Bridge/{agent_id}] reply tool → {}", target_label(&parsed.target));
            match reply_tx.send(BridgeOutbound::AgentReply(parsed)).await {
                Ok(()) => tool_ok(&msg.id, "sent"),
                Err(_) => tool_error(&msg.id, -32001, "bridge outbound channel is closed"),
            }
        }
        Err(err) => tool_error(&msg.id, -32002, &err.to_string()),
        Ok(None) => tool_error(&msg.id, -32000, "unsupported tool call"),
    }
}

async fn handle_get_online_agents(
    agent_id: &str,
    reply_tx: &tokio::sync::mpsc::Sender<BridgeOutbound>,
    rpc_id: &Option<crate::mcp_protocol::RpcId>,
) -> serde_json::Value {
    let (tx, rx) = tokio::sync::oneshot::channel();
    if reply_tx
        .send(BridgeOutbound::GetOnlineAgents(tx))
        .await
        .is_err()
    {
        return tool_error(rpc_id, -32001, "bridge outbound channel is closed");
    }
    match tokio::time::timeout(std::time::Duration::from_secs(5), rx).await {
        Ok(Ok(agents)) => {
            let payload = serde_json::json!({ "online_agents": agents });
            let text = serde_json::to_string(&payload).unwrap_or_default();
            eprintln!("[Bridge/{agent_id}] get_online_agents → {text}");
            tool_ok(rpc_id, &text)
        }
        _ => tool_error(rpc_id, -32003, "timeout waiting for online agents"),
    }
}

fn target_label(t: &MessageTarget) -> &str {
    match t {
        MessageTarget::User => "user",
        MessageTarget::Role { role } => role,
        MessageTarget::Agent { agent_id } => agent_id,
    }
}

fn tool_ok(id: &Option<crate::mcp_protocol::RpcId>, text: &str) -> serde_json::Value {
    serde_json::json!({"jsonrpc":"2.0","id":id_to_value(id),"result":{"content":[{"type":"text","text":text}]}})
}

fn tool_error(id: &Option<crate::mcp_protocol::RpcId>, code: i32, msg: &str) -> serde_json::Value {
    serde_json::json!({"jsonrpc":"2.0","id":id_to_value(id),"error":{"code":code,"message":msg}})
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
            if notif.is_some() { eprintln!("[Bridge/{agent_id}] channel event {} from {}", msg.id, msg.source_role()); }
            notif
        }
        DaemonInbound::PermissionVerdict(verdict) => {
            let notif = channel_state.permission_notification(verdict.clone());
            if notif.is_some() { eprintln!("[Bridge/{agent_id}] permission verdict {} → {:?}", verdict.request_id, verdict.behavior); }
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
                    "target": { "kind": "role", "role": "lead" },
                    "message": "hello", "status": "waiting"
                }
            })),
        };
        let response = tool_call_response("claude", &reply_tx, &msg).await;
        assert_eq!(response["error"]["code"], -32002);
        assert!(response["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("Invalid status: \"waiting\""));
    }

    #[tokio::test]
    async fn get_online_agents_returns_json_on_success() {
        let (reply_tx, mut reply_rx) = tokio::sync::mpsc::channel(4);
        let msg = RpcMessage {
            id: Some(RpcId::Number(5)),
            method: Some("tools/call".into()),
            params: Some(serde_json::json!({ "name": "get_online_agents", "arguments": {} })),
        };
        // Spawn a task to respond to the oneshot
        tokio::spawn(async move {
            if let Some(BridgeOutbound::GetOnlineAgents(tx)) = reply_rx.recv().await {
                let _ = tx.send(serde_json::json!([
                    {"agentId": "claude", "role": "lead", "modelSource": "claude"}
                ]));
            }
        });
        let response = tool_call_response("claude", &reply_tx, &msg).await;
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
        assert!(parsed["online_agents"].is_array());
        assert_eq!(parsed["online_agents"][0]["agentId"], "claude");
    }

    #[tokio::test]
    async fn get_online_agents_errors_when_channel_closed() {
        let (reply_tx, reply_rx) = tokio::sync::mpsc::channel(1);
        drop(reply_rx);
        let msg = RpcMessage {
            id: Some(RpcId::Number(6)),
            method: Some("tools/call".into()),
            params: Some(serde_json::json!({ "name": "get_online_agents", "arguments": {} })),
        };
        let response = tool_call_response("claude", &reply_tx, &msg).await;
        assert_eq!(response["error"]["code"], -32001);
    }
}
