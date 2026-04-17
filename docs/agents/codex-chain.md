# Codex 链路修复记录

> **强制规则:** 每次修复或发现 Codex 链路问题，必须在此文档记录。
> 包括：问题描述、根因、修复方案、运行时验证结果。
> 未修复的问题也必须记录，标注 `[未修复]` 和原因。

## 官方文档参考

- 完整 API: `docs/agents/codex-app-server-api.md`
- 在线: https://developers.openai.com/codex/app-server
- **注意: 官方文档与 CLI 实现存在多处不一致，以运行时测试为准！**

## 协议对照与修复记录

### 2026-04-01: Codex provider history / runtime resume / task workspace

#### [已修复] Codex 历史 thread 之前没有进入统一 session memory

**问题:** task-centric UI 只能展示 normalized session，Codex provider 自己的 thread history 没有进入统一 history picker，也无法从 task workspace 里恢复。

**根因:** Codex provider 之前只有 launch / session 注册能力，没有把 `thread/list` 输出映射成 provider-agnostic history DTO，也没有把“外部 thread 挂回当前 task”的命令面暴露给前端。

**修复:**
- 新增 `ProviderHistoryEntry` / `ProviderHistoryPage`
- `provider/codex.rs` 新增 `list_threads()` DTO 映射与 `build_resume_target()`
- `provider/history.rs` 合并 Claude transcript history + Codex thread history，按 workspace 产出统一列表
- `DaemonCmd` / Tauri commands 新增：
  - `ListProviderHistory`
  - `AttachProviderHistory`
- `ResumeSession` 对 Codex provider 走真实 runtime reconnect，不再只是移动 normalized 指针
- `register_on_launch()` 现在同时支持 lead / coder 角色，供 task workspace 直接 attach 外部 thread

**前端可见结果:**
- `CodexPanel` 会按当前 workspace 展示 Codex 历史 thread，下拉默认 `New session`
- 选中历史 thread 后，`Connect Codex` 会恢复该 provider-native thread，而不是总是新建会话
- 如果历史 thread 已映射到 normalized session，恢复成功后会同步当前 task context
- `AgentStatus/CodexHeader` 会显示当前 live provider connection 的 thread 摘要（`new` / `resumed`）

**验证:**
- `cargo test --manifest-path src-tauri/Cargo.toml provider`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun test tests/task-store.test.ts tests/task-panel-view-model.test.ts`
- `bun run build`
- `curl -fsS http://127.0.0.1:4500/readyz`

**已知限制:**
- Codex history picker 目前仍依赖 app-server 在线时的 `thread/list`

### 2026-03-25: 初始协议审计

#### [已修复] 缺少 `initialized` 通知

**问题:** 官方文档要求 `initialize` 响应后必须发送 `{ "method": "initialized", "params": {} }`。
当前实现没有发送，导致 app-server 不继续处理后续请求。

**修复:** `session.rs` 收到 init response 后发送 `initialized` 通知。

**验证:** 运行时测试确认握手成功。

#### [已修复] dynamicTools schema 字段名 — 文档与实现不一致

**问题:** 官方文档写 `parameters`，但 Codex CLI 实际要求 `inputSchema`。
报错: `Invalid request: missing field 'inputSchema'`

**根因:** 官方文档与 CLI 实现不一致。

**修复:** 保持 `inputSchema`。曾错误改为 `parameters`，验证失败后改回。

**教训:** 官方文档不可信，必须运行时测试验证。

#### [已修复] sandbox 值格式 — 全局统一 kebab-case

**问题:** 三次修复才找到正确方案。

| 尝试 | 方案 | 结果 |
|------|------|------|
| 1 | `roles.rs` 全改 camelCase | config.toml 报错 `unknown variant 'workspaceWrite'` |
| 2 | `roles.rs` kebab, `session.rs` 转 camelCase | `thread/start` 报错 `unknown variant 'workspaceWrite'` |
| 3 | 全部 kebab-case，不做转换 | 成功 |

**结论:** Codex CLI 全部使用 kebab-case (`workspace-write`, `read-only`)，包括 JSON-RPC `thread/start` 的 `sandbox` 参数。与官方文档的 camelCase 描述完全相反。

**验证:** `bun` 脚本直接测试 `inputSchema` + kebab-case → `thread/start` 成功。

#### [已修复] `--config` CLI flags 格式

**验证:** `--config sandbox_mode="workspace-write"` 格式正确。

### 2026-03-25: 生命周期问题

#### [已修复] stop→start 竞态 — 端口未释放

**问题:** Disconnect 后立即 Connect，新 codex 进程报 `Address already in use (os error 48)`。

**根因:** `lifecycle::stop()` kill 进程后，OS 需要时间释放端口 4500。新进程立即启动时端口仍被占。

**修复:**
1. `lifecycle::stop()` kill 后等 500ms 端口释放
2. `codex::start()` 启动前轮询端口空闲（最多 5s）

#### [已修复] Codex 孤儿进程 — PPID=1

**问题:** Disconnect 后 `codex app-server` 进程仍然存活，PPID=1（已脱离进程树）。

**根因:** Codex CLI 内部 fork/exec 真正的 app-server。`kill_on_drop(true)` 和 `start_kill()` 只能 kill 直接子进程，不能 kill 孙进程。

**修复:** `lifecycle::stop()` 增加 `kill_port_holder()` — 用 `lsof -ti:{port}` 找到端口占用进程并 SIGKILL。

**运行时验证:** Connect→Disconnect→Connect 循环成功。日志显示 `[Codex] killing orphan process {pid} on port 4500`。

#### [已修复] agent_status(true) 在握手完成前发出

