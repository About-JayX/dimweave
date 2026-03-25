# Rust + Channel API Migration Design

> Historical note: this dated spec records the migration target and tradeoffs. Current source of truth is `CLAUDE.md`, `.claude/rules/**`, `src-tauri/src/daemon/**`, and `bridge/**`.

**Date**: 2026-03-25
**Status**: Draft

## 1. 目标

将 Bun daemon 全量迁移到 Rust，与 Tauri 合并为单一进程。以 Claude Channel API 替代 PTY inject，实现真正的双向推送通信，彻底去除 Bun 运行时依赖。

### 核心收益

| 现状 | 迁移后 |
|------|--------|
| Bun daemon 独立进程，需用户安装 Bun | 单一 `.dmg` 分发，零外部依赖 |
| PTY inject 注入消息（脆弱） | Channel API 直接 push（可靠）|
| check_messages poll（Claude 侧） | push 通知，Claude 自动响应 |
| GUI 通过 WS :4503 与 daemon 通信 | Tauri emit/invoke 直接通信，零 WS 开销 |

---

## 2. 架构

### 2.1 进程模型

```
Tauri 主进程 (agent-bridge)
├── UI 层: React + Vite (Tauri WebView)
│     ↕ Tauri emit / invoke（无 WS :4503）
└── daemon (tokio async tasks，内嵌主进程)
    ├── gui.rs           → Tauri app.emit() → 前端事件
    ├── control_server   → WS :4502 ← bridge 连入
    ├── codex_adapter    → WS :4500 → Codex app-server
    └── session_manager  → /tmp 临时目录生命周期

bridge sidecar (agent-bridge-bridge)
← Claude Code 以 stdio 方式 spawn（从 --mcp-config 读取）
├── MCP stdio server     ← Claude Code（JSON-RPC over stdin/stdout）
│   ├── Channel push     → notifications/claude/channel → Claude
│   └── reply tool       ← Claude 调用，WS 转发给 daemon
└── daemon_client.rs     → WS :4502 → daemon control_server（自动重连）
```

**GUI 通信**：全部改为 Tauri emit（daemon → 前端）和 Tauri invoke（前端 → daemon）。WS :4503 不再存在于 Rust 版本中。

### 2.2 Claude 启动方式

daemon 通过 `tokio::sync::mpsc` channel 接收前端的启动指令（由 `launch_claude` Tauri command 触发），然后在 daemon tokio task 内用 `std::process::Command` spawn `claude` 进程：

```rust
// Tauri command（前端调用）
#[tauri::command]
async fn launch_claude(state: State<'_, AppState>, opts: LaunchOpts) -> Result<(), String> {
    state.daemon_tx.send(DaemonCmd::LaunchClaude(opts)).await
        .map_err(|e| e.to_string())
}

// daemon 内处理
Command::new("claude")
    .args([
        "--dangerously-load-development-channels", "server:agentbridge",
        "--dangerously-skip-permissions",
        "--strict-mcp-config",
        "--mcp-config", &mcp_config_json,
        "--agent", &role_id,
        "--agents", &agents_json,
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::piped())
    .spawn()
```

