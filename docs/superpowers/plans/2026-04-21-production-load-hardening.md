# 2026-04-21 — Production-Load Hardening: SQLite Debounce + Permission Recovery + MessageList Perf

## Context

上一轮 MVP review 指出三条跑通但**生产负载下会翻车**的链路。核心症状
是"短时间压力测试没问题，长时间重度使用后 UI 卡顿 / 状态错乱"。本 plan
专门处理它们，不涉及新功能。

四条链路的共同特征：

1. **SQLite 全表重写**：`save_task_graph()` 每次都 `DELETE FROM tasks/
   sessions/artifacts/task_agents/provider_auth` + 整表 INSERT。每条路由
   成功、每个 turn 完成、每个 artifact 都触发一次。消息多 + 任务多 + 流
   密集时，这是实际的写放大源。
2. **Message 双存储对齐**：`task_messages` 表（dimweave 自己的 UI 时间
   线）和 agent transcript 文件（Claude `.jsonl` / Codex `~/.codex/
   sessions`）是两条独立写入路径；重连走 agent 的 `--resume` /
   `thread/resume`，读的是 agent 侧 transcript 而不是 DB。没有集成测试
   锁定两边一致。
3. **Claude permission 与 subprocess 重连的状态悬挂**：权限协议是 5 跳
   跨 subprocess + GUI 的握手。任一方死在半路，另一方永久等待。
4. **MessageList 在长时间线下的重复计算 + 引用抖动**：上千条消息积累后，
   每次新消息到达都会让所有 `useMemo(filter...)` 重跑一次 O(N)，MessageBubble
   的 `memo` 因为 props 引用变化失效，markdown 重新渲染。

## 设计决定

### Step 1 — `save_task_graph()` 加 debounce

目前 `auto_save_task_graph()` 同步触发 `save()`。改成：

- 新增 `SharedSaveScheduler`（模块私有），包一个 `Option<JoinHandle>` + last
  queued timestamp。
- `auto_save_task_graph()` 不再直接调 `save()`，改为触发 scheduler：
  - 若无 pending 保存任务，`tokio::spawn` 一个延迟 200ms 的 save job。
  - 若已有 pending，不做事（合并后续变更到同一次 save）。
- 保证 daemon 正常退出（`Shutdown` 命令）前强制 flush pending。

**为什么 200ms**：
- 小于 human-perceivable latency（~100ms），用户感知不到保存滞后
- 大于 Codex delta 事件间隔（典型 20-50ms），能合并一个 turn 内的
  所有 state mutation
- 小于 Claude 连续 turn 的间隔，不会跨 turn 丢窗口

**风险 & 护栏**：
- 崩溃 / 强杀：最坏丢失 200ms 内的状态变更。可接受（messages 本身在
  task_messages 表的独立 `INSERT OR REPLACE` 路径上不受影响；丢的是
  task 元数据或 session 状态切换）。
- Shutdown 路径：`DaemonCmd::Shutdown` 先 flush scheduler 再 stop。

### Step 2 — transcript 与 task_messages 对齐回归测试

消息现在有两份独立存储：dimweave `task_messages` 表（UI 时间线）和
agent 自己的 transcript（Claude `.jsonl` / Codex `~/.codex/sessions`
thread 文件）。Agent resume 时只读自己的 transcript 恢复 LLM 上下文，
不读我们的 DB。两条写入路径（dimweave persist + agent transcript 写
盘）异步并行，理论上能发散。

加一个集成测试锁定"reconnect 后两边条数 / id 集合一致"：

- 启动 Codex session → 发 N 条 user message → 每条得到 agent 回复
- 对比：
  - `task_graph.list_task_messages(task_id)` 的消息 id 集合
  - 对应 `~/.codex/sessions/**/*.jsonl` 里 `role=user|assistant` 条数
- 触发重连（send `SIGTERM` 给 codex 子进程；daemon 自动 reconnect + `thread/resume`）
- 再发一条 → 断言 agent 能引用之前的消息内容（检查 agent 返回的 text
  包含前文某个关键字）