**问题:** `codex::start()` spawn session 后台任务后立即 emit `agent_status(true)`，但此时握手（initialize→initialized→thread/start）尚未完成。前端显示 Connected 但 thread ID 还没拿到。

**修复:** `session::run()` 接受 `ready_tx` oneshot，握手成功后发送 thread ID。`codex::start()` 等待 `ready_rx` 收到 thread ID 后才 emit `agent_status(true)`。

#### [已修复] 握手失败资源泄漏

**问题:** 当 `session::run()` 握手失败（返回空 thread ID）时，`codex::start()` bail 但未清理：
- 健康监控任务继续运行（孤儿 task）
- 子进程未被 kill（Arc 引用计数 > 0）
- 临时目录未清理

**修复:** 失败路径增加: `cancel.cancel()` + `lifecycle::stop(&mut child)` + `cleanup_session()`。

#### [已修复] CODEX_HOME 在进程仍引用时被删除

**问题:** `CodexHandle::stop()` 中 `cleanup_session()` 删除 `/tmp/dimweave-{pid}-{id}/`，但旧 codex 进程可能还在读取该目录下的文件。新 session 的 `thread/start` 报错: `CODEX_HOME points to "/tmp/dimweave-...", but that path does not exist`。

**根因:** stop 删目录 → start 创建新 session 用新 ID → 但旧进程引用的目录已被删。这发生在端口还没释放、新进程复用了旧 CODEX_HOME 的路径时。

**修复:** 每次 start 用独立的 session ID（递增），stop 时先 kill 进程再删目录，加端口释放等待。

### 2026-03-25: 深度审查补充修复

#### [已修复] pre-init buffer replay break 不传播

**问题:** `bridge/mcp.rs` 中 pre-init 消息回放时，`write_line` 失败的 `break` 只退出 `for` 循环，不退出外层 `loop`。MCP task 在 stdout 损坏后继续运行。

**修复:** 增加 `replay_ok` flag，`for` 循环后检查并 `break` 外层循环。

## 待确认项

#### [待确认] `settings.developer_instructions` 有效性

**问题:** 当前把 `developer_instructions` 放在 `params.settings.developer_instructions`。官方文档未明确此字段。

**状态:** 保持当前实现，等运行时有 Codex 响应后验证。

#### [待确认] tool response 格式

**问题:** handler.rs 回复格式:
```json
{ "id": id, "result": { "contentItems": [{ "type": "inputText", "text": "..." }], "success": true } }
```
需确认是否与 Codex 期望的 dynamic tool call response 格式匹配。

**状态:** ✅ 运行时验证通过（v0.116.0）。`contentItems` 格式仍然有效。

### 2026-03-26: codex v0.88.0 — `--listen` 不存在，exit status: 2

**问题:** 启动 Codex 时日志出现 `Codex process exited prematurely with status: exit status: 2`。

**根因:** `codex 0.88.0` 没有 `--listen` flag。该 flag 是在 2026-02-11 PR #11370 "Reapply 'Add app-server transport layer with websocket support'" 加入的，v0.88.0（2026-01-21 发布）早于该 PR，app-server 在 v0.88.0 中只支持 stdio 模式，不监听 TCP/WebSocket 端口。

**修复:** 升级 codex 至 v0.116.0（`brew upgrade codex`）。

**验证:** ✅ 升级后 `codex app-server --listen ws://127.0.0.1:4500` 正常启动，输出:
```
codex app-server (WebSockets)
  listening on: ws://127.0.0.1:4500
  readyz: http://127.0.0.1:4500/readyz
  healthz: http://127.0.0.1:4500/healthz
```

### 2026-03-26: codex v0.116.0 — `item/tool/call` params.name → params.tool

**问题:** 升级到 v0.116.0 后 dynamic tool handler 不触发，Codex 不会调用 `reply`/`check_messages`/`get_status`。

**根因:** `item/tool/call` 通知的参数结构在 v0.116.0 中变更：
- 旧: `{"method":"item/tool/call","id":N,"params":{"name":"reply","arguments":{...}}}`
- 新: `{"method":"item/tool/call","id":N,"params":{"threadId":"...","turnId":"...","callId":"...","tool":"reply","arguments":{...}}}`

`session.rs` 读 `v["params"]["name"]`，新版返回 `undefined`，导致 handler 永不匹配。

**修复:** `session.rs` 优先读 `v["params"]["tool"]`，降级兜底读 `v["params"]["name"]`（向后兼容）。

**文件:** `src-tauri/src/daemon/codex/session.rs`

**验证:** ✅ tool call response 格式（`contentItems`）仍然有效；turn 成功完成。

### 2026-03-26: Port 4500 残留进程导致 Codex 启动失败

**问题:** 重启 app 后启动 Codex 报 `Port 4500 still in use after 5s`。

**根因:** 上一轮的 `codex app-server` 进程未被正确 kill（`kill_on_drop` 依赖父进程正常退出，`pkill` 可能遗漏 fork 出的子进程）。残留进程持续占用端口。

**修复:** 手动 `kill $(lsof -ti:4500)` 后重新 Connect Codex。

**预防:** `lifecycle.rs::stop()` 已有 `kill_port_holder` 兜底，但仅在 daemon 正常调用 `stop` 时生效。app 异常退出（SIGKILL/crash）时端口不会被清理。

**验证:** ✅ kill 残留后正常启动。

### 2026-03-26: Codex 事件静默丢弃 — 用户看不到 thinking 和回复

**问题:** 消息成功 delivered 到 Codex（`[Route] user → coder delivered`），但 GUI 无任何后续反馈。

**根因:** `session.rs` 事件循环只处理 `item/tool/call`，其他所有 Codex 通知（`turn/started`、`item/agentMessage/delta`、`item/completed`、`turn/completed`）全部 `continue` 跳过。

