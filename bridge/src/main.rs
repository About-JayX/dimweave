mod channel_state;
mod daemon_client;
mod mcp;
mod mcp_io;
mod mcp_protocol;
mod tools;
mod types;

#[tokio::main]
async fn main() {
    let control_port: u16 = std::env::var("AGENTBRIDGE_CONTROL_PORT")
        .unwrap_or_else(|_| "4502".into())
        .parse()
        .unwrap_or_else(|e| {
            eprintln!("[Bridge] invalid AGENTBRIDGE_CONTROL_PORT: {e}");
            std::process::exit(1);
        });
    let agent_id = std::env::var("AGENTBRIDGE_AGENT").unwrap_or_else(|_| "claude".into());

    eprintln!("[Bridge/{agent_id}] starting, daemon port {control_port}");

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
    let mcp_task = tokio::spawn(mcp::run(agent_id, push_rx, reply_tx));

    let _ = tokio::join!(dc, mcp_task);
}