**不实现 "new session 注入 prior context"**。user review 确认 new
session 是 intentional starts-fresh，UI 历史和 agent 记忆在这条路径上
**允许**分叉——测试只覆盖 resume 路径对齐，不覆盖 new session。

### Step 3 — Claude/Codex subprocess 死时清理 pending permissions

`claude_sdk::monitor` task 在 child exit 时已经 invalidate 会话 slot，
但 `pending_permissions` map 不清。改为：

- `invalidate_claude_agent_session_if_current` 之外，新增
  `purge_pending_permissions_for_agent(task_id, agent_id)`：
  - 扫 `state.pending_permissions`，找出该 agent 发起的 requests
  - 每条发 GUI 事件 `permission_cancelled { requestId, reason: "agent_died" }`
  - 从 map 里删除
- `bridge-store` listener 接 `permission_cancelled`，从 `permissionPrompts`
  删对应 prompt；若 GUI 正显示该 prompt 的 banner，关闭并提示"agent 已
  断开，请重新触发"。
- control_handler 收到 bridge 侧的 `AgentDisconnect` 也触发同样清理。

### Step 4 — MessageList 大数据量性能

问题链：
```
messages[] append → selectMessages 引用变化 → filterMessagesByTaskId
  → filterRenderableChatMessages → filterMessagesByQuery → MessageList
  props 变化 → Virtuoso re-measure → MessageBubble memo fails
```

整条路径每次新消息都完整跑一遍。三个可独立落地的改动：

#### 4a — bridge-store 按 task 分桶存 messages
- 现状：`messages: BridgeMessage[]` 是全局扁平数组。
- 改为：`messagesByTask: Record<string, BridgeMessage[]>` + 保留 `messages`
  兜底访问器。
- 选择器：`selectActiveTaskMessages(state, activeTaskId)` 直接从 bucket
  取，无 O(N) filter。
- 写入：`appendMessage` 按 msg.task_id 路由到对应 bucket；不带 task_id
  的系统消息进入 `__global__` bucket。
- `hydrateMessagesForTask` 直接 set bucket，不再 dedup 全量。
- 好处：下游所有 `useMemo(() => filter...)` 退化成常量查找；引用只在
  目标 task 有变更时才变，切 task 不让当前 task bucket 重新创建。

#### 4b — MessagePanel 用稳定引用的 filter 链
- `filterRenderableChatMessages` 保持，但接 bucket 输入后规模小很多。
- `filterMessagesByQuery` 仅在 `deferredSearchQuery` 非空时运行；空查询
  直接返回 input 引用（现在是 `.filter(...)`，返回新数组）。
- 把 `deferredSearchQuery` 为空时的 no-op 改成 identity-return。

#### 4c — MessageBubble 更精确的 memo 比较
- 目前 `memo()` 用默认 shallow。msg 对象来自 store，引用稳定只要没变就
  不 rerender → 本身 OK，但存在隐患：如果上游有人 map → 新对象（例如 DTO
  转换），memo 失效。
- 补救：显式 compare 函数，对比 msg.id + msg.timestamp + msg.status +
  msg.message 的 length（content 发生改变时 length 通常一起变）。
- MarkdownRenderer 内部已 memo（假定）；如未 memo 则加。验证时测。

### 收益量化假设

- 当前 1000 条 messages + 每秒 2 次 stream delta 触发 state 更新：约 4000
  filter operations / 秒。
- 改完后：0（bucket 是 O(1) 查找）。
- SQLite 写：当前一个 turn 能触发 20-50 次 `save_to_db`（routing +
  session event + task sync）。debounce 后合并为 1-2 次。

## 关键文件