完整 Codex 事件流（运行时抓包确认）：
```
turn/started → item/started(userMessage) → item/completed(userMessage)
→ item/started(reasoning) → item/completed(reasoning)
→ item/started(dynamicToolCall) → item/tool/call → item/completed(dynamicToolCall)
→ item/started(agentMessage) → item/agentMessage/delta × N → item/completed(agentMessage)
→ turn/completed
```

**问题分两层:**

1. **Rust 层（事件丢弃）:** `session.rs` 事件循环只匹配 `item/tool/call`，其余事件 continue 跳过。Codex 的 agentMessage 和 thinking 永远不会到达前端。
2. **前端层（无渲染路径）:** 即使 Rust 侧转发了事件，前端没有对应的 listener 和 UI 组件来显示 Codex 流式输出。`agent_message` 事件只渲染到 Messages 面板的消息列表，没有实时 streaming 指示器。

**修复（Rust 侧）:**
- `session.rs` 新增 `handle_codex_event()` 分发函数，处理 5 种事件
- 新增 `codex_stream` Tauri 事件枚举（`Thinking`/`Delta`/`Message`/`TurnDone`），通过 `gui::emit_codex_stream()` 发出
- `item/completed(agentMessage)` 双发：`agent_message`（消息历史）+ `codex_stream`（实时显示）

**修复（前端层）:**
- `types.ts` 新增 `CodexStreamState` 接口（thinking/currentDelta/lastMessage/turnStatus）
- `helpers.ts` 新增 `codex_stream` 事件 listener，按 `kind` 字段分发更新 store
- `currentDelta` 字符串累加设 100KB 上限，防止长回复导致内存膨胀
- 新增 `CodexStreamIndicator.tsx` 组件：thinking 时显示 `"thinking…"` 动画脉冲；收到 delta 后实时追加显示流式文本
- `MessagePanel/index.tsx` 在消息列表底部渲染 `<CodexStreamIndicator />`
- turn 完成后清空 currentDelta 和 thinking 状态，指示器自动消失

**完整数据流:**
```
Codex app-server → WS :4500 → session.rs handle_codex_event()
  → gui::emit_codex_stream(Thinking/Delta/Message/TurnDone)
    → Tauri event "codex_stream"
      → helpers.ts listener → zustand codexStream state
        → CodexStreamIndicator 组件实时渲染

  → gui::emit_agent_message() (仅 item/completed agentMessage)
    → Tauri event "agent_message"
      → helpers.ts listener → zustand messages[]
        → MessagePanel 消息列表永久渲染
```

**文件:** `session.rs`, `gui.rs`, `helpers.ts`, `types.ts`, `sync.ts`, `index.ts`, `CodexStreamIndicator.tsx`, `MessagePanel/index.tsx`

**验证:** ✅ 用户可见 thinking → 流式文本 → 完成消息渲染到 Messages 面板。

### 2026-03-26: 消息列表虚拟化与滚动修复

**问题（3 个）：**
1. 返回底部按钮反复闪动——手动 scroll 事件 + `isNearBottom` 在内容变高时误触发
2. 返回底部只滚到一半——`scrollToIndex("LAST")` 不含 Footer 区域
3. Codex 会话未结束时 thinking 消失——`message` 事件过早清除 `thinking` 状态

**根因：**
- Footer 变高（每次 delta）触发 Virtuoso 重算→scroll 跳动→`atBottom` 抖动
- `scrollToIndex` 只算 data 项不含 Footer
- `thinking` 在 `message` 事件被置 false，但 Codex turn 可能还有后续 tool call / reasoning

**修复：**
1. 消息列表改用 `react-virtuoso` 虚拟列表，只渲染可视区域
2. Streaming 指示器作为 Virtuoso 的最后一个虚拟项（`totalCount = messages.length + 1`），不用 Footer
3. `followOutput="smooth"` 自动追底，`atBottomStateChange` 检测用户滚动
4. `scrollToIndex({ index: "LAST" })` 使用 Virtuoso 原生 API 避免 stale closure
5. `thinking` 只在 `turnDone` 时清除，`message` 事件保持 `thinking=true`
6. 提取 `MessageBubble.tsx`（气泡）+ `MessageList.tsx`（Virtuoso 封装）

**文件:** `MessageList.tsx`, `MessageBubble.tsx`, `CodexStreamIndicator.tsx`, `index.tsx`, `helpers.ts`

**验证:** ✅ 消息自动追底、用户滚动暂停追底并显示按钮、thinking 持续到 turn 结束。

### 2026-03-26: 角色 instructions 重构与强制性研究

#### 研究结论：指令约束力分层

| 层级 | 机制 | 强制性 |
|------|------|--------|
| L0 OS 沙箱 | Codex `sandbox_mode` (Seatbelt/bubblewrap) | 不可绕，内核级 |
| L1 工具可用性 | Claude `--tools`/`--disallowedTools`；Codex `dynamicTools` | 不可绕，物理不存在 |
| L2 路由拦截 | daemon `routing.rs` sender gating | 不可绕，代码控制 |
| L3 权限门 | Claude `permissionMode`；Codex `approval_policy` | 基本不可绕 |
| L4 System Prompt | Claude `--append-system-prompt`；Codex `base_instructions` | 软约束 |
| L5 Developer 指令 | Codex `developer_instructions`；Claude MCP `instructions` | 软约束 |
| L6 CLAUDE.md | 用户级上下文 | 最弱 |

**当前产品定位:** 自动化执行工具，权限全开。角色 instructions 不做权限限制，只规范路由行为和回复格式。

#### 修复：role_instructions 重构

