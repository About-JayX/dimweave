# Multi-Task UI Isolation — 彻底解决前端 singleton 串频道

## 触发问题

用户报告切换 task 后消息仍然蹿频道，且切到 Task 2 显示 "Reconnect to this task" 无法工作。

## 深度诊断

`agent_message` 已在前一条链（strict filter + system bypass）修过。但前端 bridge-store 里**多个 singleton 字段**还在按 "provider-only" 设计，每个都是 multi-task 漏水点。

### 事件 / 状态 映射（诊断结果）

| 事件 | 前端状态 | 载体 | 是否 per-task | 问题 |
|---|---|---|---|---|
| `agent_message` | `messages[]` | 带 taskId 的 BridgeMessage | ✅ 已过滤 | 修复 |
| `claude_stream` | `claudeStream: ClaudeStreamState` | **无 taskId** | ❌ singleton | Task 1 thinking，Task 2 指示器也亮 |
| `codex_stream` | `codexStream: CodexStreamState` | **无 taskId** | ❌ singleton | 同上 |
| `agent_status` | `agents: Record<"claude"\|"codex", AgentInfo>` | keyed by provider 名 | ❌ 每次 launch 覆盖 | providerSession 只反映最后一次 launch；`task-session-guard` 用它判断连通性 → Task 2 显示 Reconnect 但实际能发 |
| `permission_prompt` | `permissionPrompts[]` | **无 taskId** | ❌ 全局队列 | Task 1 的审批在 Task 2 队列出现 |
| `system_log` | `terminalLines[]` | **无 taskId** | ❌ 全局列表 | 日志流不区分 task（可接受，但需确认语义） |
| `claudeRole` / `codexRole` | 顶层字符串 | singleton | ❌ last-write-wins | 多 task launch 时 UI 的 role label 会漂 |
| `claudeNeedsAttention` | boolean | singleton | ❌ | Claude 需要 attention 所有 task 都显示 |

### 后端对应

- `emit_codex_stream(payload)` / `emit_claude_stream(payload)` — 27 处调用点，都没传 task_id
- `emit_agent_status` — 23 处，只传 agent 名（"claude"/"codex"）
- `emit_permission_prompt` — 2 处，`agent` 字符串传了但没 task_id
- 后端 daemon 侧自己是 per-task 的（`task_runtimes: HashMap<task_id, TaskRuntime>`、`codex_handles: HashMap<agent_id, _>`），只是 event 出口把这些信息丢掉了

### 已修但相关

- `C1` (routing sender gate task-first) — 后端 routing 层不再用 singleton codex_role gating
- `M3` (filterMessagesByTaskId + source=system bypass) — message list 过滤
- task-session-guard 加 `taskRuntimeStatuses` 参数 — 这是本次修复的第一步（已在 session 内做，待 commit）
- Worktree cleanup on DeleteTask — session 内顺手修了（commit `b6a89a41`），确认独立 bug，不等本 plan 上线再一起测（详见 Step 7）

## 修复总路线

原则：**把带 task 语义的事件都穿 `taskId` + `agentId` 出来，前端 state 按 taskId 分桶，selectors 取 active task**。不改造 `system_log` / `terminalLines`（这是跨 task 的调试日志，不算 bug）。

分 5 个 commit，按独立性 + 对 UX 的可感知度排：

### Step 1 — Stream events 带 taskId（Claude + Codex）

**Rust**:
- `gui.rs` 把 `emit_claude_stream` / `emit_codex_stream` 签名改为带 `task_id: Option<&str>` + `agent_id: Option<&str>`。内部 wrap 成 `{ taskId, agentId, payload }` 发给前端
- 所有 27 处 call site 补上 task_id + agent_id。context 通常在 `session_event.rs` / `event_handler.rs`，都能从 handler 闭包捕获到

