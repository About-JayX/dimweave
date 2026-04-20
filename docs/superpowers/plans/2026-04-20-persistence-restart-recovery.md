# 2026-04-20 — Persistence + Restart Recovery + Codex Config Compatibility

## Context

重启 dev / 关闭应用后多层状态丢失，外加 Codex app-server 升级带来的 config
schema 破坏，迫使一轮横向整理。串起来的链路问题：

1. **Task graph 未持久化到磁盘。** `DaemonState::new()` 在 `daemon::run()` 里
   直接用，`db_path` 是 `None`，`auto_save_task_graph()` 静默 no-op。重启后
   `~/Library/Application Support/com.dimweave.app/task_graph.db` 始终 0 字节。
2. **Task runtime 容器未补初始化。** 即便 DB 持久化修好，`task_runtimes`
   HashMap 是纯内存态，启动时没从 `task_graph.tasks` 重建，导致
   `begin_codex_task_launch_for_agent` 找不到 task_runtime，attach 失败、
   session 被 cancel，pump 瞬间 `in_tx closed`。
3. **聊天消息从未入库。** `bridge-store.messages[]` 仅靠 `agent_message` 实时
   事件堆积，重启归零。已投递消息和 `buffered_messages`（排队中未送达）语义
   完全不同，需要独立的 `task_messages` 表。
4. **Codex wire_api schema break。** Codex app-server 升级后弃用
   `wire_api = "chat"`，启动即 `error loading default config`。同时
   `model_providers.<n>` 新增 `name` 字段要求，缺失会报 `missing field 'name'`。
5. **前端读已废弃字段。** `TaskInfo.workspaceRoot` 后端早已改名 `projectRoot`，
   但前端 5 处仍在读旧字段，`effectiveCwd` 因此为空 → `fetchProviderHistory`
   永不触发、Claude/Codex History 下拉全空、workspace 任务列表也加载不到。
6. **TaskSetupDialog 漏传 providerHistory。** 对话框里的 Session 下拉只剩
   "New session"；ClaudePanel/CodexPanel 高级面板的独立查询不覆盖这里。
7. **Shell 侧边栏展开状态无持久化。** 历史上从未实现，用户误记为回归。
8. **Prompt "stay-silent" 信号被误报。** `roles.rs` 指令模型空消息结构化输出
   表示"保持沉默"，但 `handle_completed_agent_message` 的 Skip 分支没标
   `mark_durable_output`，turn/completed 因而触发 "[Codex] lead turn
   completed with no visible output" 诊断噪声。
9. **Provider auth 改动后旧 runtime 继续跑。** 换 API key / 切订阅模式后，
   已在线的 Codex/Claude subprocess 还用着旧的环境变量，endpoint 其实没
   被重定向。

## 落地内容（原子上线，可拆多 commit）

### Step 1 — Task graph 持久化 default path
- 文件：`src-tauri/src/daemon/mod.rs`
- 新增 `default_task_graph_db_path()` 返回 `dirs::config_dir() +
  "com.dimweave.app" + "task_graph.db"`（与 `feishu_project::config` /
  `telegram::config` 共用目录）。
- 新增 `build_initial_state()` 在 `run()` 开头取代 `DaemonState::new()`：
  有 path 则走 `with_task_graph_path`，失败 fallback 到 in-memory 并
  `eprintln` 日志，不阻塞启动。

### Step 2 — Task runtime hydrate
- 文件：`src-tauri/src/daemon/mod.rs`
- 新增 `hydrate_task_runtimes(&mut state)`：遍历 `task_graph.list_tasks()`
  给每个持久化 task 调 `init_task_runtime(task_id, task_worktree_root)`。
  slot 内容仍为空（没有 live 连接），但容器到位，`begin_*_task_launch_for_agent`
  不再因 `get_mut(task_id) = None` 提前退出。
- 修复症状：重启后点 Save & Connect，Codex handshake 不再 `in_tx closed`
  立即崩溃；Claude SDK launch 同样受益。

### Step 3 — task_messages 表 + schema v5 迁移
- 文件：`src-tauri/src/daemon/task_graph/persist.rs`
- `SCHEMA_VERSION: 4 → 5`；`init_schema` 多建表：
  ```sql
  CREATE TABLE task_messages (
      id         TEXT PRIMARY KEY,
      task_id    TEXT NOT NULL,
      payload    TEXT NOT NULL,
      created_at INTEGER NOT NULL
  );
  CREATE INDEX idx_task_messages_task_id ON task_messages(task_id, created_at);
  ```
- `migrate_if_needed` 加 `current_ver < 5` 分支兜底老 DB。
- **不**进 `save_to_db` 的 `DELETE + INSERT` 批量重写（那里只处理结构数据），
  走独立 `persist_task_message` / `delete_task_messages` 就地写。

