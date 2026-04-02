use crate::channel_state::ChannelState;
use crate::mcp_io::{handle_daemon_inbound_checked, tool_call_response, write_line};
use crate::mcp_protocol::{id_to_value, initialize_result, parse_permission_request, RpcMessage};
use crate::types::{BridgeOutbound, DaemonInbound};
use tokio::io::{AsyncBufReadExt, BufReader};

pub async fn run(
    agent_id: String,
    role: String,
    sdk_mode: bool,
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
                    &agent_id, &role, sdk_mode, &mut initialized, &mut channel_state,
                    &mut writer, &reply_tx, msg,
                ).await {
                    eprintln!("[Bridge/{agent_id}] stdout write failed, exiting MCP loop");
                    break;
                }
                if !was_initialized && initialized {
                    let buf = std::mem::take(&mut pre_init_buffer);
                    let mut failed = false;
                    for item in buf {
                        if failed {
                            pre_init_buffer.push(item);
                            continue;
                        }
                        if !handle_daemon_inbound_checked(
                            &agent_id, &mut channel_state, &mut writer, item,
                        ).await {
                            failed = true;
                        }
                    }
                    if failed {
                        eprintln!("[Bridge/{agent_id}] pre-init replay failed, {} items kept", pre_init_buffer.len());
                        break;
                    }
                }
            }
            Some(inbound) = push_rx.recv() => {
                if !initialized {
                    if pre_init_buffer.len() < 128 {
                        pre_init_buffer.push(inbound);
                    } else {
                        eprintln!("[Bridge/{agent_id}] pre-init buffer full, dropping");
                    }
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

async fn handle_rpc_message(
    agent_id: &str,
    role: &str,
    sdk_mode: bool,
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
                "result": initialize_result(role, !sdk_mode)
            });
            if !write_line(writer, &resp).await { return false; }
        }
        Some("tools/list") => {
            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id_to_value(&msg.id),
                "result": { "tools": crate::tools::tool_list() }
            });
            if !write_line(writer, &resp).await { return false; }
        }
        Some("tools/call") => {
            let resp = tool_call_response(agent_id, reply_tx, &msg).await;
            if !write_line(writer, &resp).await { return false; }
        }
        Some("notifications/claude/channel/permission_request") => {
            if let Some(request) = msg.params.as_ref().and_then(parse_permission_request) {
                eprintln!(
                    "[Bridge/{agent_id}] permission request {} for {}",
                    request.request_id, request.tool_name
                );
                channel_state.register_permission(request.clone());
                if reply_tx.send(BridgeOutbound::PermissionRequest(request.clone())).await.is_err() {
                    eprintln!("[Bridge/{agent_id}] daemon channel closed, auto-denying permission {}", request.request_id);
                    // Auto-deny so Claude doesn't hang forever
                    if let Some(deny) = channel_state.permission_notification(
                        crate::types::PermissionVerdict {
                            request_id: request.request_id,
                            behavior: crate::types::PermissionBehavior::Deny,
                        },
                    ) {
                        if !write_line(writer, &deny).await {
                            return false; // stdout dead — exit MCP loop
                        }
                    }
                }
            }
        }
        Some("notifications/initialized") | None => {}
        _ => {}
    }
    true
}