**前端**:
- `bridge-store/types.ts` 把 `claudeStream` / `codexStream` 改成 `Record<string /* taskId */, ClaudeStreamState>`
- Listener 分发到正确 bucket
- Selector `selectActiveClaudeStream(activeTaskId)` / `selectActiveCodexStream(...)` — 组件只消费 active task 的 stream state
- Components (`MessageList.tsx`, `ClaudeStreamIndicator.tsx`, `CodexStreamIndicator.tsx`) 改用 selector
- 切 task 时，非 active task 的 stream state 保留在 map 里不动；切回来时 UI 直接看到当前的 thinking 进度

副作用：`MSG_SEQ` 之类的 global counter 不影响；新 schema 对未上线的事件无破坏。

### Step 2 — agent_status per-(task, agent_id)

**降级为 optional / 暂不做**。Step 1（per-task reply-input guard 改用 `taskRuntimeStatuses`）已经覆盖了用户报告的 Reconnect 误报。剩下 `agents` map 消费者（`selectAnyAgentConnected`、`AccountsInfoPanel`）本身是 **provider-level** 语义（订阅状态 / 用量 / profile），不需要 per-task sharding。

**已移交给**：
- 连通性判断 → `taskRuntimeStatuses[taskId]`（已做）
- 订阅 / profile / usage → `claude-account-store` / `codex-account-store`（已在 Accounts plan 做）
- 角色标签 → Step 4 改成从 task_agents 派生

除非后续发现具体消费点被 singleton 坑到，否则保持现状。

### Step 3 — permission_prompt 带 taskId + 前端过滤

**Rust**:
- `emit_permission_prompt` 加 `task_id`
- call sites 补齐

**前端**:
- `PermissionPrompt` 类型加 `taskId?: string`
- `permissionPrompts[]` 保留，但 `PermissionQueue` 组件按 `activeTaskId` 过滤
- `selectPermissionPromptCount` 也改成 active task 的计数

### Step 4 — claudeRole / codexRole / claudeNeedsAttention per-task

`claudeRole` / `codexRole` 其实可以从 `task_agents` 推导出来（active task 下第一个 Claude 的 role / 第一个 Codex 的 role）。直接去掉 singleton 改成 selector。

`claudeNeedsAttention` 改成 `Record<taskId, boolean>`，`clearClaudeAttention(taskId)` 接 taskId 参数。

### Step 5 — 后端 active_task_id stamp 收紧

每次 launch 之后，相关事件（stream / status / permission）必须带 task_id 出口。现在 `stamp_message_context` 有 `active_task_id` 兜底；audit 所有 call sites，**能拿到具体 task_id 的地方一律不要走兜底**。兜底路径只给"真·全局"事件（启动 / 关闭 / 非 task 域的系统通知）。

已经在 session_event.rs 修过两条（commit `74e7bac9`），其他 call site 要系统性 review。

### Step 6 — UX 加固

针对 "per-task sharding" 带来的副作用补一遍：

- `PermissionQueue` 不按 activeTask 隐藏其他 task 的 prompt，改成在每条 prompt 上显示 `taskId` 或 task 名，否则用户在 Task B 看不见 Task A 的审批 → 死锁等 daemon timeout
- TaskHeader 加 pending count 徽章（permission + needs_attention 汇总），其他 task 有未处理事务时用户能看到
- 删除 task 时，store 监听 `deleteTask` 事件，清理对应的 stream / attention / permission bucket（防止 map 长期增长）
- 关键组件（MessageList、StreamIndicator）保持 mount，用 CSS 隐藏非 active；避免 CSS 动画和 typing effect 在切换时重置

### Step 7 — Worktree cleanup on DeleteTask（已完成，本 plan 内统一验证）

独立 bug。已在 `b6a89a41` 修：
- `DeleteTask` handler 接了 `cleanup_task_worktree`
- `cleanup_task_worktree` 增加 `git branch -D task/<task_id>`
- 测试 `cleanup_also_deletes_task_branch` 断言分支被删