### Step 4 — Message log 读写 API
- 新文件：`src-tauri/src/daemon/task_graph/message_log.rs`
- `persist_task_message(&BridgeMessage)` — `INSERT OR REPLACE`，按
  `msg.id` 幂等；`task_id` 为 None 时跳过（不入库系统诊断）。
- `list_task_messages(task_id)` — `SELECT ... ORDER BY created_at DESC
  LIMIT 500`，内存反转升序返回。
- `delete_task_messages(task_id)` — 级联删除。
- `task_graph/mod.rs` 增加 `mod message_log`。

### Step 5 — Emit 点挂 persist
- `routing_dispatch.rs` `route_message_with_display`：`display_in_gui &&
  Delivered|ToGui && renderable` 时 persist。**关键**：必须 gate 在
  `display_in_gui=true`，否则 `routing_user_input` 为每个 target 创建的
  副本（`route_message_silent`）会被重复入库，一条用户输入产生 N+1 行。
- `routing_user_input.rs`：display_msg persist 一次。
- `codex/session_event.rs` 5 处直 emit 诊断消息（silent turn fallback /
  WS error / parse error / MissingTarget / dropped）在 stamp 后 persist。
- 所有 persist 调用都在已持有的 `state.read().await` scope 内完成，
  避免多次 lock 获取。

### Step 6 — Tauri command + 前端 hydration
- `daemon/cmd.rs`：新增 `DaemonCmd::ListTaskMessages { task_id, reply }`。
- `daemon/mod.rs`：handler 直接调 `task_graph.list_task_messages`。
- `commands_history.rs`：`daemon_list_task_messages(task_id)` Tauri command。
- `main.rs`：注册到 `invoke_handler!`。
- `src/stores/bridge-store/listener-setup.ts`：`hydrateMessagesForTask(taskId)`
  invoke + replace + dedup live；task 切换 subscriber 调用；`initListeners`
  里补一次初始化兜底（防 task-store bootstrap 先行完成错过 subscribe）。
- `listener-setup.ts` task removal 分支补清 `messages.filter(taskId)`，
  避免删除后残留时间线。

### Step 7 — DeleteTask 级联
- `daemon/mod.rs::DaemonCmd::DeleteTask`：`remove_task_cascade` 前加
  `delete_task_messages(&task_id)` SQL DELETE，`save_task_graph()` 之后
  DB 完全无残留。前端 task-store 仅做 zustand 镜像清理，不直接操作 DB。

### Step 8 — Codex wire_api 升级 + name 字段
- 文件：`src-tauri/src/daemon/codex/lifecycle.rs`
- `apply_provider_auth`：
  - 空 / `"chat"` → 强制 `"responses"`，"chat" 打 warn 提示用户更新配置。
  - 新增 `model_providers.<n>.name="<n>"` `--config` 行，复用 TOML key
    作为 display name（缺失会 `missing field 'name'`）。
- 前端 `ProviderAuthDialog.tsx`：
  - `EMPTY_FORM.wireApi` + `fromConfig` 默认值 `"chat" → "responses"`。
  - 读库时 `"chat" | undefined → "responses"`，保存一次即覆盖旧值。
  - 下拉选项删掉 "chat"，只留 "responses"。
- 测试：`apply_auth_with_base_url_emits_model_provider_configs` 与
  `apply_auth_with_base_url_defaults_to_dimweave_custom_when_name_missing`
  更新断言（wire_api="responses" + name=<n>）。6/6 pass。

### Step 9 — Silent-turn Skip 路径不再误报
- 文件：`src-tauri/src/daemon/codex/session_event.rs`
- `handle_completed_agent_message` 的 `CompletedOutput::Skip` 分支加
  `stream_preview.mark_durable_output()` 再 return。prompt 要求的"有意
  沉默"空消息结构化输出现在被视作 durable，`turn/completed` 不再触发
  silent-turn diagnostic。

### Step 10 — 前端 workspaceRoot → projectRoot 清理
- `src/App.tsx`（2 处）、`components/AgentStatus/CodexPanel.tsx`（2 处）、
  `components/ClaudePanel/index.tsx`（2 处）、`components/TaskPanel/TaskHeader.tsx`
  （1 处）。`types.ts` 的 `workspaceRoot?: string` 注释保留为
  `@deprecated`，测试 fixture 不改。
- 修复症状：`effectiveCwd` 正确解析，ClaudePanel / CodexPanel 的 History
  下拉能拉到数据；workspace 任务列表正常。

### Step 11 — TaskSetupDialog provider history 接线 + 骨架
- 文件：`src/components/TaskPanel/index.tsx`
- 新增：`makeProviderHistorySelector(dialogWorkspace)` 读 store；
  `useEffect(dialogOpen && dialogWorkspace)` 期间 `Promise.allSettled`
  并行拉 `fetchProviderHistory + fetchCodexModels + fetchClaudeModels`。