| 文件 | 角色 |
|---|---|
| `src-tauri/src/daemon/state_persistence.rs` | 新增 SaveScheduler + debounce |
| `src-tauri/src/daemon/state.rs` | `auto_save_task_graph` 改走 scheduler |
| `src-tauri/src/daemon/mod.rs` | `DaemonCmd::Shutdown` 先 flush |
| `src-tauri/src/daemon/claude_sdk/mod.rs` | subprocess exit 清 pending permissions |
| `src-tauri/src/daemon/permission.rs`（若存在）/ `state_permission.rs` | 新增 `purge_pending_permissions_for_agent` |
| `src-tauri/src/daemon/gui.rs` | 新增 `emit_permission_cancelled` |
| `src/stores/bridge-store/types.ts` | `messagesByTask: Record<string, BridgeMessage[]>` |
| `src/stores/bridge-store/listener-setup.ts` | append 按 task 分桶 + 接 `permission_cancelled` |
| `src/stores/bridge-store/selectors.ts` | `selectActiveTaskMessages(activeTaskId)` |
| `src/components/MessagePanel/index.tsx` | 切换到 `selectActiveTaskMessages` |
| `src/components/MessagePanel/view-model.ts` | `filterMessagesByQuery` 空查询 identity-return |
| `src/components/MessagePanel/MessageBubble.tsx` | 显式 memo compare |

## 验证

**后端：**
- `cargo test -p dimweave` 全绿
- 新增单测：
  - `auto_save_task_graph_debounce_coalesces_bursts` — 10 次连续 save 在
    250ms 内调用，`save_to_db` 实际运行次数 ≤ 2
  - `shutdown_flushes_pending_save` — pending 存在时 Shutdown 强制 save
  - `claude_subprocess_exit_purges_pending_permissions_for_agent` — 构造
    pending → 模拟 subprocess exit → 断言 map 清空 + 事件发出

**前端：**
- `bun test src/stores/bridge-store/` 全绿
- 新增：
  - `bucket routing keeps other tasks stable reference` — 往 task A
    append 消息后，task B 的 bucket 引用不变
  - `filterMessagesByQuery empty query returns input identity` — 空
    query 返回入参引用
- Visual：生成 2000 条 mock messages，MessagePanel render 时间 < 50ms（
  Virtuoso 本身虚拟化，预期 < 16ms）

**E2E / 手工：**
- 创建 task，跑 50 个回合（每个 ~100 字），观察 UI 响应 + Activity Monitor
  的磁盘写入。预期：debounce 后 sqlite.db 的写入节奏从 "几乎每 20ms" 变
  "每 200ms"。

## 明确不做

- ❌ 不改 `task_messages` 写入路径（它本来就是独立 `INSERT OR REPLACE`，
  和全表重写无关）
- ❌ 不引入 SQLite 的 JSON 字段 / 分片存储（当前 schema 够用）
- ❌ 不做虚拟化以外的列表优化（Virtuoso 本身处理得好）
- ❌ 不把 MessageBubble markdown 结果缓存到 LRU（开销小于内存收益时再做）

## CM (Configuration Management)

### Commit 1 — SQLite save debounce
- **Hash**: `(TBD)`
- **Subject**: `perf(daemon): debounce task_graph.save_to_db to coalesce bursts`
- **Files**: state_persistence.rs, state.rs, mod.rs, new tests

### Commit 2 — transcript alignment integration test
- **Hash**: `(TBD)`
- **Subject**: `test(codex): lock task_messages/transcript alignment across resume`
- **Files**: new `src-tauri/tests/codex_resume_alignment.rs` (or harness equivalent)

### Commit 3 — Permission cleanup on subprocess death
- **Hash**: `(TBD)`
- **Subject**: `fix(permission): purge pending prompts when agent subprocess dies`
- **Files**: claude_sdk/mod.rs, codex/session.rs (exit path), state/permission*,
  gui.rs, bridge-store listener + types

### Commit 4 — MessageList production-load perf
- **Hash**: `(TBD)`
- **Subject**: `perf(message-panel): per-task message buckets + stable query identity`
- **Files**: bridge-store/{types,listener-setup,selectors}.ts,
  MessagePanel/{index,view-model}.tsx, MessageBubble.tsx, tests
