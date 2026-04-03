# Unified Task/Session Architecture Execution Tracker

关联文档：
- Plan：`docs/superpowers/plans/2026-03-31-unified-task-session-architecture.md`
- Spec：`docs/superpowers/specs/2026-03-31-unified-task-session-architecture-design.md`

## 当前审查结论

截至当前执行点，Task 1/2/3/4/5/6/7/8 已完成第一阶段落地，系统已经具备：
- 标准化 task/session/artifact 持久化模型
- Codex provider 的 history/fork/archive/resume 基础能力
- Claude provider 的 session metadata capture、本地 transcript history index 与 runtime resume
- 基础 orchestrator / review gate / task store / 完整 task workspace 第一阶段
- 自动化验证与链路文档收尾证据

当前主缺口已经收敛到：
- 原生 Tauri GUI 的完整点击式手工回放仍需在本机继续做最终 smoke

### 已有进展

1. 已新增 `task_graph` 基础域模型与索引：
   - `src-tauri/src/daemon/task_graph/{mod,types,store,session_index,task_index,artifact_index,tests}.rs`
2. `DaemonState` 已接入 `task_graph`、`active_task_id` 与基础 review gate 状态：
   - `src-tauri/src/daemon/state.rs`
   - `src-tauri/src/daemon/state_task_flow.rs`
3. 路由层已开始感知 task 上下文与 review gate：
   - `src-tauri/src/daemon/routing.rs`
   - `src-tauri/src/daemon/routing_user_input.rs`
   - `src-tauri/src/daemon/codex/handler.rs`
   - `src-tauri/src/daemon/control/handler.rs`
4. Codex WS 连接层做了整理，补出 `ws_helpers.rs`，为后续 session/history 能力打底：
   - `src-tauri/src/daemon/codex/{session,session_event,ws_client,ws_helpers}.rs`

### 当前残余限制

1. **原生 GUI 完整回放仍未自动化**
   - 当前环境可以完成 Tauri/daemon/codex 启动 smoke，但无法完整脚本化 native WebView 点击流程。
2. **Codex history 仍依赖 app-server 在线**
   - `provider/history.rs` 读取 Codex history 目前仍走 `thread/list`，因此 `4500` 不在线时无法像 Claude transcript 那样离线索引。

## 按 Plan 拆分后的执行任务

> 状态说明：`✅ 已完成/已完成第一阶段` / `❌ 未完成`

### Task 1：构建标准化 Task / Session / Artifact 领域模型

**状态：✅ 已完成第一阶段收尾（优先级 P0）**

**已覆盖：**
- 基础类型已创建：`task_graph/types.rs`
- 内存 store 已创建：`task_graph/store.rs`
- session/task/artifact 查询索引已创建
- 单元测试已补齐并通过：`cargo test task_graph --manifest-path src-tauri/Cargo.toml`
- `DaemonState` 已挂载 `task_graph`

**本轮新增完成：**
- 已实现 `TaskGraphStore` JSON 持久化快照与 `load/save`
- 已支持可注入持久化路径
- `observe_task_message()` 的 task graph mutation 已接 auto-save
- 已补真实持久化 round-trip 测试与 `DaemonState` 集成测试

**剩余注意事项：**
- daemon 启动时的默认持久化路径仍待 Task 5/启动接线阶段确定
- `create_task` / `create_session` / `add_artifact` 等后续 command 入口在落地时仍需接 `save_task_graph()`
- `set_coder_session` 尚未被实际流程消费

### Task 2：补齐 Codex provider adapter、history、resume、fork、archive

**状态：✅ 已完成第一阶段（优先级 P0）**

**已覆盖：**
- Codex WS 握手、resume helper、thread id 获取逻辑已整理
- `session.rs` / `ws_client.rs` 已为 thread 生命周期处理打基础
- `provider/{mod,codex,shared}.rs` 已创建，provider adapter 骨架已落地
- 已有 `SessionRegistration` 共享 DTO
- Codex 启动时已能把 normalized session 注册进 task graph，并绑定 `thread.id`
- `codex_tests.rs` 已覆盖 session 注册、child 关系、late bind 与 launch registration