- `dialogReady` 本地 state：首次无缓存 → false → 全部 settle 置 true；
  后续打开若 `providerHistory[workspace]` 已有 key + 两个 model 列表都非空
  → 直接 true（cache hit），但仍后台刷新。
- 新文件：`src/components/TaskPanel/TaskSetupDialogSkeleton.tsx`
  同构外壳（overlay + agent list + config pane + footer），内容换成
  `bg-muted/* animate-pulse` 占位块；保留 ESC / 点遮罩关闭。
- 替换：`!dialogReady` 渲染骨架，`dialogReady` 渲染 TaskSetupDialog。
- 新增 prop：TaskSetupDialog `providerHistory` 从 TaskPanel 透传。

### Step 12 — 侧边栏展开状态持久化（首次实现）
- 文件：`src/components/shell-layout-state.ts`
- `SHELL_LAYOUT_STORAGE_KEY = "dimweave:shell-layout"`。
- 新增 `loadShellLayoutState()` / `saveShellLayoutState(state)`：JSON
  序列化到 localStorage，反序列化时把 enum 未知值 fallback 到默认。
- `src/App.tsx`：`useState(loadShellLayoutState)` 懒初始化 + `useEffect`
  监听 `shellLayout` 变更即写。Key 命名对齐 `dimweave:recent-workspaces`
  / theme / border-radius 的既有前缀。

### Step 13 — Provider auth 改动拔 runtime
- 文件：`src-tauri/src/daemon/mod.rs`
- 新增 `tear_down_provider_runtime(provider, ...)`：按 provider 调
  `stop_all_codex_sessions` / `stop_all_claude_sdk_sessions`；无 handle
  则 no-op，避免多余日志。
- `DaemonCmd::SaveProviderAuth` / `ClearProviderAuth` 两个 handler 都在
  save 成功后调 tear_down。写入前 `state.write()` 的锁释放后再 tear_down
  （避免两段锁 lifetime 交叉）。
- **不自动重连**：用户可能同时改 model / endpoint，自动 relaunch 会用旧
  task_agent.model 打新 endpoint；留给用户显式 Save & Connect 更稳。

## 关键文件清单

| 文件 | 角色 |
|---|---|
| `src-tauri/src/daemon/mod.rs` | 持久化启动 path + hydrate runtime + provider auth teardown + ListTaskMessages handler |
| `src-tauri/src/daemon/task_graph/persist.rs` | schema v5 + migration |
| `src-tauri/src/daemon/task_graph/message_log.rs` | 新：message 读写 API |
| `src-tauri/src/daemon/task_graph/mod.rs` | 注册 mod |
| `src-tauri/src/daemon/routing_dispatch.rs` | persist gate on display_in_gui |
| `src-tauri/src/daemon/routing_user_input.rs` | display_msg persist |
| `src-tauri/src/daemon/codex/session_event.rs` | 5 处诊断 persist + Skip durable |
| `src-tauri/src/daemon/codex/lifecycle.rs` | wire_api 升级 + name 字段 |
| `src-tauri/src/daemon/cmd.rs` | ListTaskMessages 枚举 |
| `src-tauri/src/commands_history.rs` | daemon_list_task_messages |
| `src-tauri/src/main.rs` | invoke_handler 注册 |
| `src/App.tsx` | shell layout persist + workspaceRoot 清理 |
| `src/components/shell-layout-state.ts` | load/save + storage key |
| `src/components/TaskPanel/index.tsx` | providerHistory 透传 + 缓存 ready gate |
| `src/components/TaskPanel/TaskSetupDialogSkeleton.tsx` | 新：加载骨架 |
| `src/components/TaskPanel/TaskHeader.tsx` | projectRoot |
| `src/components/AgentStatus/CodexPanel.tsx` | projectRoot |
| `src/components/ClaudePanel/index.tsx` | projectRoot |
| `src/components/ToolsPanel/ProviderAuthDialog.tsx` | wire_api default/选项收敛 |
| `src/stores/bridge-store/helpers.ts` | initListeners 兜底 hydrate |
| `src/stores/bridge-store/listener-setup.ts` | hydrateMessagesForTask + task removal sweep |

## 验证

- `cargo test -p dimweave apply_auth` — 6/6 pass（wire_api/name 断言更新）
- `cargo test -p dimweave session_event` — 20/20 pass
- `cargo check -p dimweave` — no new errors（29 个既有 dead_code warn 未增）
- `bun x tsc --noEmit -p tsconfig.app.json` — 除既有 pre-existing（bun:test
  类型、ClaudeLaunchRequest/InvokeArgs 早期 mismatch）外无新增 TS 错误