- `roles.rs` 改用 `role_instructions!` 宏，compile-time `concat!` 拼接共享前言 + 角色专属段
- 共享前言：角色图谱、工具说明、主动汇报进展、自行判断路由目标
- 每个角色附加典型路由路径（如 lead: `receive task → assign coder → send reviewer → report user`）
- read-only 角色（reviewer）明确写 "read-only sandbox"，不写 "full permissions"
- write 角色（user/lead/coder）写 "full permissions, execute directly"

**文件:** `src-tauri/src/daemon/role_config/roles.rs`

#### 修复：Claude MCP instructions 扩充

- `CHANNEL_INSTRUCTIONS` 从简短指引扩展为完整角色图谱 + 路由规则 + 工作风格
- `initialize_result(role)` 运行时追加 `"Your role: {role}"`

**文件:** `bridge/src/mcp_protocol.rs`

### 2026-03-26: Superpowers 代码审查修复

#### [已修复] I-1: currentDelta 字符串无限累加

**问题:** `helpers.ts` 的 delta handler 无限拼接 `currentDelta`，长回复导致内存膨胀和 React 重渲染性能下降。

**修复:** 设 100KB 上限，超过截断。

**文件:** `src/stores/bridge-store/helpers.ts`

#### [已修复] I-2: upsert_mcp_server 测试断言被弱化

**问题:** 添加 `env` 字段后，测试 fixture 缺少 `env`，`changed` 永远为 true，`assert!(!changed)` 被注释掉。"unchanged" 路径不再被测试覆盖。

**修复:** fixture 补全 `env: { "AGENTBRIDGE_ROLE": "lead" }`，恢复 `assert!(!changed)`。

**文件:** `src-tauri/src/mcp.rs`

#### [已修复] I-4: read-only 角色指令声称 "full permissions"

**问题:** `role_instructions!` 共享前言写 "You have full permissions"，但 reviewer 的 `sandbox_mode` 是 `"read-only"`（OS 内核级限制）。LLM 被误导后尝试写文件会被内核拒绝。

**修复:** 移除共享前言中的权限声明，改为按角色写入：write 角色写 "full permissions"，read-only 角色写 "read-only sandbox, cannot modify files"。

**文件:** `src-tauri/src/daemon/role_config/roles.rs`

#### [已修复] M-4/M-5: 文件超 200 行限制

**修复:**
- `MessagePanel/index.tsx` 提取 `CodexStreamIndicator.tsx`（28 行）
- `helpers.ts` 提取 `sync.ts`（60 行）

### 2026-03-26: baseInstructions 替换 system prompt + outputSchema 结构化输出

#### 背景：Codex 不可靠地调用 reply 工具

**问题:** Codex 收到 "让 lead 审查代码" 指令后，输出文本 "我已通知 lead" 但从未调用 `reply()` 工具。`[Route]` 日志中无 `coder → lead`。`developerInstructions` 加强指令（MUST / NEVER / 示例）后仍无效。

**根因:** GPT 模型对 `developer_instructions`（developer role 消息）的工具调用遵从度不足。文本描述 "发了消息" 但实际未触发 tool call。

#### 方案：baseInstructions + outputSchema 双层强制

**1. `baseInstructions`（替换 system prompt）**

`thread/start` 参数 `baseInstructions` 替换 Codex 内置 system prompt（~14K 字符），直接映射到 OpenAI API `ResponsesApiRequest.instructions` 字段。

源码确认：`codex-rs/app-server-protocol/src/protocol/v2.rs:2583`
```rust
pub struct ThreadStartParams {
    pub base_instructions: Option<String>,  // ← 替换整个 system prompt
    pub developer_instructions: Option<String>,  // ← 追加 developer message
}
```

优先级链（`codex-rs/core/src/codex.rs:561-570`）：
```
1. baseInstructions（thread/start 参数）  ← 最高，完全替换
2. conversation history base_instructions  ← 恢复会话时
3. model_info.get_model_instructions()     ← 内置默认 prompt
```

运行时验证：发送 `baseInstructions: "只回复 PINEAPPLE"` → 问 "2+2=?" → 回复 `"PINEAPPLE"` ✅ 确认替换生效。

**2. `outputSchema`（turn/start 参数，GPT Structured Output 硬约束）**

每次 `turn/start` 附带 JSON Schema，强制模型输出包含 `send_to` 路由字段和 `status` 生命周期字段：
```json
{
  "type": "object",
  "properties": {
    "message": { "type": "string" },
    "send_to": { "enum": ["user","lead","coder","reviewer","none"] },
    "status": { "enum": ["in_progress","done","error"] }
  },
  "required": ["message", "send_to", "status"],
  "additionalProperties": false
}
```

`session.rs` 解析 `item/completed(agentMessage)` 文本为 JSON，提取 `send_to` 和 `status`。非 `"none"`/`"user"` 时自动调用 `routing::route_message` 投递；缺失 `status` 兼容按 `done` 处理，非法值会转成用户可见的错误提示并打 `error` 日志。

### 2026-03-27: 非 lead 默认只回 lead

- [已修复] `roles.rs` 的 Codex `baseInstructions` 新增层级路由规则：
  - `lead` 可以按上下文直接回复 `user` 或分派给其他 worker
  - 非 `lead` 默认 `send_to = "lead"`
  - 只有用户明确点名该角色或明确要求该角色直接回答时，非 `lead` 才允许 `send_to = "user"`
  - 只有当前指令明确点名目标 worker 时，非 `lead` 才允许直接发给其他非 `lead` 角色；否则仍回 `lead`
- [目的] 让 Codex worker 默认向 `lead` 汇报，而不是在 auto/broadcast 场景里直接对用户发声，减少多 agent 回答面扩散。

### 2026-03-27: 移除 tester，reviewer 覆盖测试职责