**`--dangerously-load-development-channels server:agentbridge`**：Channel API research preview 必须的 flag（来源：[官方文档](https://code.claude.com/docs/en/channels-reference#test-during-the-research-preview)）。`server:agentbridge` 中的 `agentbridge` 对应 `mcp_config_json` 里的 `mcpServers` key 名称。正式发布后此 flag 可移除。

**`mcp_config_json` 格式**：

```json
{
  "mcpServers": {
    "agentbridge": {
      "command": "agent-bridge-bridge",
      "args": [],
      "env": {
        "AGENTBRIDGE_CONTROL_PORT": "4502",
        "AGENTBRIDGE_AGENT": "claude"
      }
    }
  }
}
```

Claude Code 读取此配置，spawn `agent-bridge-bridge` 子进程，通过 stdio 与之通信。

### 2.3 通信流

**daemon → Claude（push）**

```
routeMessage(msg, to="claude")
  → control_server 向 bridge WS 连接发送 {"type":"routed_message","message":{...}}
  → bridge 收到 → 写 stdout: notifications/claude/channel notification
  → Claude Code 注入 <channel> tag → Claude 自动响应
```

**Claude → daemon（reply）**

```
Claude 调用 reply tool → MCP tools/call → bridge tools.rs
  → bridge daemon_client 发送 {"type":"agent_reply","message":{...}}
  → daemon control_server handler → routeMessage → Codex / GUI
```

**前端用户输入 → Claude**

```
UI ReplyInput → Tauri invoke "send_message" → daemon
  → routeMessage(to="lead") → control_server → bridge → Channel notification
```

**Codex 通信（dynamicTools，协议不变）**

```
daemon codex_adapter → turn/start → Codex app-server (WS :4500)
Codex LLM → dynamicTool reply / check_messages / get_status → daemon handler
daemon → routeMessage → 目标 agent
```

### 2.4 claude 进程崩溃处理

daemon 监控 claude 子进程退出（`child.wait().await`），退出时：

1. 向 Tauri emit `agent_status { agent: "claude", online: false, exitCode }`
2. 将 claude agent 从 attachedAgents 中移除（bridge 断连时已触发）
3. **不自动重启**——由用户通过 UI 重新点击启动按钮

---

## 3. 协议规范

### 3.1 daemon ↔ bridge WS 消息格式（:4502）

**daemon → bridge**（push 消息给 Claude）：

```json
{ "type": "routed_message", "message": { ...BridgeMessage } }
{ "type": "status", "status": { ...DaemonStatus } }
```

**bridge → daemon**：

```json
{ "type": "agent_connect",    "agentId": "claude" }
{ "type": "agent_reply",      "message": { ...BridgeMessage } }
{ "type": "agent_disconnect"                                   }
```

与现有 TypeScript control-protocol.ts 格式兼容，无变更。

### 3.2 BridgeMessage（Rust）

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub content: String,
    pub timestamp: u64,          // Unix ms，对应前端 number
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<String>,  // 序列化为 "replyTo"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}
```

### 3.3 Channel API notification（stdio JSON-RPC）

```json
{
  "jsonrpc": "2.0",
  "method": "notifications/claude/channel",
  "params": {
    "content": "<消息正文>",
    "meta": { "from": "coder", "chat_id": "msg-123" }
  }
}
```

meta key 只允许字母/数字/下划线（含连字符的 key 会被 Claude Code 静默丢弃）。

**`BridgeMessage` → Channel `meta` 映射**（`channel.rs` 实现依据）：

| meta key | 来源 |
|----------|------|
| `from` | `msg.from`（发送方角色名） |
| `chat_id` | `msg.id`（消息唯一 ID，供 reply tool 回传用） |

`content` 字段直接使用 `msg.content`，不附加其他 BridgeMessage 字段。

### 3.4 bridge MCP tools（Claude 侧）

bridge 仅暴露一个 tool：**`reply`**。

```json
{
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
}
```

`check_messages` 和 `get_status` **不在 bridge MCP tools 中**——它们是 Codex dynamicTools（Codex 调 daemon），与 Claude MCP tools 无关。Claude 不再需要 poll，Channel push 替代。

---

## 4. 模块结构

### 4.1 Cargo workspace

```
agent-bridge/
├── Cargo.toml              # [workspace] members = ["src-tauri", "bridge"]
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs            # 编译后复制 bridge binary 到 binaries/
│   └── src/
│       ├── main.rs         # Tauri builder，注册 commands，启动 daemon tokio task
│       ├── daemon/
│       │   ├── mod.rs
│       │   ├── state.rs         # Arc<RwLock<DaemonState>>
│       │   ├── gui.rs           # app.emit() helpers（agent_message/system_log 等）
│       │   ├── routing.rs       # routeMessage：按 to 字段分发
│       │   ├── control/         # WS :4502，bridge 连入
│       │   │   ├── mod.rs
│       │   │   ├── server.rs    # axum WS 升级 + accept loop
│       │   │   └── handler.rs   # agent_connect / agent_reply / disconnect 处理
│       │   ├── codex/
│       │   │   ├── mod.rs
│       │   │   ├── lifecycle.rs
│       │   │   ├── session.rs   # thread/start + dynamicTools 注册
│       │   │   ├── proxy.rs
│       │   │   └── handler.rs   # turn notification 解析 + accountInfo 捕获
│       │   ├── session_manager.rs   # CODEX_HOME /tmp 生命周期
│       │   └── role_config/
│       │       ├── mod.rs
│       │       └── roles.rs
│       └── codex/               # 现有：auth/usage/models/oauth（不变）
└── bridge/
    ├── Cargo.toml
    └── src/
        ├── main.rs              # env 解析，tokio::main，spawn mcp task + daemon_client task
        ├── mcp.rs               # MCP stdio JSON-RPC（手动实现，不依赖 rmcp）
        ├── channel.rs           # notifications/claude/channel push 逻辑
        ├── tools.rs             # reply tool handler → agent_reply 消息
        └── daemon_client.rs     # WS client → :4502，自动重连，routed_message 接收
```

**200 行/文件硬性约束**，超过则拆分子模块。

### 4.2 Tauri sidecar 打包

`src-tauri/tauri.conf.json`：

```json
{
  "bundle": {
    "externalBin": ["binaries/agent-bridge-bridge"]
  }
}
```

`src-tauri/tauri.conf.json`（完整配置片段）：

```json
{
  "build": {
    "beforeBuildCommand": "cargo build -p agent-bridge-bridge --release"
  },
  "bundle": {
    "externalBin": ["binaries/agent-bridge-bridge"]
  }
}
```

`src-tauri/build.rs`（sidecar binary 复制）：

```rust
fn main() {
    // CARGO_BUILD_TARGET 在交叉编译时由 Tauri CLI 设置；
    // 本机编译时用 rustc -vV 获取 host triple
    let target = std::env::var("CARGO_BUILD_TARGET").unwrap_or_else(|_| {
        let output = std::process::Command::new("rustc")
            .args(["-vV"])
            .output()
            .expect("rustc not found");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .find(|l| l.starts_with("host:"))
            .map(|l| l["host: ".len()..].trim().to_string())
            .expect("cannot parse rustc host triple")
    });
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".into());
    let src = format!("../target/{}/{}/agent-bridge-bridge", target, profile);
    let dst = format!("binaries/agent-bridge-bridge-{}", target);
    std::fs::create_dir_all("binaries").ok();
    std::fs::copy(&src, &dst).ok(); // 首次编译 bridge 未就绪时静默忽略
    tauri_build::build();
}
```

---

## 5. bridge 重连规范

`bridge/src/daemon_client.rs` 实现以下重连行为：

- 首次连接失败：指数退避重试（100ms → 200ms → 400ms，上限 5s），最多重试 20 次
- 已建立连接后断开：立即触发重连循环（退避策略同上）
- 重连成功后：重新发送 `agent_connect { agentId: "claude" }` 重新注册
- daemon 未就绪时 bridge 已被 Claude Code spawn：静默等待，不 panic，不 exit（bridge 是 Claude 的 MCP server，不能提前退出）

---

## 6. 前端变更

### 6.1 移除

- `TerminalView.tsx` + xterm.js 依赖（`package.json`）
- `ClaudePanel` 中的 PTY invoke（`launch_pty` / `stop_pty` / `pty_write` / `pty_resize`）
- `bridge-store` 中的 `pty_inject` 事件处理
- `ws-connection.ts` 中 WS :4503 连接逻辑（改为 Tauri 事件）

### 6.2 新增 / 保留

- `ClaudePanel`：启动/停止按钮 → `invoke("launch_claude", opts)` / `invoke("stop_claude")`
- Claude 状态 dot：监听 Tauri 事件 `agent_status { agent:"claude", online:bool }`
- `ReplyInput`：`invoke("send_message", { to, text })` → daemon routeMessage（替换 WS :4503）
- `MessagePanel` Messages + Logs tab：监听 Tauri 事件 `agent_message` / `system_log`（保留，改为 Tauri 事件驱动）

### 6.3 新增 Tauri Commands

| Command | 参数 | 说明 |
|---------|------|------|
| `launch_claude` | `{ roleId, cwd, model }` | 通知 daemon spawn claude；`agentsJson`/`mcpConfigJson` 由 daemon 从 `roleId` 内部构建，不由前端传入 |
| `stop_claude` | — | daemon kill claude 进程；队列中待发消息**丢弃**，GUI 收到 `system_log { level:"warn", message:"Claude stopped, pending messages discarded" }` |
| `send_message` | `{ to, text }` | 用户消息路由 |

`launch_claude` 失败时（daemon_tx 发送错误）：Tauri command 返回 `Err(String)`，前端显示 `system_log` 错误级别提示，不静默忽略。

---

## 7. 移除内容

| 模块 | 原因 |
|------|------|
| `pty.rs`（Rust）+ `portable-pty` crate | Channel API 不需要 PTY |
| `daemon/**/*.ts`（所有 TypeScript daemon） | 全量迁移到 src-tauri/src/daemon/ |
| `daemon/bridge.ts` + `daemon-client/` | 迁移到 bridge/ crate |
| `check_messages` / `get_status` MCP tools（Claude 侧 bridge） | Channel push 替代 poll；仅保留 `reply` |
| `pty_inject` GUI event | Channel push 替代 |
| WS :4503 gui-server | 改为 Tauri emit/invoke |
| `bun run daemon` npm script | 改为 Tauri 内嵌启动 |

---

## 8. 保留内容

| 模块 | 说明 |
|------|------|
| `codex/`（Rust auth/usage/models/oauth） | 不变 |
| Codex dynamicTools（reply/check_messages/get_status） | 不变，Codex 调 daemon，非 Claude MCP |
| `session_manager` 逻辑 | 迁移到 Rust，逻辑等价 |
| Role config（roles/starlark） | 迁移到 Rust，逻辑等价 |
| React 前端 `src/`（除 PTY + WS 相关） | 保留，改为 Tauri 事件驱动 |
| BridgeMessage 协议格式 | 字段兼容，serde camelCase |

---

## 9. 迁移步骤与检查点

| 步骤 | 内容 | 完成标准 |
|------|------|---------|
| 1 | Cargo workspace 初始化，bridge crate 骨架 | `cargo build` 零报错 |
| 2 | bridge：MCP stdio + Channel push + reply tool | stdin/stdout 能完成 MCP initialize 握手 |
| 3 | bridge：daemon_client WS + 自动重连 | bridge 连上 daemon，agent_connect 被收到 |
| 4 | daemon：control_server + routing | routed_message 能正确转发到 bridge WS |
| 5 | daemon：codex_adapter + session_manager | Codex 启动正常，dynamicTools 可调 |
| 6 | daemon：gui.rs Tauri emit | 前端 agent_message / system_log 事件正常到达 |
| 7 | Tauri commands：launch_claude / stop_claude / send_message | UI 按钮能启停 Claude，消息能发送 |
| 8 | 前端：删除 PTY + WS，接入 Tauri 事件 | HMR 无报错，Messages/Logs 正常显示 |
| 9 | 打包验证 | `bun run tauri build` 生成可用 `.dmg`，bridge sidecar 包含其中 |
| 10 | 删除 `daemon/`（Bun） | 步骤 1-9 全部通过后执行 |

---

## 10. 风险

| 风险 | 缓解 |
|------|------|
| Channel API 需 `--dangerously-load-development-channels` | 接受，等正式发布后移除 flag |
| Channel API 需 claude.ai OAuth（不支持 API key） | 项目已有 OAuth 登录，无影响 |
| `rmcp` 不支持 Channel API | 手动实现 stdio JSON-RPC，逻辑可控 |
| bridge sidecar 跨平台 binary 命名 | build.rs 按 TARGET 自动命名，tauri bundle 处理 |
| claude 进程崩溃 | daemon 监控 exit，emit agent_status offline，不自动重启 |
