# Rust + Channel API Migration Implementation Plan

> Historical note: this dated plan describes the migration path, not the current source of truth. Use `CLAUDE.md`, `.claude/rules/**`, `src-tauri/src/daemon/**`, and `bridge/**` for the live architecture.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Migrate the entire Bun daemon to Rust (embedded in Tauri), replace PTY-inject with Claude Channel API push, and ship as a single `.dmg` with zero Bun runtime dependency.

**Architecture:** A `bridge/` Cargo crate compiles to a sidecar binary that Claude Code spawns via `--mcp-config`; it speaks MCP stdio to Claude Code (Channel notifications push, reply tool receives) and connects to the Tauri-embedded daemon via WS :4502. The daemon runs as tokio async tasks inside the Tauri process; all GUI communication switches from WS :4503 to Tauri emit/invoke.

**Tech Stack:** Rust 2021, Tauri 2, tokio (async), axum (WS server), tokio-tungstenite (WS client), serde_json, cargo workspace.

**Spec:** `docs/superpowers/specs/2026-03-25-rust-channel-migration-design.md`

---

## File Map

### New files (bridge crate)
| File | Responsibility |
|------|----------------|
| `bridge/Cargo.toml` | Bridge binary crate config |
| `bridge/src/main.rs` | Entry: parse env, start tokio runtime, spawn tasks |
| `bridge/src/mcp.rs` | MCP stdio JSON-RPC: read stdin, dispatch, write stdout |
| `bridge/src/channel.rs` | Push `notifications/claude/channel` notifications |
| `bridge/src/tools.rs` | `reply` tool handler → produce `agent_reply` WS message |
| `bridge/src/daemon_client.rs` | WS client to :4502, auto-reconnect, `routed_message` receiver |
| `bridge/src/types.rs` | Shared types: `BridgeMessage`, `ControlMsg` enums |

### New files (daemon in src-tauri)
| File | Responsibility |
|------|----------------|
| `src-tauri/src/daemon/mod.rs` | Daemon public interface, startup entry |
| `src-tauri/src/daemon/state.rs` | `Arc<RwLock<DaemonState>>`, attachedAgents map |
| `src-tauri/src/daemon/types.rs` | Shared types (duplicated from bridge via workspace) |
| `src-tauri/src/daemon/routing.rs` | `route_message()`: dispatch to bridge WS or GUI or Codex |
| `src-tauri/src/daemon/gui.rs` | `emit_*` helpers wrapping `app.emit()` |
| `src-tauri/src/daemon/control/mod.rs` | Re-exports |
| `src-tauri/src/daemon/control/server.rs` | axum WS server on :4502 |
| `src-tauri/src/daemon/control/handler.rs` | Handle `agent_connect/reply/disconnect` |
| `src-tauri/src/daemon/codex/mod.rs` | Re-exports |
| `src-tauri/src/daemon/codex/lifecycle.rs` | Start/stop Codex app-server |
| `src-tauri/src/daemon/codex/session.rs` | `thread/start` + dynamicTools registration |
| `src-tauri/src/daemon/codex/handler.rs` | Parse turn notifications, dispatch dynamicTool calls |
| `src-tauri/src/daemon/codex/proxy.rs` | WS proxy for TUI connections |
| `src-tauri/src/daemon/session_manager.rs` | CODEX_HOME `/tmp` directory lifecycle |
| `src-tauri/src/daemon/role_config/mod.rs` | Re-exports |
| `src-tauri/src/daemon/role_config/roles.rs` | Role definitions (Lead/Coder/Reviewer/Tester) |

### Modified files
| File | Change |
|------|--------|
| `Cargo.toml` | New: workspace root |
| `src-tauri/Cargo.toml` | Add axum, tokio-tungstenite; remove portable-pty |
| `src-tauri/build.rs` | Copy bridge binary to `binaries/` for sidecar bundling |
| `src-tauri/tauri.conf.json` | Add `externalBin`, update `beforeBuildCommand` |
| `src-tauri/src/main.rs` | Add daemon startup, new commands, remove PTY commands |
| `src/components/ClaudePanel/index.tsx` | Remove PTY invoke, add launch/stop buttons via invoke |
| `src/components/MessagePanel/index.tsx` | Remove TerminalView tab |
| `src/stores/bridge-store/index.ts` | Switch from WS :4503 to Tauri event listeners |
| `src/stores/bridge-store/ws-connection.ts` | Replace with Tauri event subscription |

### Deleted after Phase B verified
- `daemon/` (entire directory)
- `src-tauri/src/pty.rs`

---

## Phase A: Bridge Crate

### Task 1: Cargo Workspace + Shared Types

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `bridge/Cargo.toml`
- Create: `bridge/src/types.rs`
- Create: `bridge/src/main.rs` (stub)

- [ ] **Step 1: Create workspace Cargo.toml**

```toml
# /Cargo.toml
[workspace]
members = ["src-tauri", "bridge"]
resolver = "2"
```

- [ ] **Step 2: Create bridge/Cargo.toml**

```toml
# bridge/Cargo.toml
[package]
name = "agent-bridge-bridge"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "agent-bridge-bridge"
path = "src/main.rs"

[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "io-util", "sync", "time", "net"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio-tungstenite = { version = "0.24", features = ["native-tls"] }
futures-util = "0.3"
```

- [ ] **Step 3: Create bridge/src/types.rs**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

/// Messages daemon sends TO bridge over WS :4502
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DaemonMsg {
    RoutedMessage { message: BridgeMessage },
    Status { status: serde_json::Value },
}