**本轮新增完成：**
- 已新增 provider DTO：
  - `ProviderHistoryEntry`
  - `ProviderHistoryPage`
  - `ProviderResumeTarget`
- 已实现 `thread/list` / `thread/fork` / `thread/archive` 的 provider 适配与 WS RPC helper
- `resume_session` 已对 Codex provider 走真实 runtime reconnect，而不是只改 normalized 指针
- 已补 provider mapping / resume target / archive 状态同步 / stable `CODEX_HOME` 测试覆盖

**仍待后续阶段完成：**
- provider history / fork / archive 还未暴露到统一前端 history picker
- 更高层的端到端 GUI 验证待 Task 8 收尾

### Task 3：补齐 Claude provider adapter、session capture、history、resume

**状态：✅ 已完成第一阶段（优先级 P1）**

**已覆盖：**
- `SessionHandle` 已新增 `transcript_path`
- Claude managed launch 现在会显式分配 `session_id`，并把 `session_id + transcript_path` 注册进 normalized task graph
- 已实现 workspace 级本地 transcript history index，按 `~/.claude/projects/<workspace-slug>/*.jsonl` 构建 provider history DTO
- `resume_session` 已对 Claude provider 走真实 runtime resume（`--resume <session_id>`），并在恢复后补齐 transcript metadata
- 已补 Claude adapter / launch argv / relative-path history slug 回归测试

**仍待后续阶段完成：**
- 当前 history index 仍基于本地 transcript 文件解析，尚未形成统一前端 history picker
- `--session-id` / `--resume` 的完整人工 GUI 回放验证待 Task 8 收尾

### Task 4：构建 Task Orchestrator 与严格 Review Gate

**状态：✅ 已完成第一阶段（优先级 P0）**

**已覆盖：**
- `active_task_id`、`preferred_auto_target()` 已接入
- 路由层开始按 task 状态偏向 lead/coder
- 基础 review gate 测试已存在

**本轮新增完成：**
- 已新增 `orchestrator` 模块与最小状态机
- review gate 已从散落逻辑收敛为独立模块
- 已补 `PendingLeadApproval` 语义
- 已修复关键错误：`reviewer -> lead done` 不再直接释放下一条 coder todo，必须 lead 显式批准
- 已补 orchestrator 测试与状态流回归测试

**仍待后续阶段完成：**
- “lead 创建 coder child session” 仍未真正落地
- task 生命周期的外部触发入口（commands/UI）仍未接入
- 每个 todo 的显式结构化建模仍未出现，目前 gate 仍是 task 级别近似实现

### Task 5：暴露 commands 与 GUI task/session/artifact 事件

**状态：✅ 已完成第一阶段（优先级 P1）**

**本轮新增完成：**
- 已拆出 `DaemonCmd` 到独立模块并新增 task 相关命令
- 已新增 Tauri task commands：
  - create/list/select task
  - get task snapshot
  - approve review
  - list session tree
  - list history
  - resume session（最小骨架）
- 已新增 GUI events：
  - `task_updated`
  - `active_task_changed`
  - `review_gate_changed`
  - `session_tree_changed`
  - `artifacts_changed`
- 已新增 DTO：
  - `TaskSnapshot`
  - `SessionTreeSnapshot`
  - `HistoryEntry`
- 已补 state snapshot / types / gui event 测试

**仍待后续阶段完成：**
- `resume_session` 目前只更新 normalized task graph 指针，不会真正重连 Claude/Codex provider
- `session_tree_changed` / `artifacts_changed` 目前已在 task commands 流程中触发，但还未全面接入 provider/session/artifact 所有写路径

### Task 6：新增前端 task store 与 task-centric UI shell

**状态：✅ 已完成第一阶段（优先级 P2）**