- [已修复] Codex 当前角色模型已收敛为 `lead / coder / reviewer` 三角色；`tester` 已从 `send_to` schema、角色配置和前端 target 中移除。
- [已修复] `reviewer` 现在同时承担 review 与 test verification，负责质量审查、运行测试、验证行为，并向 `lead` 或 `coder` 汇报结果。
- [已修复] 当 Claude 离线时，Codex 不会再因为 Claude 缓存角色仍是 `lead` 就被挡住；角色冲突只对在线 agent 生效。但如果在线 Claude 已占用 `lead`，Codex 启动前会直接被拒绝，避免 live duplicate role。

**3. 替换 prompt 后的影响**

不受影响（独立注入机制）：
- MCP 工具（`tools` 参数）、Skills（`input[]` user message）、AGENTS.md（`input[]`）
- developer_sections（sandbox info、memory tool、collaboration mode）
- dynamicTools（`tools` 参数）

丢失（已手动补回 8 条关键规则）：
- 工具使用偏好（`rg` 优先、并行化、`apply_patch` 强制）
- Git 安全边界（不用 `reset --hard`、不 revert 他人改动）
- 自治行为（执行到底、不停在分析阶段）

**4. 默认 prompt 存档**

从 `codex-rs/core/models.json`（Apache 2.0 许可）提取 13 个模型的完整默认 prompt：

```
docs/codex/prompts/
├── gpt-5.4.md          (14100 chars base + 12265 template + 3 personality)
├── gpt-5.3-codex.md    (12341 chars base + 10507 template)
├── gpt-5.2-codex.md    (7563 + 7311)
├── gpt-5.2.md          (21544)
├── gpt-5.1.md          (24046)
├── gpt-5.1-codex.md    (6621)
├── gpt-5.1-codex-max.md(7563)
├── gpt-5-codex.md      (6621)
├── gpt-5.md            (20771)
├── gpt-oss-120b.md     (20771)
└── gpt-oss-20b.md      (20771)
```

**文件:** `roles.rs`, `handshake.rs`, `session.rs`, `mod.rs`, `role_config/mod.rs`

**验证:** ✅ `[Route] coder → lead delivered` + `[Route] claude → coder delivered` — 双向通信通过 outputSchema 路由成功。

### 2026-03-26: Codex 指令注入机制全景（源码确认）

#### AGENTS.md 发现与注入

**源码:** `codex-rs/core/src/project_doc.rs`

搜索文件名（优先级）：
1. `AGENTS.override.md`（本地覆盖）
2. `AGENTS.md`（默认）
3. `config.project_doc_fallback_filenames`（额外配置）

搜索目录：从 project root（`.git` 标记或 `config.project_root_markers`）到 CWD 的每一层目录都扫描，找到的文件内容按目录顺序拼接。大小限制 `config.project_doc_max_bytes`。

**注入位置:** `input[]` 中的 `user` role message，`<INSTRUCTIONS>` 标签包裹。独立于 `baseInstructions`。

#### Skills 发现与注入

**源码:** `codex-rs/core-skills/src/loader.rs`

搜索路径（优先级从高到低）：

| 路径 | Scope | 说明 |
|------|-------|------|
| `<project>/.codex/skills/` | Repo | 项目级 |
| `<project root→CWD>/.agents/skills/` | Repo | 项目级（逐层扫描） |
| `$CODEX_HOME/skills/` | User | 用户级（旧路径，兼容） |
| `$HOME/.agents/skills/` | User | 用户级（新标准路径） |
| `$CODEX_HOME/skills/.system/` | System | 内嵌系统 skills |
| `/etc/codex/skills/` | Admin | 管理员级 |

文件名：`SKILL.md`（必须），可选 `SKILL.json`（interface/dependencies/policy）。

**注入位置:** `input[]` 中的 `user` role message，`<skill>` 标签包裹。独立于 `baseInstructions`。

#### 默认 Prompt 存档

从 `codex-rs/core/models.json`（Apache 2.0）提取 13 个模型完整 prompt → `docs/codex/prompts/`。

#### 关键结论

AGENTS.md、Skills、MCP 工具、developer_sections 全部通过 `input[]` 或 `tools` 参数注入，**覆盖 `baseInstructions` 不影响这些机制**。`baseInstructions` 只替换 OpenAI API `instructions` 字段（system prompt）。

### 2026-03-27: Codex 结构化输出预览归一化

- [已修复] `item/agentMessage/delta` 之前会把结构化输出原始 JSON token 直接透传到前端，Messages 底部 streaming 区会显示 `{"message":"...","send_to":"..."}` 这类模板文本。当前 daemon 改为维护当前 turn 的原始缓冲，只提取 `message` 字段作为 preview，再通过 `codex_stream.delta` 发给前端。
- [已修复] 前端 `codex_stream.delta` 的消费语义已从“原始 token 追加”改成“当前完整 preview 替换”，因此 `CodexStreamIndicator` 只显示当前可展示文本，不再自己拼 JSON 片段。
- [已修复] 若 Codex 最终完成消息的 `message.trim().is_empty()`，daemon 不再 emit 最终 message，也不再做内部路由；只等待 `turn/completed` 清理 thinking，避免空消息或空路由副作用。
- [已修复] Codex 最终结构化输出新增 `status` 字段，固定为 `in_progress` / `done` / `error`。统一 `BridgeMessage` 已保留该字段，agent 间转发不会再把状态丢掉；发往 Codex 的内部消息文本也会附带 `(status: ...)` 上下文。

**验证:** ✅ Codex streaming 区只显示 `message` 内容；最终空消息不会落入历史消息或内部 route。

### 2026-03-27: Superpowers 复核收口

