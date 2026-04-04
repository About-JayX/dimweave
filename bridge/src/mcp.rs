use crate::channel_state::ChannelState;
use crate::mcp_io::{handle_daemon_inbound_checked, tool_call_response, write_line};
use crate::mcp_protocol::{id_to_value, initialize_result, parse_permission_request, RpcMessage};
use crate::types::{BridgeOutbound, DaemonInbound};
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{error, info, warn};

type BridgeWriter = tokio::io::BufWriter<tokio::io::Stdout>;

struct RpcContext<'a> {
    agent_id: &'a str,
    role: &'a str,
    sdk_mode: bool,
    initialized: &'a mut bool,
    channel_state: &'a mut ChannelState,
    writer: &'a mut BridgeWriter,
    reply_tx: &'a tokio::sync::mpsc::Sender<BridgeOutbound>,
}

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
    let mut writer = BridgeWriter::new(stdout);
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
                let mut rpc = RpcContext {
                    agent_id: &agent_id,
                    role: &role,
                    sdk_mode,
                    initialized: &mut initialized,
                    channel_state: &mut channel_state,
                    writer: &mut writer,
                    reply_tx: &reply_tx,
                };
                if !handle_rpc_message(&mut rpc, msg).await {
                    error!(agent_id = %agent_id, "stdout write failed, exiting MCP loop");
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
                        error!(
                            agent_id = %agent_id,
                            buffered_items = pre_init_buffer.len(),
                            "pre-init replay failed"
                        );
                        break;
                    }
                }
            }
            Some(inbound) = push_rx.recv() => {
                if !initialized {
                    if pre_init_buffer.len() < 128 {
                        pre_init_buffer.push(inbound);
                    } else {
                        warn!(agent_id = %agent_id, "pre-init buffer full, dropping");
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

async fn handle_rpc_message(ctx: &mut RpcContext<'_>, msg: RpcMessage) -> bool {
    match msg.method.as_deref() {
        Some("initialize") => {
            *ctx.initialized = true;
            info!(
                agent_id = %ctx.agent_id,
                role = %ctx.role,
                sdk_mode = ctx.sdk_mode,
                "MCP initialize complete"
            );
            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id_to_value(&msg.id),
                "result": initialize_result(ctx.role, !ctx.sdk_mode)
            });
            if !write_line(ctx.writer, &resp).await {
                return false;
            }
        }
        Some("tools/list") => {
            let resp = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id_to_value(&msg.id),
                "result": { "tools": crate::tools::tool_list() }
            });
            if !write_line(ctx.writer, &resp).await {
                return false;
            }
        }
        Some("tools/call") => {
            let resp = tool_call_response(ctx.agent_id, ctx.reply_tx, &msg).await;
            if !write_line(ctx.writer, &resp).await {
                return false;
            }
        }
        Some("notifications/claude/channel/permission_request") => {
            if let Some(request) = msg.params.as_ref().and_then(parse_permission_request) {
                info!(
                    agent_id = %ctx.agent_id,
                    request_id = %request.request_id,
                    tool_name = %request.tool_name,
                    "permission request received"
                );
                ctx.channel_state.register_permission(request.clone());
                if ctx
                    .reply_tx
                    .send(BridgeOutbound::PermissionRequest(request.clone()))
                    .await
                    .is_err()
                {
                    warn!(
                        agent_id = %ctx.agent_id,
                        request_id = %request.request_id,
                        "daemon channel closed, auto-denying permission"
                    );
                    // Auto-deny so Claude doesn't hang forever
                    if let Some(deny) = ctx.channel_state.permission_notification(
                        crate::types::PermissionVerdict {
                            request_id: request.request_id,
                            behavior: crate::types::PermissionBehavior::Deny,
                        },
                    )
                    {
                        if !write_line(ctx.writer, &deny).await {
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