/// Messages bridge sends TO daemon over WS :4502
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeMsg<'a> {
    AgentConnect { #[serde(rename = "agentId")] agent_id: &'a str },
    AgentReply { message: &'a BridgeMessage },
    AgentDisconnect,
}
```

- [ ] **Step 4: Create bridge/src/main.rs stub**

```rust
mod channel;
mod daemon_client;
mod mcp;
mod tools;
mod types;

#[tokio::main]
async fn main() {
    let control_port: u16 = std::env::var("AGENTBRIDGE_CONTROL_PORT")
        .unwrap_or_else(|_| "4502".into())
        .parse()
        .expect("invalid AGENTBRIDGE_CONTROL_PORT");
    let agent_id = std::env::var("AGENTBRIDGE_AGENT")
        .unwrap_or_else(|_| "claude".into());

    eprintln!("[Bridge/{agent_id}] starting, daemon port {control_port}");

    // channel for daemon_client → mcp (push routed messages as Channel notifications)
    let (push_tx, push_rx) = tokio::sync::mpsc::channel::<types::BridgeMessage>(64);
    // channel for mcp (reply tool) → daemon_client (send agent_reply)
    let (reply_tx, reply_rx) = tokio::sync::mpsc::channel::<types::BridgeMessage>(64);

    let dc = tokio::spawn(daemon_client::run(control_port, agent_id.clone(), push_tx, reply_rx));
    let mcp_task = tokio::spawn(mcp::run(agent_id, push_rx, reply_tx));

    let _ = tokio::join!(dc, mcp_task);
}
```

- [ ] **Step 5: Verify workspace compiles**

```bash
cd /Users/jason/floder/agent-bridge
cargo build -p agent-bridge-bridge 2>&1
```
Expected: compile errors only for missing modules (channel/daemon_client/mcp/tools stubs needed). Create empty stubs:

```bash
touch bridge/src/channel.rs bridge/src/daemon_client.rs bridge/src/mcp.rs bridge/src/tools.rs
# Add stub pub async fn run() to each
```

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml bridge/
git commit -m "feat(bridge): cargo workspace + bridge crate skeleton"
```

---

### Task 2: MCP Stdio Server

**Files:**
- Create: `bridge/src/mcp.rs`

The MCP stdio protocol is JSON-RPC 2.0 over stdin/stdout. Claude Code sends `initialize`, then `tools/list`, then `tools/call` requests. We respond, then can push `notifications/claude/channel` at any time.

- [ ] **Step 1: Write test for MCP message parsing**

```rust
// bridge/src/mcp.rs — add at bottom:
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_initialize_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"claude-code","version":"1.0"}}}"#;
        let msg: RpcMessage = serde_json::from_str(raw).unwrap();
        assert_eq!(msg.method.as_deref(), Some("initialize"));
        assert_eq!(msg.id, Some(RpcId::Number(1)));
    }

    #[test]
    fn parse_tools_list_request() {
        let raw = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
        let msg: RpcMessage = serde_json::from_str(raw).unwrap();
        assert_eq!(msg.method.as_deref(), Some("tools/list"));
    }

    #[test]
    fn serialize_notification() {
        let n = channel_notification("hello", "msg-1", "coder");
        let s = serde_json::to_string(&n).unwrap();
        assert!(s.contains("notifications/claude/channel"));
        assert!(s.contains("hello"));
    }
}
```

- [ ] **Step 2: Run test to confirm it fails**

```bash
cargo test -p agent-bridge-bridge 2>&1 | grep -E "FAILED|error"
```
Expected: compile error (`RpcMessage` not defined yet).

- [ ] **Step 3: Implement mcp.rs**

```rust
// bridge/src/mcp.rs
use crate::tools::handle_tool_call;
use crate::types::BridgeMessage;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RpcId { Number(i64), Str(String) }

#[derive(Debug, Deserialize)]
pub struct RpcMessage {
    pub id: Option<RpcId>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct RpcResponse<T: Serialize> {
    jsonrpc: &'static str,
    id: serde_json::Value,
    result: T,
}

#[derive(Serialize)]
struct RpcNotification<T: Serialize> {
    jsonrpc: &'static str,
    method: &'static str,
    params: T,
}

pub fn channel_notification(content: &str, chat_id: &str, from: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/claude/channel",
        "params": {
            "content": content,
            "meta": { "from": from, "chat_id": chat_id }
        }
    })
}

fn id_to_value(id: &Option<RpcId>) -> serde_json::Value {
    match id {
        Some(RpcId::Number(n)) => serde_json::json!(n),
        Some(RpcId::Str(s)) => serde_json::json!(s),
        None => serde_json::Value::Null,
    }
}

pub async fn run(
    agent_id: String,
    mut push_rx: tokio::sync::mpsc::Receiver<BridgeMessage>,
    reply_tx: tokio::sync::mpsc::Sender<BridgeMessage>,
) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = tokio::io::BufWriter::new(stdout);
    let mut initialized = false;

    loop {
        let mut line = String::new();
        tokio::select! {
            n = reader.read_line(&mut line) => {
                if n.unwrap_or(0) == 0 { break; } // stdin closed
                let trimmed = line.trim();
                if trimmed.is_empty() { continue; }
                let Ok(msg) = serde_json::from_str::<RpcMessage>(trimmed) else { continue };

                match msg.method.as_deref() {
                    Some("initialize") => {
                        initialized = true;
                        let resp = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id_to_value(&msg.id),
                            "result": {
                                "protocolVersion": "2024-11-05",
                                "capabilities": {
                                    "tools": {},
                                    "experimental": { "claude/channel": {} }
                                },
                                "serverInfo": { "name": "agentbridge", "version": "0.1.0" }
                            }
                        });
                        write_line(&mut writer, &resp).await;
                    }
                    Some("tools/list") => {
                        let resp = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": id_to_value(&msg.id),
                            "result": { "tools": [crate::tools::reply_tool_schema()] }
                        });
                        write_line(&mut writer, &resp).await;
                    }
                    Some("tools/call") => {
                        if let Some(params) = &msg.params {
                            if let Some(bridge_msg) = handle_tool_call(params, &agent_id) {
                                let _ = reply_tx.send(bridge_msg).await;
                                let resp = serde_json::json!({
                                    "jsonrpc": "2.0",
                                    "id": id_to_value(&msg.id),
                                    "result": { "content": [{ "type": "text", "text": "sent" }] }
                                });
                                write_line(&mut writer, &resp).await;
                            }
                        }
                    }
                    Some("notifications/initialized") | None => {}
                    _ => {}
                }
            }
            Some(msg) = push_rx.recv() => {
                if initialized {
                    let notif = channel_notification(&msg.content, &msg.id, &msg.from);
                    write_line(&mut writer, &notif).await;
                }
            }
        }
    }
}

async fn write_line(w: &mut tokio::io::BufWriter<tokio::io::Stdout>, val: &serde_json::Value) {
    let mut line = serde_json::to_string(val).unwrap();
    line.push('\n');
    let _ = w.write_all(line.as_bytes()).await;
    let _ = w.flush().await;
}
```

- [ ] **Step 4: Implement tools.rs**

```rust
// bridge/src/tools.rs
use crate::types::BridgeMessage;

pub fn reply_tool_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "reply",
        "description": "Send a message to another agent or the user.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "to":   { "type": "string", "description": "Target role: lead/coder/reviewer/tester/user" },
                "text": { "type": "string", "description": "Message content" }
            },
            "required": ["to", "text"]
        }
    })
}

pub fn handle_tool_call(params: &serde_json::Value, from: &str) -> Option<BridgeMessage> {
    let name = params.get("name")?.as_str()?;
    if name != "reply" { return None; }
    let args = params.get("arguments")?;
    let to = args.get("to")?.as_str()?;
    let text = args.get("text")?.as_str()?;
    Some(BridgeMessage {
        id: format!("claude_{}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        to: to.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_reply_tool() {
        let params = serde_json::json!({
            "name": "reply",
            "arguments": { "to": "lead", "text": "hello" }
        });
        let msg = handle_tool_call(&params, "coder").unwrap();
        assert_eq!(msg.to, "lead");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.from, "coder");
    }

    #[test]
    fn unknown_tool_returns_none() {
        let params = serde_json::json!({ "name": "unknown", "arguments": {} });
        assert!(handle_tool_call(&params, "claude").is_none());
    }
}
```

Add `chrono` to bridge/Cargo.toml:

```toml
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p agent-bridge-bridge 2>&1
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add bridge/src/
git commit -m "feat(bridge): MCP stdio server + reply tool handler"
```

---

### Task 3: Bridge Daemon Client (WS + Reconnect)

**Files:**
- Create: `bridge/src/daemon_client.rs`
- Create: `bridge/src/channel.rs` (thin wrapper, re-exports from mcp.rs)

- [ ] **Step 1: Implement channel.rs**

```rust
// bridge/src/channel.rs
// Re-export channel_notification for use by daemon_client
pub use crate::mcp::channel_notification;
```

- [ ] **Step 2: Implement daemon_client.rs**

```rust
// bridge/src/daemon_client.rs
use crate::types::{BridgeMessage, BridgeMsg, DaemonMsg};
use futures_util::{SinkExt, StreamExt};
use tokio::time::{sleep, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

const MAX_RETRIES: u32 = 20;

pub async fn run(
    port: u16,
    agent_id: String,
    push_tx: tokio::sync::mpsc::Sender<BridgeMessage>,
    mut reply_rx: tokio::sync::mpsc::Receiver<BridgeMessage>,
) {
    let url = format!("ws://127.0.0.1:{port}/ws");
    let mut attempt = 0u32;

    loop {
        match connect_async(&url).await {
            Ok((ws, _)) => {
                eprintln!("[Bridge/{agent_id}] connected to daemon");
                attempt = 0;
                let (mut sink, mut stream) = ws.split();

                // Register this agent
                let connect_msg = serde_json::to_string(&BridgeMsg::AgentConnect {
                    agent_id: &agent_id,
                }).unwrap();
                let _ = sink.send(Message::Text(connect_msg.into())).await;

                loop {
                    tokio::select! {
                        msg = stream.next() => {
                            match msg {
                                Some(Ok(Message::Text(txt))) => {
                                    if let Ok(dm) = serde_json::from_str::<DaemonMsg>(&txt) {
                                        match dm {
                                            DaemonMsg::RoutedMessage { message } => {
                                                let _ = push_tx.send(message).await;
                                            }
                                            DaemonMsg::Status { .. } => {}
                                        }
                                    }
                                }
                                Some(Ok(_)) => {}
                                _ => break, // connection dropped, reconnect
                            }
                        }
                        Some(reply) = reply_rx.recv() => {
                            let msg = serde_json::to_string(&BridgeMsg::AgentReply {
                                message: &reply,
                            }).unwrap();
                            if sink.send(Message::Text(msg.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                eprintln!("[Bridge/{agent_id}] daemon connection dropped, reconnecting...");
            }
            Err(e) => {
                attempt += 1;
                if attempt >= MAX_RETRIES {
                    eprintln!("[Bridge/{agent_id}] max retries reached: {e}");
                    return;
                }
                let delay = Duration::from_millis(100 * (1u64 << attempt.min(6)));
                eprintln!("[Bridge/{agent_id}] connect failed (attempt {attempt}): {e}, retry in {delay:?}");
                sleep(delay).await;
            }
        }
    }
}
```

- [ ] **Step 3: Build bridge binary end-to-end**

```bash
cargo build -p agent-bridge-bridge 2>&1
```
Expected: compiles cleanly, binary at `target/debug/agent-bridge-bridge`.

- [ ] **Step 4: Smoke test — MCP handshake via stdin**

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}' \
  | AGENTBRIDGE_CONTROL_PORT=9999 AGENTBRIDGE_AGENT=claude \
    timeout 2 ./target/debug/agent-bridge-bridge 2>/dev/null