- 手工 E2E：
  1. 新建 task，发几条消息 → 关闭 app → 重开 → 选中 task → 聊天历史恢复
  2. 重启应用 → 点 Save & Connect → Codex/Claude 成功 handshake（
     `[Codex-WS] pump: in_tx closed` 不再）
  3. 打开编辑 dialog → 先是骨架，随 history+models 全到位切到正式 dialog；
     Session 下拉含 Claude transcripts + Codex threads
  4. 切换 provider_auth 模式（subscription ↔ api_key）→ 系统日志显示
     `<Provider> auth changed — stopping active sessions`，agent 状态
     variant 置 offline，显式 Save & Connect 后用新 config 重连
  5. 删除 task → SQL 验证 `task_messages WHERE task_id=?` 行数为 0

## 已确认设计决策

- task_graph.db 路径统一 `$XDG_CONFIG_HOME/com.dimweave.app/task_graph.db`
  （与现有 config.db/telegram.json 同目录）。
- 消息持久化表与 buffered_messages 互不干涉：前者是 UI 时间线，后者是
  投递队列。
- 前端是 DB 镜像，所有写入都经 Tauri invoke → DaemonCmd → task_graph；
  前端任何"删除"都先 await 后端成功再同步 zustand。
- wire_api "chat" 一律升级 "responses" 而非给用户选择——Codex 新版彻底
  不支持 "chat"，留着只会重复触发启动 error。
- Provider auth 变更后**手动**重连，不自动。
- 侧边栏持久化只用 localStorage，不进 task_graph（UI 偏好不是跨设备状态）。

## 明确不做

- ❌ Codex 原生 transcript 推断出 dimweave 聊天时间线（过于复杂，不如直接入库）
- ❌ task_graph 的 sessions/artifacts 迁移到增量 update（当前 DELETE+INSERT
  批量重写在单用户场景够用）
- ❌ Provider auth 自动重连（避免 auth + model 同步改时的 double-fault）
- ❌ 为已废弃 `TaskInfo.workspaceRoot` 写 runtime 兼容层（直接清理调用点更干净）

## CM (Configuration Management)

### Commit 1
- **Hash**: `ef3d2da0`
- **Subject**: `feat(daemon,ui): persistence + restart recovery + codex config compat`

### Commit 2 — per-agent slot invalidation on auth teardown
- **Hash**: `(will be filled after commit)`
- **Subject**: `fix(daemon): clear per-task agent slots when auth changes; stale online flag blocked relaunch`
- **Scope**: Follow-up to Step 13. After `tear_down_provider_runtime` the UI
  panel still showed "connected" and clicking Save & Connect was a no-op.
  Root cause: `stop_all_codex_sessions` / `stop_all_claude_sdk_sessions`
  only invalidated the **singleton** session (`invalidate_codex_session` /
  `invalidate_claude_sdk_session`), leaving `task_runtimes[tid].*_slots[aid]`
  with stale `inject_tx` / `ws_tx` so `is_online()` still returned true.
  Next launch's "already online, skipping" short-circuit at
  `daemon/mod.rs::LaunchCodex` / `LaunchClaudeSdk` early-returned.
- **Files**: `src-tauri/src/daemon/mod.rs` (`stop_all_codex_sessions`,
  `stop_all_claude_sdk_sessions`)
- **Fix**: iterate every handle and call
  `invalidate_codex_agent_session(tid, aid)` /
  `invalidate_claude_agent_session(tid, aid)` which clears the slot
  `inject_tx`/`ws_tx` + `connection` and bumps `session_epoch`. Emit
  `emit_task_context_events` for **each** affected task so every TaskPanel
  instance refreshes its `agent_runtime_statuses`.
- **Scope**: 跨多个子系统的原子修复。内容对应 plan Step 1-13。
  - Rust daemon / task_graph / codex lifecycle / routing dispatch
  - 前端 bridge-store / task-store / TaskPanel / ClaudePanel / CodexPanel /
    ProviderAuthDialog / shell-layout / App.tsx
- **Late-stage cleanups**:
  - wire_api auto-upgrade 日志提示用户去 Tools → Accounts 覆盖旧值，避免
    同一警告在每次 launch 重复打印（DB 覆盖后一劳永逸）。
  - Skip 分支 durable 标记的注释解释 prompt-driven 空消息的动机，防后人
    误删。
  - tear_down_provider_runtime 的 no-op 短路（`handles.is_empty()`），避免
    无 session 时打多余日志。

## Supersedes / Related

- 扩展 `docs/superpowers/plans/2026-04-17-transmission-layer-unification.md`
  里"task persistence 延后"的遗留项。
- 与 `docs/superpowers/plans/2026-04-19-provider-auth-dialog.md`（provider
  auth 表结构）互补：那里定义静态 schema，本 plan 定义运行时接线。