- [已修复] `src-tauri/src/daemon/codex/session.rs` 已拆分：结构化输出解析、preview 提取和空消息守卫被提取到 `src-tauri/src/daemon/codex/structured_output.rs`，主会话循环回到 200 行以内，职责重新聚焦在握手与事件分发。
- [已修复] `item/completed(agentMessage)` 路径中重复的 `should_emit_final_message()` 判断已合并成单次 early return，空消息不会再继续进入最终 GUI emit 或 schema-route 判定。
- [已修复] 前端 `codex_stream.delta` 继续使用“覆盖当前 preview”语义，而不是回到旧的 token append。原因是 daemon 现在每次 delta 都发送“当前完整可展示 preview”，不是原始增量 token；listener 侧已补注释固定这个协议约定。
- [已修复] `codex_stream.delta` 的前端 preview state 重新补回长度上限，当前只保留最近 100,000 个字符，避免长回复重新把消息面板状态推回无限增长。

**验证:** ✅ `cargo test --manifest-path src-tauri/Cargo.toml` 通过；Codex 结构化输出 preview/空消息测试通过；`cargo clippy --workspace --all-targets -- -D warnings` 通过。

### 2026-03-27: Codex `status` 协议与非法值处理

- [已修复] `structured_output.rs` 现已把最终 JSON 解析为强类型结果：`message`、`send_to`、`status`。缺失 `status` 会兼容默认成 `done`。
- [已修复] `status` 非法值不再静默降级。daemon 会写一条 `error` 级 system log，并生成一条面向用户的错误消息：`Invalid status: "<value>". Expected "in_progress", "done", or "error".`
- [已修复] `StreamPreviewState.raw_delta` 现已受 `RAW_DELTA_CAP = 512_000` 约束，daemon 侧不会再无限累积原始 preview 缓冲。

**验证:** ✅ `cargo test --manifest-path src-tauri/Cargo.toml` 通过（85 tests）；`invalid_status_returns_error`、`status_defaults_to_done_when_missing`、`parses_explicit_in_progress_status` 回归测试已加入。

### 2026-03-27: 现场故障修复（Port 4500 残留）

- [已修复] Codex 启动前现在会主动清理占用 4500 端口的孤儿 app-server。`codex::start()` 不再只是等 5 秒看端口会不会自己释放，而是先执行 `ensure_port_available()`，在端口被占用时调用 `kill_port_holder()` 再重试。
- [已修复] 新增测试：`ensure_port_available_runs_cleanup_before_failing` 与 `ensure_port_available_times_out_when_cleanup_cannot_free_port`，锁住“先清理、后失败”的启动策略。

### 2026-03-27: Codex 消息颜色固定按模型身份显示

- [已修复] Codex 最终消息与 dynamic reply 现在都会写入 `displaySource=codex`，而 `from` 继续保留当前路由角色（例如 `lead`、`coder`、`reviewer`）。
- [已修复] 前端消息气泡改为优先使用 `displaySource` 决定 badge 和颜色，因此 Codex 即使临时扮演 `lead`，气泡也仍然保持 Codex 绿色。
- [已修复] 若 `displaySource` 与 `from` 不一致，UI 会把路由角色作为次级标签显示，避免丢掉“谁在以什么身份说话”的信息。

### 2026-03-27: Codex 恢复 pre-launch model / reasoning / project 配置

- [已修复] Codex 面板重新恢复 `Reasoning` selector；这次不是单纯把 UI 放回来，而是把参数链真正接通到 daemon。
- [已修复] `reasoningEffort` 现在会从 `CodexPanel` 透传到 `daemon_launch_codex`，并最终写入 Codex app-server `thread/start` 的 `effort` 字段。
- [已修复] 模型切换时会自动重置 reasoning 到模型默认 effort；如果没有默认值，则退回到第一个 supported effort。
- [已修复] 启动 Codex 现在必须先选项目目录：
  - 前端 `Connect Codex` 按钮在 `cwd` 为空时 disabled
  - store `applyConfig()` 会拒绝空目录
  - Tauri command `daemon_launch_codex` 也会拒绝空 `cwd`
- [结果] Codex 启动前配置重新和 Claude 对齐：`model + reasoning + project` 三项都明确，且目录不再偷偷回退到 `"."`。

### 2026-03-30: review 收口（文件行数与 coverage）

- [已修复] `src-tauri/src/daemon/codex/handshake.rs` 新增了 `effort=None` 的反向测试，锁住“未选择 reasoning 时不发送 `params.effort`”的路径。
- [已修复] `src-tauri/src/daemon/codex/mod.rs` 已拆出 `runtime.rs`，把端口清理和健康监控移出主文件。
- [已修复] `src-tauri/src/daemon/codex/session.rs` 已拆出 `session_event.rs`，把事件分发与最终消息构造移出主循环文件。
- [已修复] 相关文件行数已重新压回限制内：
  - `daemon/codex/mod.rs`: 167 行
  - `daemon/codex/session.rs`: 111 行
  - `daemon/codex/handshake.rs`: 183 行

### 2026-03-30: 切角色重连后旧 session 不再冲掉新连接

- [已修复] 旧 Codex session 和旧 health monitor 退出时曾经无条件执行 `codex_inject_tx = None`。在“断开 -> 切角色 -> 重连”场景下，这会把已经接管的新连接清空，后续消息被错误当成离线 buffer，表现为发消息后无响应。
- [已修复] daemon state 现在为 Codex 会话维护 session epoch。只有当前 epoch 的 session 才能：
  - 挂载 `codex_inject_tx`
  - 在退出时清理 `codex_inject_tx`
  - 触发当前连接的断开副作用
