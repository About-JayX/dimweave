mod channel_state;
mod daemon_client;
mod daemon_client_io;
mod mcp;
mod mcp_io;
mod mcp_protocol;
mod tools;
mod types;

#[tokio::main]
async fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    let control_port: u16 = std::env::var("DIMWEAVE_DAEMON_PORT")
        .or_else(|_| std::env::var("AGENTBRIDGE_CONTROL_PORT"))
        .unwrap_or_else(|_| "4502".into())
        .parse()
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "invalid AGENTBRIDGE_CONTROL_PORT");
            std::process::exit(1);
        });
    let agent_id = std::env::var("AGENTBRIDGE_AGENT").unwrap_or_else(|_| "claude".into());
    let role_raw = std::env::var("AGENTBRIDGE_ROLE").unwrap_or_else(|_| "lead".into());
    let role = match role_raw.as_str() {
        "user" | "lead" | "coder" => role_raw,
        _ => {
            tracing::warn!(role = %role_raw, "unknown role, defaulting to lead");
            "lead".into()
        }
    };
    let sdk_mode = std::env::var("AGENTBRIDGE_SDK_MODE")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    tracing::info!(
        agent_id = %agent_id,
        control_port,
        role = %role,
        sdk_mode,
        "bridge starting"
    );

    // daemon_client → mcp: push routed messages as Channel notifications
    let (push_tx, push_rx) = tokio::sync::mpsc::channel::<types::DaemonInbound>(64);
    // mcp → daemon_client: send agent_reply / permission_request
    let (reply_tx, reply_rx) = tokio::sync::mpsc::channel::<types::BridgeOutbound>(64);

    let dc = tokio::spawn(daemon_client::run(
        control_port,
        agent_id.clone(),
        push_tx,
        reply_rx,
    ));
    let mcp_task = tokio::spawn(mcp::run(agent_id, role, sdk_mode, push_rx, reply_tx));

    let _ = tokio::join!(dc, mcp_task);
}
