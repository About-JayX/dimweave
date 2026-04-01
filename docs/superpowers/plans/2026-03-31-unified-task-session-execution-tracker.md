# Unified Task/Session Architecture Execution Tracker

关联文档：
- Plan：`docs/superpowers/plans/2026-03-31-unified-task-session-architecture.md`
- Spec：`docs/superpowers/specs/2026-03-31-unified-task-session-architecture-design.md`

## 当前审查结论

截至当前执行点，Task 1/2/3/4/5/6 已完成第一阶段落地，系统已经具备：
- 标准化 task/session/artifact 持久化模型
- Codex provider 的 history/fork/archive/resume 基础能力
- Claude provider 的 session metadata capture、本地 transcript history index 与 runtime resume
- 基础 orchestrator / review gate / task store / 最小 task shell

当前主缺口已经收敛到：
- Task 8：全流程验证、文档收尾与最终交付证据

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

### 关键缺口

1. **没有真正落地持久化存储**
   - 当前 `TaskGraphStore` 仅是内存结构，未实现本地持久化。
2. **没有 provider adapter 层**
   - `provider/{mod,codex,claude,shared}.rs` 尚不存在。
3. **没有 orchestrator 模块**
   - `orchestrator/{mod,task_flow,review_gate}.rs` 尚不存在。
4. **review gate 逻辑与 spec 不完全一致**
   - 目前 `reviewer -> lead` 完成后会直接解除阻塞，**没有 lead 最终批准关卡**。
5. **没有 commands / GUI events / frontend task store / task UI**
   - Task 5~7 基本未开始。

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

**状态：❌ 未完成（优先级 P2）**

**已有最小进展：**
- 已补充 execution tracker 与 Claude 逆向/链路文档
- 已完成多组后端与前端局部测试验证

**本阶段仍未完成：**
1. 全量后端/前端测试验收
2. 全流程手工验证
3. 全量架构与交付文档收尾
4. 汇总最终交付证据

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

## 审查中发现的高优先级风险

1. 当前 task graph 与 provider session 仍是松耦合，导致“统一 task/session 产品模型”还没有真正闭环。
2. Claude provider adapter / history / resume 仍未开始，双 provider 架构还不完整。
3. 前端虽然已有最小 task shell，但 session tree / history picker / artifact timeline 仍未完成，产品形态还不完整。

## 已验证项

- `cargo test task_graph --manifest-path src-tauri/Cargo.toml`
- `cargo test routing --manifest-path src-tauri/Cargo.toml`
- `cargo test state_tests --manifest-path src-tauri/Cargo.toml`
- `cargo test state_task_snapshot_tests --manifest-path src-tauri/Cargo.toml`
- `bun test tests/task-store.test.ts tests/message-panel-view-model.test.ts`

结果：以上测试均通过。