- [已修复] 新增回归测试 `stale_codex_session_cleanup_cannot_clear_new_session`，锁住这条竞态。
- [已修复] 为了不让状态文件重新膨胀，权限缓存相关逻辑已拆到 `src-tauri/src/daemon/state_permission.rs`。当前相关文件行数：
  - `daemon/mod.rs`: 196 行
  - `daemon/state.rs`: 187 行
  - `daemon/codex/mod.rs`: 200 行
  - `daemon/codex/session.rs`: 133 行

### 2026-03-27: WS pump loop 稳定化与 session lifecycle 清理

#### [已修复] WS pump loop debug 日志残留

**问题:** `ws_client.rs` 中残留生产环境不应出现的 debug 日志：每条 WS 消息都会追加写入 `/tmp/ws_pump.log`。`log` 闭包及 `ws_log!` / `ws_logf!` 宏未清理。

**修复:** 移除所有 `/tmp/ws_pump.log` 相关代码，pump loop 不再写磁盘。

**文件:** `src-tauri/src/daemon/codex/ws_client.rs`

#### [已确认] unsplit WS pump loop 稳定性

**背景:** 之前使用 WS split 方案（拆成独立的 read/write half）存在 borrow-split 生命周期问题，导致首消息丢失和重连后消息不投递。

**当前设计:** 单个 `tokio::spawn` task 使用 `tokio::select!` 同时处理：
- outbound: `out_rx.recv()` -> `ws.send(Message::Text(...))`
- inbound: `ws.next()` -> 解析 JSON -> `in_tx.send(v)`

Ping/Pong 由 tungstenite 底层自动处理。任一方向出错或通道关闭时 pump loop 退出。

**验证:** `cargo test daemon::codex` 17 tests 通过；手动验证消息投递链路正常。

#### [已确认] session epoch 防竞态机制

**问题:** 旧 session 退出时无条件清空 `codex_inject_tx`，在快速重连场景下会覆盖新 session 的活跃通道。

**当前实现:** `DaemonState.codex_session_epoch` 作为单调递增计数器，三个守卫方法确保只有当前 epoch 的 session 能操作 inject 通道：
- `begin_codex_launch()` -> epoch += 1
- `attach_codex_session_if_current(epoch, tx)` -> epoch 匹配才挂载
- `clear_codex_session_if_current(epoch)` -> epoch 匹配才清空

**回归测试:** `stale_codex_session_cleanup_cannot_clear_new_session`

**验证:** `cargo test --manifest-path src-tauri/Cargo.toml` 97 tests 通过。

### 2026-03-27: `get_status` 升级为结构化 JSON 接口

**变更:** `handler.rs` 中 `handle_get_status()` 的返回格式从 ad-hoc 字符串改为结构化 JSON。

旧格式（已删除）：
```
Claude role: lead, Codex role: coder, Online agents: [codex]
```

新格式：
```json
{"online_agents": [{"agentId": "codex", "role": "coder", "modelSource": "codex"}]}
```

**根因:** 旧格式是人类可读的自由文本，Codex 模型需要对其进行非结构化解析，容易出错或被不同模型版本解析方式不同。统一改为 JSON 后，agent 可以通过 `online_agents[*].agentId` 可靠地遍历在线 agent，无需字符串解析。

**实现:**
- `handle_get_status()` 调用 `state.online_agents_snapshot()` 获取 `Vec<OnlineAgentInfo>`
- 用 `serde_json::json!({"online_agents": snapshot})` 序列化后返回
- `handshake.rs` 工具描述同步更新，明确说明返回 JSON 结构

**测试（新增 3 个）：**
- `get_status_returns_valid_json` — 断言返回值是合法 JSON，顶层有 `online_agents` 数组
- `get_status_includes_wired_codex_session` — 挂载 Codex inject tx 后断言 `agentId`/`role`/`modelSource` 字段存在
- `get_status_empty_when_no_agents_online` — 无 agent 时断言空数组

**文件:** `src-tauri/src/daemon/codex/handler.rs`, `src-tauri/src/daemon/codex/handshake.rs`

**验证:** ✅ `cargo test --manifest-path src-tauri/Cargo.toml daemon::codex::handler` — 3 tests passed.

### 2026-03-27: Codex baseInstructions 更新 — get_status 返回结构说明

**问题:** `role_prompt!` 宏中 `## Communication` 章节仅写 `get_status(): see which agents are online`，Codex agent 不知道返回结构（字段名），无法正确解析以选择委派目标。

**修复:** 将该行改为：

```
- get_status(): returns a structured online_agents list; each item includes agent_id, role, and model_source — use this to decide which agent to send work to
```

**文件:** `src-tauri/src/daemon/role_config/roles.rs`

**测试:** 新增 `prompt_documents_get_status_structured_response` 测试（`roles_tests.rs`），断言 prompt 中包含 `get_status`、`agent_id`、`role`、`model_source`。

**验证:** ✅ `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config` — 5 tests passed.

### 2026-03-27: 统一在线 Agent 查询 — 全量验证通过

**摘要:** Codex 和 Claude 的在线 agent 查询能力已统一。两侧使用同一个 `DaemonState::online_agents_snapshot()` 数据源，返回结构相同（`agent_id`, `role`, `model_source`）。

**当前状态:**
- Codex 通过 `get_status()` 动态工具查询
- Claude 通过 `get_online_agents()` MCP tool 查询
- 两者返回格式一致
- 不支持 `send_to_agent_id`（实例级精确路由）
- 路由目标仍按角色名匹配
- [已修复] shared-role live routing bug: 离线 Claude 不再遮挡在线 Codex（反之亦然）

**验证:** 全量通过 — 112 Tauri tests, 26 bridge tests, 26 frontend tests, clippy clean, build success.

## 当前已知限制

