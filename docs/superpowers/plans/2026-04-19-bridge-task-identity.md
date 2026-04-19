# 2026-04-19 bridge 层 task 身份贯通（多任务消息归属 root fix）

## 背景

在 [2026-04-19-multi-task-ui-isolation](2026-04-19-multi-task-ui-isolation.md) 落地后的实机测试中，用户反馈：

> 如果存在两个 task，消息还是无法同步到 task 对应的 provider 上。意味着只会出现在一个窗口。

根因：Claude MCP bridge 回复的 `BridgeMessage` 在 daemon 侧用
`agent_owning_task_id("claude")` 决定 stamp 哪个 task_id，该 fn 只是遍历
`task_runtimes` 返回第一个 online，HashMap 迭代序非确定。两个 task 同时跑
Claude 时，所有回复都被 stamp 到同一个 task，前端过滤器只在那一个窗口展示。

`attached_agents["claude"]` 同时是 singleton：后连的 bridge 覆盖前连的，
导致只有一个 bridge 与 daemon 通信（symptom 合起来就是"只在一个窗口"）。

Codex 侧 `handle_reply` 已经在 scope 内拿到 `task_id` + `agent_id`，
不受影响。

## 目标

让每一个 bridge 实例在 handshake 时**自报其所属 task / TaskAgent**，
daemon 按连接记录身份，用它 stamp AgentReply / PermissionRequest，
不再依赖全局"猜"。

## 非目标

- 不重构 `attached_agents` 的 key（仍然按 runtime_id 单 slot；在
  SDK 模式下 bridge 不是投递通道，只处理入站工具调用，singleton 覆盖
  不影响投递）；但要补一条 WARN 日志，避免回归时看不见
- 不改 Codex 路径（已经按 task 正确 stamp）
- 不改前端 `filterMessagesByTaskId`

## 修改清单

### 1. `.mcp.json` 注入 task 身份

- `src-tauri/src/mcp.rs::build_dimweave_mcp_config`
  - 签名扩展：`(project_dir, role, task_id, task_agent_id)`
  - 写入 env：`DIMWEAVE_TASK_ID=<task_id>`、`DIMWEAVE_AGENT_ID=<TaskAgent.agent_id>`
  - 保留 `AGENTBRIDGE_ROLE`、`AGENTBRIDGE_SDK_MODE` 不变

- `src-tauri/src/daemon/mod.rs` Claude launch 链路
  - 把当前 scope 里的 `task_id` 和 `agent_id`（TaskAgent id，不是 "claude" runtime）传进 `build_dimweave_mcp_config`

- `src-tauri/src/mcp.rs::register_mcp`（用户显式注册入口）
  - 不改签名：用户侧注册写的 `.mcp.json` 没有 task 上下文，env 只带
    `AGENTBRIDGE_ROLE`；这个 `.mcp.json` 只有 Claude Code 直接启动时才用，
    daemon 启动 Claude 永远走 `--strict-mcp-config` + `build_dimweave_mcp_config`，
    不受影响

### 2. bridge 读取 env → 在 AgentConnect 里带上

- `bridge/src/main.rs`
  - 读 `DIMWEAVE_TASK_ID` / `DIMWEAVE_AGENT_ID`，透传下去
- `bridge/src/types.rs`
  - `BridgeMsg::AgentConnect` 加 `task_id: Option<String>` 和
    `task_agent_id: Option<String>`
  - 旧字段 `agent_id` 保留（runtime key：`claude` / `codex`）
- `bridge/src/daemon_client.rs`（或 AgentConnect 构造点）
  - 构造 AgentConnect 时带上 task_id、task_agent_id
- `bridge/src/daemon_client_io.rs::to_wire_message`
  - `source.agent_id` 用 `task_agent_id`（若有），否则回退 runtime id；
    便于 daemon 端 `validate_claimed_agent_id` 命中真实 TaskAgent

### 3. daemon 接 AgentConnect 新字段 + AgentReply stamp 用连接级身份

- `src-tauri/src/daemon/types.rs::FromAgent::AgentConnect`
  - 镜像加 `task_id` / `task_agent_id` 可选字段（serde default）
- `src-tauri/src/daemon/control/handler.rs`
  - `handle_connection` 在 AgentConnect 里拿到 task_id / task_agent_id，
    存为 local var `connection_task: Option<(String, String)>`（task_id,
    task_agent_id）
  - `AgentReply` 分支：
    - 若 connection_task 有值 → 用 task_agent_id 查 task_graph 验证（存在且
      provider 匹配），成功则 `stamp_message_context_for_task(task_id, role, ...)`
      + `source.agent_id = task_agent_id`
    - 否则走旧 `resolve_agent_identity` / `agent_owning_task_id`（legacy 兼容）
  - `PermissionRequest` 分支：同样优先使用 connection_task 的 task_id
  - `attached_agents.insert(runtime_id, ...)` 前 log_warn 若已存在一个不同
    连接（singleton 覆盖信号）

### 4. Direct SDK 路径兜底

- `src-tauri/src/daemon/claude_sdk/event_handler.rs::handle_assistant`
  - 目前 `build_direct_sdk_gui_message` 产出 `task_id: None`。`handle_events`
    已经有 `task_id: &str`，补进去
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs::build_direct_sdk_gui_message`
  - 签名加 `task_id: Option<&str>`，塞进 BridgeMessage

### 5. 测试

- `bridge/src/daemon_client_io_tests.rs`（若有）或新增：AgentConnect 序列化
  带 task_id / task_agent_id；to_wire_message 用 task_agent_id
- `src-tauri/src/daemon/control/handler_tests.rs`：
  - connection_task 注入后，AgentReply 被 stamp 到正确 task_id（与其它 task
    的 online Claude 无关）
  - 缺失时回退到旧路径（legacy 兼容）
- 现有 `claude_task_slot_find_task_for_nonce_scans_runtimes` 等 slot 测试不变

### 6. 文档

- `docs/agents/claude-chain.md` 追加一条 "bridge handshake carries
  DIMWEAVE_TASK_ID + DIMWEAVE_AGENT_ID" 修复记录
- `.claude/rules/daemon.md` 在"连接与重连"段补一行：bridge 握手必须
  带 task_id / task_agent_id（若被 daemon 拉起）

## 验收

- 同时跑两个 task，每个 task 各一个 Claude lead
- 从 task A 发一条消息 → 仅在 task A 窗口看到 user echo + Claude reply
- 从 task B 发一条消息 → 仅在 task B 窗口
- daemon 日志里能看到两条 AgentReply 各自带 `task_id=<A>` / `task_id=<B>`
- 关 task A：仅 A 的 bridge 断开，B 的 bridge 仍然 online（`attached_agents`
  singleton 覆盖警告若出现，说明两 bridge 同时在线 — 这是期望行为，但 B 持有
  最终 slot 时投递走 `claude_task_ws_tx_for_agent`，不受影响）

## Commit 规划

1. `feat(bridge): propagate DIMWEAVE_TASK_ID + TaskAgent id in handshake`
2. `fix(daemon): stamp bridge replies using per-connection task identity`
3. `fix(claude-sdk): direct-SDK fallback stamps task_id from handle_events scope`
4. `docs: record bridge task identity fix`

## CM 回填区

- `814b36f7` — `fix(bridge): carry task identity in handshake so multi-task stamping is correct` — 一次提交完成 step 1–5，cargo test -p dimweave-bridge + dimweave bin 全绿
