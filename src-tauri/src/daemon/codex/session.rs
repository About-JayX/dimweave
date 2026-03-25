use crate::daemon::{gui, SharedState};
use crate::daemon::codex::handler;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

pub struct SessionOpts {
    pub role_id: String,
    pub cwd: String,
    pub model: Option<String>,
    pub sandbox_mode: Option<String>,
    pub developer_instructions: Option<String>,
}

/// Connect to a running Codex app-server, initialize the session, and enter
/// the event loop.  Messages arriving on `inject_rx` are forwarded as `turn/start`.
pub async fn run(
    port: u16,
    opts: SessionOpts,
    state: SharedState,
    app: AppHandle,
    mut inject_rx: mpsc::Receiver<String>,
) {
    let url = format!("ws://127.0.0.1:{port}");
    let ws = match connect_async(&url).await {
        Ok((ws, _)) => ws,
        Err(e) => {
            gui::emit_system_log(&app, "error", &format!("[Codex] connect failed: {e}"));
            return;
        }
    };

    let (mut sink, mut stream) = ws.split();
    let (ws_tx, mut ws_rx) = mpsc::channel::<String>(64);

    // Outbound writer task
    let tx2 = ws_tx.clone();
    tokio::spawn(async move {
        while let Some(text) = ws_rx.recv().await {
            if sink.send(Message::Text(text.into())).await.is_err() { break; }
        }
        drop(tx2);
    });

    // === Handshake: initialize ===
    let mut next_id: u64 = 1;
    let init_id = next_id; next_id += 1;
    ws_tx.send(json!({
        "method": "initialize", "id": init_id,
        "params": { "clientInfo": {"name":"agentbridge","version":"0.1.0"},
                    "protocolVersion": "0.1.0",
                    "capabilities": {"experimentalApi": true} }
    }).to_string()).await.ok();

    // Wait for init response
    loop {
        let Some(Ok(msg)) = stream.next().await else { return };
        let Ok(v) = serde_json::from_str::<Value>(&msg.to_text().unwrap_or("")) else { continue };
        if v["id"].as_u64() == Some(init_id) { break; }
    }

    // === Handshake: thread/start ===
    let thread_id_rpc = next_id; next_id += 1;
    let mut params = json!({
        "dynamicTools": [
            { "name": "reply", "description": "Send a message to another agent role.",
              "inputSchema": {"type":"object","properties":{"to":{"type":"string"},"text":{"type":"string"}},"required":["to","text"]} },
            { "name": "check_messages", "description": "Check for new messages from other agents.",
              "inputSchema": {"type":"object","properties":{}} },
            { "name": "get_status", "description": "Get AgentBridge status: available roles and online agents.",
              "inputSchema": {"type":"object","properties":{}} }
        ]
    });
    if let Some(cwd) = (!opts.cwd.is_empty()).then(|| opts.cwd.as_str()) {
        params["cwd"] = json!(cwd);
    }
    if let Some(m) = &opts.model { params["model"] = json!(m); }
    if let Some(sb) = &opts.sandbox_mode { params["sandbox"] = json!(sb); }
    if let Some(di) = &opts.developer_instructions {
        if !di.is_empty() { params["settings"] = json!({"developer_instructions": di}); }
    }
    ws_tx.send(json!({"method":"thread/start","id":thread_id_rpc,"params":params}).to_string()).await.ok();

    // Wait for thread/start response
    let mut thread_id = String::new();
    loop {
        let Some(Ok(msg)) = stream.next().await else { return };
        let Ok(v) = serde_json::from_str::<Value>(&msg.to_text().unwrap_or("")) else { continue };
        if v["id"].as_u64() == Some(thread_id_rpc) {
            if let Some(tid) = v["result"]["thread"]["id"].as_str() {
                thread_id = tid.to_string();
                gui::emit_system_log(&app, "info", &format!("[Codex] session started thread={tid}"));
            }
            break;
        }
    }
    if thread_id.is_empty() {
        gui::emit_system_log(&app, "error", "[Codex] failed to start thread");
        return;
    }

    // === Main event loop ===
    let role_id = opts.role_id.clone();
    loop {
        tokio::select! {
            msg_opt = stream.next() => {
                let Some(Ok(msg)) = msg_opt else { break };
                let Ok(v) = serde_json::from_str::<Value>(&msg.to_text().unwrap_or("")) else { continue };
                // Dynamic tool call from Codex
                if v["method"].as_str() == Some("item/tool/call") {
                    if let (Some(id), Some(name)) = (v["id"].as_u64(), v["params"]["name"].as_str()) {
                        let args = v["params"]["arguments"].clone();
                        handler::handle_dynamic_tool(id, name, &args, &role_id, &state, &app, &ws_tx).await;
                    }
                }
            }
            inject = inject_rx.recv() => {
                let Some(text) = inject else { break };
                let id = next_id; next_id += 1;
                ws_tx.send(json!({
                    "method": "turn/start", "id": id,
                    "params": {"threadId": &thread_id, "input": [{"type":"text","text":text}]}
                }).to_string()).await.ok();
            }
        }
    }
    gui::emit_system_log(&app, "info", "[Codex] session ended");
}