等本 plan 全部上线后一起测：创建 → 删除 → 确认 `<repo>/.worktrees/tasks/<task_id>/` 和 `task/<task_id>` 分支都消失。

## 不做的事

- `terminalLines` / `system_log` 保持全局 —— 日志流跨 task 对 debug 更有用
- `AccountsInfoPanel` 的 profile / usage / provider_auth 不动（provider-level 信息）
- 不改底层 `agent_message` filter（已经解决）
- 不动 routing sender gate（`C1` 已修）
- 不砍 `claude_sdk_ws_tx` / `codex_inject_tx` singleton fallback —— Pre-task 的 user input 路径还要兜底

## 风险 + 回退

- Step 1（stream）最脆，要在 dev 下手动测：两个 task 同时跑 Claude，切来切去指示器是否正确
- Step 2（status）最大改动。agents map 类型变更会影响测试 fixture；单独 commit，review 每个 consumer
- 所有 commit 在发现 regression 时可独立 revert，不会级联

## 验证（每步都要做）

- `cargo test` — 后端单测
- `bun x tsc --noEmit` — 无新增错误（允许 pre-existing）
- `bun run build` — OK
- 手工：
  1. 两个 task 各跑 Claude lead + Codex coder；Task 1 发消息让 Claude 思考，**Task 2 指示器不亮**
  2. Task 1 触发 permission prompt，**Task 2 审批队列里看不见**
  3. 切回 Task 1，指示器 / 队列恢复
  4. `Reconnect to this task` 不再在已连通的 Task 2 上误报
  5. 两 task 并发通讯，消息归各自 task

## Commit 规划

1. `fix(ui): per-task reply-input guard using task_runtime_statuses`（已开工，本 session 内完成）
2. `feat(stream): scope claude/codex stream events by taskId + agentId`
3. `refactor(agent-status): shard agents map by agentId so multi-task doesn't collide`
4. `fix(permission): scope permission queue by taskId, surface task context on prompt`
5. `refactor(singleton): derive claudeRole/codexRole/attention from active task`
6. `fix(ui): TaskHeader pending badges + bucket cleanup on task delete`
7. `audit(emit): tighten task_id stamping at all per-task event emit sites`
8. (已做) `fix(task): clean up git worktree + branch on DeleteTask` — `b6a89a41`

## CM 回填区

- `4b8f1357` — `docs: plan multi-task UI isolation` — plan 初稿
- `b6a89a41` — `fix(task): clean up git worktree + branch on DeleteTask` — Step 8（原 Step 7 worktree）独立 bug，先修先合
- `b7e47288` — `docs: fold worktree cleanup + UX hardening into multi-task UI plan` — plan 更新
- `3b511905` — `fix(ui): per-task reply-input guard using task_runtime_statuses` — Step 1
- `4acf53ea` — `feat(stream): scope claude/codex stream events by taskId + agentId` — Step 2（envelope + listener filter + task-switch reset）
- `ba3a6d36` — `fix(permission): surface task label on permission prompts` — Step 4（task_id 字段 + PermissionQueue 徽章）
- `2cfb640f` — `fix(ui): TaskHeader pending-approval badge + task-delete prompt sweep` — Step 6（含 task-delete 时 permissionPrompts 扫除）
- Step 3（agent_status 分片）与 Step 5（singleton 派生）— 经评估范围超出当前用户可见问题，未实施；Step 1 已覆盖 reply-input 误报，stream 指示器由 Step 2 envelope 过滤直接修复
- Step 7（emit-site 审计）— 仅 audit，无代码变更：`codex/session_event.rs` 全部 emit_agent_message 前都已调用 `stamp_message_context_for_task`，`routing_user_input.rs:45` 走 `stamp_user_message`，`control/handler.rs:235` 走 task-scoped 优先链路；现状已满足不变量，不需要补丁
