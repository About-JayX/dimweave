use crate::channel_state::ChannelState;
use crate::mcp_io::{handle_daemon_inbound_checked, tool_call_response, write_line};
use crate::mcp_protocol::{id_to_value, initialize_result, parse_permission_request, RpcMessage};
use crate::types::{BridgeOutbound, DaemonInbound};
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn run(
    agent_id: String,
    role: String,
    mut push_rx: tokio::sync::mpsc::Receiver<DaemonInbound>,
    reply_tx: tokio::sync::mpsc::Sender<BridgeOutbound>,
) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = tokio::io::BufWriter::new(stdout);
    let mut initialized = false;
    let mut channel_state = ChannelState::new();
    let mut pre_init_buffer: Vec<DaemonInbound> = Vec::new();

    loop {
        let mut line = String::new();
        tokio::select! {
            n = reader.read_line(&mut line) => {
                if n.unwrap_or(0usize) == 0 { break; }
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                let Ok(msg) = serde_json::from_str::<RpcMessage>(trimmed) else { continue };
                let was_initialized = initialized;
                if !handle_rpc_message(
                    &agent_id,
                    &role,
                    &mut initialized,
                    &mut channel_state,
                    &mut writer,
                    &reply_tx,
                    msg,
                ).await {
                    eprintln!("[Bridge/{agent_id}] stdout write failed, exiting MCP loop");
                    break;
                }
                // Replay any messages buffered before initialization
                if !was_initialized && initialized {
                    let mut replay_ok = true;
                    for buffered in pre_init_buffer.drain(..) {
                        if !handle_daemon_inbound_checked(
                            &agent_id, &mut channel_state, &mut writer, buffered,
                        ).await {
                            replay_ok = false;
                            break;
                        }
                    }
                    if !replay_ok { break; }
                }
            }
            Some(inbound) = push_rx.recv() => {
                if !initialized {
                    eprintln!("[Bridge/{agent_id}] pre-init: buffering inbound (not yet initialized)");
                    pre_init_buffer.push(inbound);
                    continue;
                }
                if !handle_daemon_inbound_checked(
                    &agent_id, &mut channel_state, &mut writer, inbound,
                ).await {
                    break;
                }
            }
        }
    }
}

/// Returns false if stdout write failed.
async fn handle_rpc_message(
    agent_id: &str,
    role: &str,
    initialized: &mut bool,
    channel_state: &mut ChannelState,
    writer: &mut tokio::io::BufWriter<tokio::io::Stdout>,
    reply_tx: &tokio::sync::mpsc::Sender<BridgeOutbound>,
    msg: RpcMessage,
) -> bool {
    match msg.method.as_deref() {
        Some("initialize") => {
            *initialized = true;
            eprintln!("[Bridge/{agent_id}] MCP initialize complete, role={role}");
            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id_to_value(&msg.id),
                "result": initialize_result(role)
            });
            if !write_line(writer, &resp).await {
                return false;
            }
        }
        Some("tools/list") => {
            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id_to_value(&msg.id),
                "result": { "tools": [crate::tools::reply_tool_schema()] }
            });
            if !write_line(writer, &resp).await {
                return false;
            }
        }
        Some("tools/call") => {
            let resp = tool_call_response(agent_id, channel_state, reply_tx, &msg).await;
            if !write_line(writer, &resp).await {
                return false;
            }
        }
        Some("notifications/claude/channel/permission_request") => {
            if let Some(request) = msg.params.as_ref().and_then(parse_permission_request) {
                eprintln!(
                    "[Bridge/{agent_id}] permission request {} for {}",
                    request.request_id, request.tool_name
                );
                channel_state.register_permission(request.clone());
                let _ = reply_tx
                    .send(BridgeOutbound::PermissionRequest(request))
                    .await;
            }
        }
        Some("notifications/initialized") | None => {}
        _ => {}
    }
    true
}