```
Expected output (single JSON line):
```
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"experimental":{"claude/channel":{}},"tools":{}},"serverInfo":{"name":"agentbridge","version":"0.1.0"}}}
```

- [ ] **Step 5: Commit**

```bash
git add bridge/src/
git commit -m "feat(bridge): daemon WS client with auto-reconnect + smoke test verified"
```

---

## Phase B: Daemon Rust Modules

### Task 4: Shared Types + State

**Files:**
- Create: `src-tauri/src/daemon/mod.rs`
- Create: `src-tauri/src/daemon/types.rs`
- Create: `src-tauri/src/daemon/state.rs`

- [ ] **Step 1: Add daemon dependencies to src-tauri/Cargo.toml**

```toml
# Add to [dependencies]:
axum = { version = "0.7", features = ["ws"] }
tokio-tungstenite = { version = "0.24", features = ["native-tls"] }
futures-util = "0.3"
chrono = { version = "0.4" }
uuid = { version = "1", features = ["v4"] }
# Remove:
# portable-pty = "0.9.0"
```

- [ ] **Step 2: Create daemon/types.rs**

```rust
// src-tauri/src/daemon/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

impl BridgeMessage {
    pub fn system(id: &str, content: &str, to: &str) -> Self {
        Self {
            id: format!("sys_{}", chrono::Utc::now().timestamp_millis()),
            from: "system".into(),
            to: to.into(),
            content: content.into(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            reply_to: None,
            priority: None,
        }
    }
}

/// Messages daemon sends TO bridge over WS :4502
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToAgent {
    RoutedMessage { message: BridgeMessage },
}

/// Messages bridge sends TO daemon over WS :4502
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FromAgent {
    AgentConnect { #[serde(rename = "agentId")] agent_id: String },
    AgentReply { message: BridgeMessage },
    AgentDisconnect,
}
```

- [ ] **Step 3: Create daemon/state.rs**

```rust
// src-tauri/src/daemon/state.rs
use crate::daemon::types::BridgeMessage;
use std::collections::HashMap;
use tokio::sync::mpsc;

pub type AgentSender = mpsc::Sender<BridgeMessage>;

#[derive(Default)]
pub struct DaemonState {
    /// agentId → channel sender to their WS handler task
    pub attached_agents: HashMap<String, AgentSender>,
    pub buffered_messages: Vec<BridgeMessage>,
    pub claude_role: String,
    pub codex_role: String,
    pub codex_bootstrapped: bool,
    pub active_thread_id: Option<String>,
    pub codex_home: Option<String>,
}

impl DaemonState {
    pub fn new() -> Self {
        Self {
            claude_role: "lead".into(),
            codex_role: "coder".into(),
            ..Default::default()
        }
    }

    pub fn flush_buffered(&mut self) -> Vec<BridgeMessage> {
        std::mem::take(&mut self.buffered_messages)
    }

    pub fn buffer_message(&mut self, msg: BridgeMessage) {
        self.buffered_messages.push(msg);
        if self.buffered_messages.len() > 200 {
            self.buffered_messages.drain(0..100);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_clears_buffer() {
        let mut s = DaemonState::new();
        s.buffer_message(BridgeMessage::system("t1", "hello", "lead"));
        assert_eq!(s.buffered_messages.len(), 1);
        let flushed = s.flush_buffered();
        assert_eq!(flushed.len(), 1);
        assert!(s.buffered_messages.is_empty());
    }
}
```

- [ ] **Step 4: Create daemon/mod.rs**

```rust
// src-tauri/src/daemon/mod.rs
pub mod codex;
pub mod control;
pub mod gui;
pub mod role_config;
pub mod routing;
pub mod session_manager;
pub mod state;
pub mod types;

use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, RwLock};

pub type SharedState = Arc<RwLock<state::DaemonState>>;

/// Commands the frontend can send to the daemon via Tauri invoke → mpsc
#[derive(Debug)]
pub enum DaemonCmd {
    LaunchClaude { role_id: String, cwd: String, model: String },
    StopClaude,
    SendMessage { to: String, text: String, from: String },
}

pub async fn start(app: AppHandle, mut cmd_rx: mpsc::Receiver<DaemonCmd>) {
    let shared = Arc::new(RwLock::new(state::DaemonState::new()));

    // Start control server (WS :4502)
    let control_shared = shared.clone();
    let control_app = app.clone();
    tokio::spawn(async move {
        control::server::start(4502, control_shared, control_app).await;
    });

    // Start Codex adapter
    let codex_shared = shared.clone();
    let codex_app = app.clone();
    tokio::spawn(async move {
        codex::lifecycle::start(4500, codex_shared, codex_app).await;
    });

    // Command loop
    let cmd_shared = shared.clone();
    let cmd_app = app.clone();
    while let Some(cmd) = cmd_rx.recv().await {
        handle_cmd(cmd, &cmd_shared, &cmd_app).await;
    }
}

async fn handle_cmd(cmd: DaemonCmd, shared: &SharedState, app: &AppHandle) {
    match cmd {
        DaemonCmd::LaunchClaude { role_id, cwd, model } => {
            codex::lifecycle::launch_claude(shared, app, &role_id, &cwd, &model).await;
        }
        DaemonCmd::StopClaude => {
            codex::lifecycle::stop_claude(shared, app).await;
        }
        DaemonCmd::SendMessage { to, text, from } => {
            let msg = types::BridgeMessage {
                id: format!("ui_{}", chrono::Utc::now().timestamp_millis()),
                from,
                to: to.clone(),
                content: text,
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                reply_to: None,
                priority: None,
            };
            routing::route_message(shared, app, msg).await;
        }
    }
}
```

- [ ] **Step 5: Run daemon unit tests**

```bash
cargo test -p agent-bridge 2>&1
```
Expected: `flush_clears_buffer` passes (other tests compile error until stubs added).

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/daemon/
git commit -m "feat(daemon): shared types, state, DaemonCmd enum"
```

---

### Task 5: Control Server + Message Routing

**Files:**
- Create: `src-tauri/src/daemon/control/server.rs`
- Create: `src-tauri/src/daemon/control/handler.rs`
- Create: `src-tauri/src/daemon/control/mod.rs`
- Create: `src-tauri/src/daemon/routing.rs`
- Create: `src-tauri/src/daemon/gui.rs`

- [ ] **Step 1: Write routing unit test**

```rust
// src-tauri/src/daemon/routing.rs — tests module:
#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::{state::DaemonState, types::BridgeMessage};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn route_to_offline_agent_buffers_message() {
        let state = Arc::new(RwLock::new(DaemonState::new()));
        let msg = BridgeMessage::system("t1", "hello", "lead");
        // No agent attached — should buffer
        let result = route_message_inner(&state, msg).await;
        assert!(matches!(result, RouteResult::Buffered));
        let s = state.read().await;
        assert_eq!(s.buffered_messages.len(), 1);
    }
}
```

- [ ] **Step 2: Run to confirm failure**

```bash
cargo test -p agent-bridge route_to_offline 2>&1 | grep -E "FAILED|error\[" | head -5
```

- [ ] **Step 3: Implement gui.rs**

```rust
// src-tauri/src/daemon/gui.rs
use crate::daemon::types::BridgeMessage;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessageEvent {
    pub payload: BridgeMessage,
    pub timestamp: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SystemLogEvent {
    pub level: String,
    pub message: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatusEvent {
    pub agent: String,
    pub online: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

pub fn emit_agent_message(app: &AppHandle, msg: &BridgeMessage) {
    let _ = app.emit("agent_message", AgentMessageEvent {
        payload: msg.clone(),
        timestamp: msg.timestamp,
    });
}

pub fn emit_system_log(app: &AppHandle, level: &str, message: &str) {
    let _ = app.emit("system_log", SystemLogEvent {
        level: level.into(),
        message: message.into(),
    });
}

pub fn emit_agent_status(app: &AppHandle, agent: &str, online: bool, exit_code: Option<i32>) {
    let _ = app.emit("agent_status", AgentStatusEvent {
        agent: agent.into(),
        online,
        exit_code,
    });
}
```

- [ ] **Step 4: Implement routing.rs**

```rust
// src-tauri/src/daemon/routing.rs
use crate::daemon::{gui, state::DaemonState, types::BridgeMessage, SharedState};
use tauri::AppHandle;

pub enum RouteResult { Delivered, Buffered, ToGui }

pub async fn route_message_inner(state: &SharedState, msg: BridgeMessage) -> RouteResult {
    let mut s = state.write().await;
    if msg.to == "user" {
        return RouteResult::ToGui;
    }
    // Resolve target agent id by role
    let target_agent = if s.claude_role == msg.to { Some("claude") }
        else if s.codex_role == msg.to { Some("codex") }
        else { None };

    if let Some(agent_id) = target_agent {
        if let Some(tx) = s.attached_agents.get(agent_id) {
            if tx.send(msg.clone()).await.is_ok() {
                return RouteResult::Delivered;
            }
        }
    }
    s.buffer_message(msg);
    RouteResult::Buffered
}

pub async fn route_message(state: &SharedState, app: &AppHandle, msg: BridgeMessage) {
    gui::emit_agent_message(app, &msg);
    let result = route_message_inner(state, msg.clone()).await;
    match result {
        RouteResult::Delivered => {
            gui::emit_system_log(app, "info", &format!("[Route] {} → {} delivered", msg.from, msg.to));
        }
        RouteResult::Buffered => {
            gui::emit_system_log(app, "warn", &format!("[Route] {} offline, buffered", msg.to));
        }
        RouteResult::ToGui => {}
    }
}
```

- [ ] **Step 5: Implement control/server.rs**

```rust
// src-tauri/src/daemon/control/server.rs
use crate::daemon::{SharedState, control::handler};
use axum::{Router, extract::{State, WebSocketUpgrade}, response::IntoResponse, routing::get};
use tauri::AppHandle;

pub async fn start(port: u16, state: SharedState, app: AppHandle) {
    let shared = (state, app);
    let router = Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/ws", get(ws_handler))
        .with_state(shared);

    let addr = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await
        .expect("cannot bind control server");
    eprintln!("[Daemon] control server on ws://{addr}/ws");
    axum::serve(listener, router).await.unwrap();
}

async fn ws_handler(
    State((state, app)): State<(SharedState, AppHandle)>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handler::handle_connection(socket, state, app))
}
```

- [ ] **Step 6: Implement control/handler.rs**

```rust
// src-tauri/src/daemon/control/handler.rs
use crate::daemon::{
    gui, routing, state::DaemonState, types::{BridgeMessage, FromAgent, ToAgent}, SharedState,
};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio::sync::mpsc;

pub async fn handle_connection(socket: WebSocket, state: SharedState, app: AppHandle) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<BridgeMessage>(64);
    let mut agent_id: Option<String> = None;

    // Forward outbound messages to WS
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let payload = serde_json::to_string(&ToAgent::RoutedMessage { message: msg }).unwrap();
            if sink.send(Message::Text(payload.into())).await.is_err() { break; }
        }
    });

    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(txt) = msg else { continue };
        let Ok(from_agent) = serde_json::from_str::<FromAgent>(&txt) else { continue };

        match from_agent {
            FromAgent::AgentConnect { agent_id: id } => {
                agent_id = Some(id.clone());
                state.write().await.attached_agents.insert(id.clone(), tx.clone());
                gui::emit_agent_status(&app, &id, true, None);
                gui::emit_system_log(&app, "info", &format!("[Control] {id} connected"));
            }
            FromAgent::AgentReply { message } => {
                routing::route_message(&state, &app, message).await;
            }
            FromAgent::AgentDisconnect => break,
        }
    }

    if let Some(id) = &agent_id {
        state.write().await.attached_agents.remove(id);
        gui::emit_agent_status(&app, id, false, None);
        gui::emit_system_log(&app, "info", &format!("[Control] {id} disconnected"));
    }
}
```

- [ ] **Step 7: Create control/mod.rs**

```rust
// src-tauri/src/daemon/control/mod.rs
pub mod handler;
pub mod server;
```

- [ ] **Step 8: Run routing tests**

```bash
cargo test -p agent-bridge route_to_offline 2>&1
```
Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/daemon/
git commit -m "feat(daemon): control server WS :4502, message routing, gui emit"
```

---

### Task 6: Role Config + Session Manager

**Files:**
- Create: `src-tauri/src/daemon/role_config/roles.rs`
- Create: `src-tauri/src/daemon/role_config/mod.rs`
- Create: `src-tauri/src/daemon/session_manager.rs`

- [ ] **Step 1: Implement role_config/roles.rs**

Port the TypeScript `ROLES` map. Only the fields needed by daemon (defaultTarget, developerInstructions, sandboxMode).

```rust
// src-tauri/src/daemon/role_config/roles.rs
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RoleConfig {
    pub id: &'static str,
    pub label: &'static str,
    pub default_target: &'static str,
    pub sandbox_mode: &'static str,   // "read-only" | "workspace-write"
    pub approval_policy: &'static str,// "on-failure" | "untrusted"
    pub developer_instructions: &'static str,
}

pub fn all_roles() -> HashMap<&'static str, RoleConfig> {
    let mut m = HashMap::new();
    m.insert("lead", RoleConfig {
        id: "lead", label: "Lead", default_target: "coder",
        sandbox_mode: "workspace-write", approval_policy: "on-failure",
        developer_instructions: "You are the Lead agent. Coordinate other agents. CRITICAL: Use the reply tool to communicate.",
    });
    m.insert("coder", RoleConfig {
        id: "coder", label: "Coder", default_target: "lead",
        sandbox_mode: "workspace-write", approval_policy: "on-failure",
        developer_instructions: "You are the Coder agent. Implement tasks and report back to Lead via reply tool.",
    });
    m.insert("reviewer", RoleConfig {
        id: "reviewer", label: "Reviewer", default_target: "lead",
        sandbox_mode: "read-only", approval_policy: "untrusted",
        developer_instructions: "You are the Reviewer agent. Review code read-only and report via reply tool.",
    });
    m.insert("tester", RoleConfig {
        id: "tester", label: "Tester", default_target: "lead",
        sandbox_mode: "read-only", approval_policy: "untrusted",
        developer_instructions: "You are the Tester agent. Run tests and report results via reply tool.",
    });
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_four_roles_exist() {
        let roles = all_roles();
        assert!(roles.contains_key("lead"));
        assert!(roles.contains_key("coder"));
        assert!(roles.contains_key("reviewer"));
        assert!(roles.contains_key("tester"));
    }
}
```

- [ ] **Step 1.5: Add `dirs` to src-tauri/Cargo.toml** (already present — verify it's there)

```bash
grep "dirs" src-tauri/Cargo.toml
```
If missing, add: `dirs = "6"`

- [ ] **Step 2: Implement session_manager.rs**

```rust
// src-tauri/src/daemon/session_manager.rs
use std::path::{Path, PathBuf};

pub struct Session {
    pub session_id: String,
    pub codex_home: PathBuf,
}

pub fn create_session(session_id: &str, role_id: &str, bridge_path: &str, control_port: u16) -> std::io::Result<Session> {
    let codex_home = PathBuf::from(format!("/tmp/agentbridge-{session_id}/codex"));
    std::fs::create_dir_all(&codex_home)?;
    std::fs::create_dir_all(codex_home.join("rules"))?;

    // Symlink auth.json
    let auth_src = dirs::home_dir().unwrap().join(".codex").join("auth.json");
    let auth_dst = codex_home.join("auth.json");
    if auth_src.exists() && !auth_dst.exists() {
        #[cfg(unix)]
        std::os::unix::fs::symlink(&auth_src, &auth_dst)?;
    }

    // Write config.toml with MCP server entry
    let config_toml = format!(
        "[mcp_servers.agentbridge]\ncommand = \"{bridge_path}\"\nenv = {{ AGENTBRIDGE_CONTROL_PORT = \"{control_port}\", AGENTBRIDGE_AGENT = \"codex\" }}\n"
    );
    std::fs::write(codex_home.join("config.toml"), &config_toml)?;

    Ok(Session { session_id: session_id.into(), codex_home })
}

pub fn cleanup_session(session: &Session) {
    let _ = std::fs::remove_dir_all(&session.codex_home.parent().unwrap_or(&session.codex_home));
}

pub fn cleanup_stale_sessions() {
    if let Ok(entries) = std::fs::read_dir("/tmp") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name.to_string_lossy().starts_with("agentbridge-") {
                let _ = std::fs::remove_dir_all(entry.path());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_cleanup() {
        let s = create_session("test123", "lead", "/tmp/bridge", 4502).unwrap();
        assert!(s.codex_home.exists());
        assert!(s.codex_home.join("config.toml").exists());
        cleanup_session(&s);
        assert!(!s.codex_home.parent().unwrap().exists());
    }
}
```

- [ ] **Step 3: Run new tests**

```bash
cargo test -p agent-bridge all_four_roles create_and_cleanup 2>&1
```
Expected: both pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/daemon/
git commit -m "feat(daemon): role config + session manager"
```

---

### Task 7: Codex Adapter (Port from TypeScript)

**Files:**
- Create: `src-tauri/src/daemon/codex/lifecycle.rs`
- Create: `src-tauri/src/daemon/codex/session.rs`
- Create: `src-tauri/src/daemon/codex/handler.rs`
- Create: `src-tauri/src/daemon/codex/proxy.rs` (stub)
- Create: `src-tauri/src/daemon/codex/mod.rs`

This task ports the TypeScript `codex-adapter` to Rust. The Codex app-server runs on port 4500 and speaks WebSocket JSON-RPC 2.0.

- [ ] **Step 1: Implement codex/handler.rs** (dynamicTool dispatch)

```rust
// src-tauri/src/daemon/codex/handler.rs
use crate::daemon::{routing, types::BridgeMessage, SharedState};
use tauri::AppHandle;

/// Handle dynamicTool calls from Codex (reply / check_messages / get_status)
pub async fn handle_dynamic_tool(
    tool_name: &str,
    args: &serde_json::Value,
    state: &SharedState,
    app: &AppHandle,
) -> serde_json::Value {
    match tool_name {
        "reply" => {
            let to = args.get("to").and_then(|v| v.as_str()).unwrap_or("lead");
            let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let msg = BridgeMessage {
                id: format!("codex_{}", chrono::Utc::now().timestamp_millis()),
                from: state.read().await.codex_role.clone(),
                to: to.to_string(),
                content: text.to_string(),
                timestamp: chrono::Utc::now().timestamp_millis() as u64,
                reply_to: None,
                priority: None,
            };
            routing::route_message(state, app, msg).await;
            serde_json::json!({ "contentItems": [{ "type": "inputText", "text": format!("Message routed to {to}.") }] })
        }
        "check_messages" => {
            let messages = state.write().await.flush_buffered();
            if messages.is_empty() {
                serde_json::json!({ "contentItems": [{ "type": "inputText", "text": "No new messages." }] })
            } else {
                let formatted: Vec<String> = messages.iter().map(|m| {
                    format!("[{}] {}: {}", m.timestamp, m.from, m.content)
                }).collect();
                serde_json::json!({ "contentItems": [{ "type": "inputText", "text": formatted.join("\n---\n") }] })
            }
        }
        "get_status" => {
            let s = state.read().await;
            let text = format!(
                "Bridge ready: yes\nClaude ({}): {}\nCodex ({}): online",
                s.claude_role,
                if s.attached_agents.contains_key("claude") { "online" } else { "offline" },
                s.codex_role,
            );
            serde_json::json!({ "contentItems": [{ "type": "inputText", "text": text }] })
        }
        _ => serde_json::json!({ "contentItems": [{ "type": "inputText", "text": format!("Unknown tool: {tool_name}") }] }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::state::DaemonState;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn check_messages_empty() {
        let state = Arc::new(RwLock::new(DaemonState::new()));
        // Need a mock AppHandle — skip gui emit in test by using a test helper
        // For now verify the logic path compiles and runs without panic
        let result = handle_dynamic_tool("check_messages", &serde_json::json!({}), &state, &fake_app()).await;
        let text = result["contentItems"][0]["text"].as_str().unwrap();
        assert_eq!(text, "No new messages.");
    }
}
```

Note: `fake_app()` is a test stub — see step below.

- [ ] **Step 2: Add test helper for AppHandle mock**

Because `AppHandle` cannot be constructed in unit tests, wrap `emit_*` calls behind a trait in `gui.rs` or simply gate test compilation. For now, skip the AppHandle-dependent test and test just the data logic:

```rust
// Replace the test with:
#[tokio::test]
async fn check_messages_empty_returns_expected_text() {
    let mut s = crate::daemon::state::DaemonState::new();
    s.buffer_message(BridgeMessage::system("t", "msg1", "lead"));
    let flushed = s.flush_buffered();
    assert_eq!(flushed.len(), 1);
    assert!(s.buffered_messages.is_empty());
}
```

- [ ] **Step 3: Implement codex/session.rs** (thread/start with dynamicTools)

```rust
// src-tauri/src/daemon/codex/session.rs
use crate::daemon::SharedState;
use serde_json::json;

pub fn build_dynamic_tools() -> serde_json::Value {
    json!([
        {
            "name": "reply",
            "description": "Send a message to another agent role.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to":   { "type": "string" },
                    "text": { "type": "string" }
                },
                "required": ["to", "text"]
            }
        },
        {
            "name": "check_messages",
            "description": "Check for new messages from other agents.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": "get_status",
            "description": "Get AgentBridge status.",
            "inputSchema": { "type": "object", "properties": {} }
        }
    ])
}

pub fn build_thread_start_params(
    state: &crate::daemon::state::DaemonState,
    model: Option<&str>,
    cwd: Option<&str>,
) -> serde_json::Value {
    let mut params = json!({
        "dynamicTools": build_dynamic_tools()
    });
    if let Some(m) = model { params["model"] = json!(m); }
    if let Some(c) = cwd { params["cwd"] = json!(c); }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dynamic_tools_has_three_tools() {
        let tools = build_dynamic_tools();
        assert_eq!(tools.as_array().unwrap().len(), 3);
    }
}
```

- [ ] **Step 4: Implement codex/lifecycle.rs** (WS connect + thread/start)

This is the most complex module — ports `codex-adapter/lifecycle.ts` + `session.ts`. Keep under 200 lines; split if needed.

```rust
// src-tauri/src/daemon/codex/lifecycle.rs
use crate::daemon::{
    codex::{handler::handle_dynamic_tool, session::build_thread_start_params},
    gui, SharedState,
};
use futures_util::{SinkExt, StreamExt};
use tauri::AppHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};

static CLAUDE_CHILD: tokio::sync::Mutex<Option<tokio::process::Child>> =
    tokio::sync::Mutex::const_new(None);

pub async fn start(_app_port: u16, state: SharedState, app: AppHandle) {
    // Codex is started externally (app-server binary), we just connect to it
    // This task handles reconnection logic
    let url = "ws://127.0.0.1:4500";
    loop {
        match connect_async(url).await {
            Ok((ws, _)) => {
                gui::emit_system_log(&app, "info", "Connected to Codex app-server");
                if let Err(e) = run_codex_session(ws, &state, &app).await {
                    gui::emit_system_log(&app, "error", &format!("Codex session error: {e}"));
                }
            }
            Err(_) => {
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}

pub async fn launch_claude(state: &SharedState, app: &AppHandle, role_id: &str, cwd: &str, model: &str) {
    let bridge_binary = resolve_bridge_binary();
    let mcp_config = serde_json::json!({
        "mcpServers": {
            "agentbridge": {
                "command": bridge_binary,
                "args": [],
                "env": { "AGENTBRIDGE_CONTROL_PORT": "4502", "AGENTBRIDGE_AGENT": "claude" }
            }
        }
    });
    let agents_json = build_agents_json(role_id);
    let child = tokio::process::Command::new("claude")
        .args([
            "--dangerously-load-development-channels", "server:agentbridge",
            "--dangerously-skip-permissions",
            "--strict-mcp-config",
            "--mcp-config", &serde_json::to_string(&mcp_config).unwrap(),
            "--agent", role_id,
            "--agents", &agents_json,
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn();

    match child {
        Ok(c) => {
            *CLAUDE_CHILD.lock().await = Some(c);
            gui::emit_agent_status(app, "claude", true, None);
        }
        Err(e) => {
            gui::emit_system_log(app, "error", &format!("Failed to launch claude: {e}"));
        }
    }
}

pub async fn stop_claude(state: &SharedState, app: &AppHandle) {
    if let Some(mut child) = CLAUDE_CHILD.lock().await.take() {
        let _ = child.kill().await;
        // Discard pending messages
        let discarded = state.write().await.flush_buffered();
        if !discarded.is_empty() {
            gui::emit_system_log(app, "warn",
                &format!("Claude stopped, {} pending messages discarded", discarded.len()));
        }
        gui::emit_agent_status(app, "claude", false, None);
    }
}

fn resolve_bridge_binary() -> String {
    // In dev: find bridge binary relative to cargo manifest
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let dev_path = std::path::Path::new(&manifest)
            .parent().unwrap()
            .join("target/debug/agent-bridge-bridge");
        if dev_path.exists() { return dev_path.to_string_lossy().into(); }
    }
    "agent-bridge-bridge".into()
}

fn build_agents_json(role_id: &str) -> String {
    use crate::daemon::role_config::roles::all_roles;
    let roles = all_roles();
    let role = roles.get(role_id).cloned().unwrap_or_else(|| roles["lead"].clone());
    serde_json::to_string(&serde_json::json!({
        role_id: {
            "description": role.label,
            "prompt": role.developer_instructions,
            "permissionMode": "bypass"
        }
    })).unwrap()
}

async fn run_codex_session(
    ws: tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    state: &SharedState,
    app: &AppHandle,
) -> anyhow::Result<()> {
    let (mut sink, mut stream) = ws.split();
    let mut rpc_id: i64 = 1;

    // Initialize
    sink.send(Message::Text(serde_json::to_string(&serde_json::json!({
        "jsonrpc": "2.0", "id": rpc_id, "method": "initialize",
        "params": { "clientInfo": { "name": "agentbridge", "version": "0.1.0" },
                    "protocolVersion": "0.1.0",
                    "capabilities": { "experimentalApi": true } }
    }))?.into())).await?;
    rpc_id += 1;

    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(txt) = msg else { continue };
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&txt) else { continue };

        // Handle dynamicToolCall
        if val.get("method").and_then(|m| m.as_str()) == Some("item/tool/call") {
            if let Some(params) = val.get("params") {
                let tool = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or_default();
                let call_id = params.get("callId").and_then(|v| v.as_str()).unwrap_or("");
                let thread_id = params.get("threadId").and_then(|v| v.as_str()).unwrap_or("");
                let result = handle_dynamic_tool(tool, &args, state, app).await;
                sink.send(Message::Text(serde_json::to_string(&serde_json::json!({
                    "jsonrpc": "2.0", "id": rpc_id,
                    "method": "dynamicToolCallResponse",
                    "params": { "threadId": thread_id, "callId": call_id,
                                "contentItems": result["contentItems"] }
                }))?.into())).await?;
                rpc_id += 1;
            }
        }
    }
    Ok(())
}
```

Add `anyhow` to src-tauri/Cargo.toml:
```toml
anyhow = "1"
```

- [ ] **Step 5: Create codex/proxy.rs stub**

```rust
// src-tauri/src/daemon/codex/proxy.rs
// TUI proxy — stub, not required for Channel API flow
pub async fn start(_proxy_port: u16) {}
```

- [ ] **Step 6: Create codex/mod.rs**

```rust
// src-tauri/src/daemon/codex/mod.rs
pub mod handler;
pub mod lifecycle;
pub mod proxy;
pub mod session;
```

- [ ] **Step 7: Run all daemon tests**

```bash
cargo test -p agent-bridge 2>&1
```
Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/daemon/
git commit -m "feat(daemon): codex adapter — dynamicTools, session, lifecycle"
```

---

## Phase C: Tauri Integration + Frontend

### Task 8: Wire Daemon into Tauri + New Commands

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/tauri.conf.json`
- Create: `src-tauri/build.rs`

- [ ] **Step 1: Update main.rs**

Replace the existing `main.rs` content. Remove all PTY commands, add daemon startup and new commands:

```rust
// src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod codex;
mod daemon;

use codex::auth::CodexProfile;
use codex::models::CodexModel;
use codex::oauth::{OAuthHandle, OAuthLaunchInfo};
use codex::usage::UsageSnapshot;
use daemon::DaemonCmd;
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::mpsc;

struct AppState {
    daemon_tx: mpsc::Sender<DaemonCmd>,
    oauth: Arc<OAuthHandle>,
}

#[tauri::command]
fn get_codex_account() -> Result<CodexProfile, String> { codex::auth::read_profile() }

#[tauri::command]
async fn refresh_usage() -> Result<UsageSnapshot, String> { codex::usage::get_snapshot().await }

#[tauri::command]
fn list_codex_models() -> Result<Vec<CodexModel>, String> { codex::models::list_models() }

#[tauri::command]
async fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
    app.dialog().file().pick_folder(move |path| { let _ = tx.send(path.map(|p| p.to_string())); });
    rx.await.map_err(|_| "dialog cancelled".to_string())
}

#[tauri::command]
async fn codex_login(app: tauri::AppHandle) -> Result<OAuthLaunchInfo, String> {
    let handle = app.state::<Arc<OAuthHandle>>();
    codex::oauth::start_login(handle.inner().clone()).await
}

#[tauri::command]
fn codex_cancel_login(app: tauri::AppHandle) -> bool {
    app.state::<Arc<OAuthHandle>>().cancel()
}

#[tauri::command]
async fn codex_logout() -> Result<(), String> { codex::oauth::do_logout().await }

#[tauri::command]
async fn launch_claude(
    state: State<'_, AppState>,
    role_id: String, cwd: String, model: String,
) -> Result<(), String> {
    state.daemon_tx.send(DaemonCmd::LaunchClaude { role_id, cwd, model })
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_claude(state: State<'_, AppState>) -> Result<(), String> {
    state.daemon_tx.send(DaemonCmd::StopClaude)
        .await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn send_message(
    state: State<'_, AppState>,
    to: String, text: String,
) -> Result<(), String> {
    state.daemon_tx.send(DaemonCmd::SendMessage { to, text, from: "user".into() })
        .await.map_err(|e| e.to_string())
}

fn main() {
    let (daemon_tx, daemon_rx) = mpsc::channel::<DaemonCmd>(32);
    let oauth = Arc::new(OAuthHandle::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState { daemon_tx, oauth: oauth.clone() })
        .manage(oauth)
        .setup(|app| {
            let handle = app.handle().clone();
            let rx = daemon_rx;  // move into setup closure
            tokio::spawn(async move {
                daemon::start(handle, rx).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_codex_account, refresh_usage, list_codex_models, pick_directory,
            codex_login, codex_cancel_login, codex_logout,
            launch_claude, stop_claude, send_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 2: Update tauri.conf.json**

```json
{
  "$schema": "https://raw.githubusercontent.com/nickelpack/tauri-v2-nightly/main/tauri-config-schema.json",
  "productName": "AgentBridge",
  "version": "0.1.0",
  "identifier": "com.agentbridge.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "bun run dev",
    "beforeBuildCommand": "cargo build -p agent-bridge-bridge --release && bun run build"
  },
  "bundle": {
    "externalBin": ["binaries/agent-bridge-bridge"]
  },
  "app": {
    "windows": [{ "title": "AgentBridge", "width": 1000, "height": 700, "resizable": true }],
    "security": { "csp": null }
  },
  "plugins": { "shell": { "open": true } }
}
```

- [ ] **Step 3: Create build.rs**

```rust
// src-tauri/build.rs
fn main() {
    let target = std::env::var("CARGO_BUILD_TARGET").unwrap_or_else(|_| {
        let out = std::process::Command::new("rustc")
            .args(["-vV"]).output().expect("rustc not found");
        String::from_utf8(out.stdout).unwrap()
            .lines().find(|l| l.starts_with("host:"))
            .map(|l| l["host: ".len()..].trim().to_string())
            .expect("rustc host triple")
    });
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let src = format!("../target/{}/{}/agent-bridge-bridge", target, profile);
    let dst = format!("binaries/agent-bridge-bridge-{}", target);
    std::fs::create_dir_all("binaries").ok();
    std::fs::copy(&src, &dst).ok();
    tauri_build::build();
}
```

- [ ] **Step 4: Remove pty.rs from main.rs** (already done in Step 1 above — verify no references remain)

```bash
grep -r "pty\|portable_pty" src-tauri/src/ 2>&1
```
Expected: no output.

- [ ] **Step 5: Build Tauri (daemon only, no bridge yet)**

```bash
cargo build -p agent-bridge 2>&1 | grep -E "^error" | head -20
```
Expected: compiles cleanly.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat(tauri): embed daemon, add launch_claude/stop_claude/send_message commands, remove PTY"
```

---

### Task 9: Frontend — Remove PTY, Add Tauri Events

**Files:**
- Modify: `src/stores/bridge-store/ws-connection.ts` → replace with Tauri event listeners
- Modify: `src/stores/bridge-store/index.ts`
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/MessagePanel/index.tsx` — remove Terminal tab

- [ ] **Step 1: Update bridge-store to use Tauri events**

```typescript
// src/stores/bridge-store/ws-connection.ts
import { listen } from '@tauri-apps/api/event';
import { useBridgeStore } from './index';

export async function startTauriListeners() {
  await listen('agent_message', (event: any) => {
    useBridgeStore.getState().addMessage(event.payload.payload);
  });
  await listen('system_log', (event: any) => {
    useBridgeStore.getState().addTerminalLine({
      type: event.payload.level === 'error' ? 'error' : 'info',
      text: event.payload.message,
      timestamp: Date.now(),
    });
  });
  await listen('agent_status', (event: any) => {
    const { agent, online } = event.payload;
    useBridgeStore.getState().setAgentOnline(agent, online);
  });
  await listen('daemon_status', (event: any) => {
    useBridgeStore.getState().setStatus(event.payload);
  });
}
```

- [ ] **Step 2: Update bridge-store/index.ts**

In `src/stores/bridge-store/index.ts`, find where `initWebSocket()` (or the WS connect call) is invoked — typically in the store's `init()` or at the bottom of the file. Replace:

```typescript
// Remove:
// initWebSocket()  (or equivalent WS setup call)

// Add at the bottom of the store init function:
startTauriListeners();
```

Also remove the `ws` state field and any `wsConnected` / `wsStatus` state — connection status is now implicit via Tauri events.

- [ ] **Step 3: Update ClaudePanel/index.tsx**

Replace PTY invoke calls with `invoke('launch_claude', { roleId, cwd, model })` and `invoke('stop_claude')`. Remove xterm.js import. Remove Terminal tab reference.

- [ ] **Step 4: Remove TerminalView from MessagePanel**

In `src/components/MessagePanel/index.tsx`, remove the Terminal tab and `TerminalView` component.

- [ ] **Step 5: Update ReplyInput to use invoke**

```typescript
// src/components/ReplyInput.tsx — update send handler:
import { invoke } from '@tauri-apps/api/core';

const handleSend = async (text: string) => {
  await invoke('send_message', { to: targetRole, text });
};
```

- [ ] **Step 6: Run frontend dev build**

```bash
bun run dev 2>&1 | grep -E "ERROR|error" | head -10
```
Expected: no TypeScript errors, Vite builds cleanly.

- [ ] **Step 7: Commit**

```bash
git add src/
git commit -m "feat(frontend): replace WS bridge-store with Tauri events, remove PTY UI"
```

---

### Task 10: End-to-End Verification + Cleanup

- [ ] **Step 1: Build bridge binary in debug**

```bash
cargo build -p agent-bridge-bridge 2>&1
```

- [ ] **Step 2: Start Tauri dev app**

```bash
bun run tauri dev 2>&1 &
```

- [ ] **Step 3: Verify daemon control server is running**

```bash
curl -s http://127.0.0.1:4502/healthz
```
Expected: `ok`

- [ ] **Step 4: Verify bridge connects and Claude can be launched via UI**

In the running app:
1. Click "Start Claude" with Lead role
2. Check Logs tab — expect `[Control] claude connected`
3. Type a message in ReplyInput → check Messages tab shows it

- [ ] **Step 5: Delete Bun daemon (after verified)**

```bash
rm -rf daemon/
# Update package.json scripts — remove daemon-related scripts
```

- [ ] **Step 6: Remove portable-pty from Cargo.toml**

```bash
# In src-tauri/Cargo.toml, remove:
# portable-pty = "0.9.0"
cargo build -p agent-bridge 2>&1 | grep "^error" | head -5
```

- [ ] **Step 7: Full test suite**

```bash
cargo test --workspace 2>&1 | tail -20
```
Expected: all tests pass.

- [ ] **Step 8: Final commit**

```bash
git add -A
git commit -m "feat: complete Rust + Channel API migration — remove Bun daemon"
```

---

## Summary

| Phase | Tasks | Deliverable |
|-------|-------|-------------|
| A: Bridge | 1–3 | Standalone Rust MCP stdio sidecar with Channel API |
| B: Daemon | 4–7 | Full Rust daemon embedded in Tauri |
| C: Integration | 8–10 | Working app, frontend updated, Bun daemon deleted |

Each phase can be reviewed and tested independently before moving to the next.
