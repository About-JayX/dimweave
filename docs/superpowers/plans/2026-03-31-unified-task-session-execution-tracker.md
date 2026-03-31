# Unified Task/Session Architecture Execution Tracker

关联文档：
- Plan：`docs/superpowers/plans/2026-03-31-unified-task-session-architecture.md`
- Spec：`docs/superpowers/specs/2026-03-31-unified-task-session-architecture-design.md`

## 当前审查结论

本次变更已完成一部分底层铺垫，但整体仍处于“Task 1 已大体落地、Task 4 做了早期路由试探、其余任务大多未开始”的状态。

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

**状态：❌ 未完成（已完成第一阶段基础接入，优先级 P0）**

**已覆盖：**
- Codex WS 握手、resume helper、thread id 获取逻辑已整理
- `session.rs` / `ws_client.rs` 已为 thread 生命周期处理打基础

**未完成：**
- `provider/codex.rs` 不存在
- 没有统一 DTO 映射层
- 没有 `thread/list` / `thread/fork` / `thread/archive` 适配
- Codex `thread.id` 尚未标准化落入 task graph session 记录
- 没有 `codex_tests.rs`

**本阶段子任务：**
1. 新建 provider adapter 目录与共享 DTO
2. 把 Codex 启动得到的 `thread.id` 绑定到 normalized session
3. 实现 list/resume/fork/archive 适配接口
4. 补测试覆盖 thread 注册与 DTO 映射

### Task 3：补齐 Claude provider adapter、session capture、history、resume

**状态：❌ 未完成（优先级 P1）**

**未完成：**
- `provider/claude.rs` 不存在
- Claude session metadata capture 未接入 task graph
- 本地 history index/resume 入口未实现
- 无 `claude_tests.rs`

**本阶段子任务：**
1. 设计 Claude session 元数据采集点
2. 建立 workspace 级本地 history index
3. 实现 normalized session 映射与 resume 流程
4. 增加测试

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

**状态：❌ 未完成（仅有最小 TaskPanel 雏形，优先级 P2）**

**已有最小进展：**
- 已新增 `src/components/TaskPanel/index.tsx`
- 当前可显示 active task、task status、review status、session 数量

**本阶段仍未完成：**
1. TaskPanel 细分组件拆分
2. lead / coder 父子 session tree 正式展示
3. provider-agnostic history picker
4. artifact timeline
5. 更完整的 review badge / gate 可视化

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