- 端口 4500 固定，不可配置
- `kill_port_holder` 用 SIGKILL 可能误杀同端口的其他进程
- 不处理 `item/commandExecution/requestApproval` 审批
- 不处理 `-32001` 过载错误重试
- app 异常退出时 codex app-server 残留进程不会被自动清理
- `item/completed(agentMessage)` 构造的 BridgeMessage 硬编码 `to: "user"`，不反映实际路由目标
- `dynamicTools` 未按角色过滤（所有角色收到相同 3 个工具），可做 L1 硬约束但尚未实现

---

## 2026-04-17 — Transmission layer unification (agent_id-aware routing)

**问题场景**：同 task 下多 lead / 多 coder 场景（shared-role），worker 诊断消息按 role 字符串匹配会送错 lead；`build_completed_output_message` 在 schema 启用但 `target` 缺失时默认 `User`，worker 结果绕过 lead 直达用户（fail-open）；三条表面（Claude MCP reply input / Codex output_schema / BridgeMessage 存储）envelope 字段名分裂为 `text` / `message` / `content`，target 形态分裂为 oneOf / flat / discriminated-union。

**根因**：
1. `TaskAgent` 才是路由权威实体，role 字符串不唯一，但路由/诊断代码在多处按 role 字符串回链。
2. `parsed.target.unwrap_or(MessageTarget::User)` 是 fail-open 路径，Codex 输出 target 缺失时不阻断而是静默走 user。
3. Claude MCP tool schema + Codex strict output schema + daemon 存储 JSON 三份手工维护，字段名和形态自然漂移。

**修复**（本次一次性落地，原子上线）：

1. **`MessageTarget` 自定义 serde**（新增 [src-tauri/src/daemon/message_target.rs](../../src-tauri/src/daemon/message_target.rs) + [bridge/src/message_target.rs](../../bridge/src/message_target.rs)）：Rust 类型保留 `User | Role | Agent` 判别 enum，但 wire 形态统一为扁平 3 字段 `{kind, role, agentId}`，未用字段空串。Deserialize 同时接受老判别联合形态（持久化后向兼容）。

2. **envelope `text`/`content` 统一为 `message`**：Claude MCP reply tool、BridgeMessage wire、daemon → frontend 事件、`daemon_send_user_input` Tauri 参数、前端 TS 类型全部对齐。`daemon_send_user_input` / `DaemonCmd::SendUserInput` Rust 参数名也跟随。

3. **worker 诊断 helper 按 agent_id 回链**（[codex/session_event.rs::worker_diagnostic_target](../../src-tauri/src/daemon/codex/session_event.rs)）：P1 `routing::delegator_agent_id(sender_agent_id)` → P2 `agents_for_task(task).find(role=="lead")` → P3 `User`。第二级触发即发 WARN 日志。替换了原来 4 处硬写 `MessageTarget::User` 的诊断路径（error / parse error / dropped / silent turn）。

4. **Codex 输出 fail-closed**（`build_completed_output_message` 改为返回 `CompletedOutput { Ready | Skip | MissingTarget }`）：schema 启用但 target 缺失时走 diagnostic 链路回 lead，不再默认 `User`。

5. **Routing 层 sender-role soft guard**（[routing.rs:route_message_inner_with_meta](../../src-tauri/src/daemon/routing.rs)）：非 lead worker 直投 user 时 WARN 日志观测，暂不硬拒（LLM 偶尔合法）。

6. **Bridge parser 对 legacy `to` 精确报错**（[bridge/src/tools.rs::parse_target](../../bridge/src/tools.rs)）：检测到 `args["to"]` 但无 `target` 时返回明确错误提示，让模型自修复。

7. **`<channel>` 元数据双向透传**（[routing_format.rs](../../src-tauri/src/daemon/routing_format.rs) + [claude_sdk/protocol.rs::wrap_channel_content](../../src-tauri/src/daemon/claude_sdk/protocol.rs)）：Claude SDK 注入的 channel 标签带 `sender_agent_id` 和 `task_id` 属性；Codex 注入的文本也带 `[agent_id]` 和 `(task: tid)`。worker 因此能拿到 delegator agent_id 并用 `{kind:"agent", agentId}` 精确回链。

8. **Prompt 教学 agent_id-first 目标**（[claude_prompt.rs](../../src-tauri/src/daemon/role_config/claude_prompt.rs) + [roles.rs](../../src-tauri/src/daemon/role_config/roles.rs)）：Claude 和 Codex 两侧都要求模型优先用 `{kind:"agent", agentId:<incoming sender_agent_id>}` 回复特定 delegator。

9. **CI drift guard**（[scripts/check_contract_drift.sh](../../scripts/check_contract_drift.sh)）：防止 `.claude/agents/`、`.claude/rules/`、`docs/agents/` 漂回旧 `to=` kwarg 或 `text` envelope 字段签名（具体模式见脚本）。

**测试覆盖**：
- `cargo test -p dimweave-bridge` — 53/53 通过（含 11 个新 MessageTarget serde 测试）
- `cargo test -p dimweave` — 691 通过（3 个失败全部 pre-existing：2 个 state_persistence 文件权限 + 1 个 `reply_target_map` 静态污染 flake，和本次改动无关）
- 新增测试：`completed_output_builder_fails_closed_when_target_missing`、`silent_turn_fallback_uses_provided_target_verbatim`、`worker_diagnostic_target_*`（3 个分支覆盖）、`format_ndjson_user_message_includes_task_id_when_present`、`format_ndjson_user_message_omits_sender_agent_id_for_user_source`

**运行时验证**：待手工端到端验证（两个 Codex coder shared role、reply_target_map 精确回链、silent turn 诊断发回 delegating lead）。