**本轮新增完成：**
1. 已新增 `src/stores/task-store/{index,events,types}.ts`
2. 已建立 task event reducer，并补上启动 hydration（`bootstrapTaskStore`）
3. 已在 `src/App.tsx` 挂载最小 task-centric shell
4. 已保持旧消息流兼容，未拆掉 legacy message UI
5. 已新增前端测试：`tests/task-store.test.ts`

**仍待后续阶段完成：**
- 目前还是最小 shell，未形成完整 task workspace 页面
- 尚未接入更完整的 task list / create / select 交互界面
- 仍缺少更丰富的 session/history/artifact 可视化

### Task 7：新增 session tree / history picker / artifact timeline / review badge

**状态：✅ 已完成第一阶段（优先级 P2）**

**本轮新增完成：**
1. 已新增 TaskPanel 细分组件：
   - `src/components/TaskPanel/SessionTree.tsx`
   - `src/components/TaskPanel/ArtifactTimeline.tsx`
   - `src/components/TaskPanel/HistoryPicker.tsx`
   - `src/components/TaskPanel/ReviewGateBadge.tsx`
2. 已实现 lead / coder 父子 session tree 展示，并在当前 coder 节点挂 review badge
3. 已实现 unified history picker：
   - 当前 task 已挂载历史
   - 其他 task 已映射历史
   - 外部 provider history attach 为 lead/coder
4. 已实现 artifact timeline，并将 artifact 关联回 session title
5. 已把 task context 摘要接入：
   - `TaskPanel`
   - `MessagePanel`
   - `ReplyInput`
   - `AgentStatus/CodexHeader`
6. 已新增 provider history commands / store action / attach flow，并补齐前后端验证

**仍待后续阶段完成：**
- Task 7 UI 目前已是完整第一阶段版本，但跨 provider 的手工 GUI 回放仍放到 Task 8 一并验收

### Task 8：整体验证、文档与硬化

**状态：✅ 已完成第一阶段（优先级 P2）**

**本轮新增完成：**
1. 已完成全量自动化验证：
   - `cargo test --manifest-path src-tauri/Cargo.toml`
   - `cargo test --manifest-path src-tauri/Cargo.toml daemon::`
   - `cargo test --manifest-path src-tauri/Cargo.toml provider`
   - `bun test tests/task-store.test.ts tests/task-panel-view-model.test.ts`
   - `bun run build`
2. 已完成运行时 smoke：
   - `dimweave` 继续监听 `127.0.0.1:4502`
   - `codex app-server` 继续监听 `127.0.0.1:4500`
   - `curl -fsS http://127.0.0.1:4500/readyz` 成功
3. 已更新架构与链路文档：
   - `CLAUDE.md`
   - `UPDATE.md`
   - `docs/agents/codex-chain.md`
   - `docs/agents/claude-chain.md`
   - `docs/dimweave-audit-summary.md`
4. 已同步 execution tracker 到最新实现状态

**仍待后续阶段完成：**
- 最终用户视角的 native GUI 点击式 smoke 仍建议在本机做一次人工回放

## 推荐执行顺序

### Phase A（必须先完成）
1. Task 1 收尾：补持久化
2. Task 4 收尾：补完整 orchestrator + 正确 review gate
3. Task 2：补 Codex adapter

### Phase B（打通双 provider）
4. Task 3：Claude adapter
5. Task 5：commands + GUI events

### Phase C（产品界面）
6. Task 6：task store + shell
7. Task 7：session tree + history + artifact + review badge

### Phase D（验收）
8. Task 8：整体验证与文档

## 审查中发现的当前风险

1. 原生 Tauri GUI 的完整点击式回放仍未自动化，最终用户视角的 smoke 还需要本机人工补一次。
2. Codex workspace history 仍依赖 app-server 在线；当 `4500` 不在线时，history picker 无法像 Claude transcript 一样离线索引。

## 已验证项

- `cargo test --manifest-path src-tauri/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::`
- `cargo test --manifest-path src-tauri/Cargo.toml provider`
- `bun test tests/task-store.test.ts tests/task-panel-view-model.test.ts`
- `bun run build`
- `curl -fsS http://127.0.0.1:4500/readyz`

结果：以上测试均通过。
