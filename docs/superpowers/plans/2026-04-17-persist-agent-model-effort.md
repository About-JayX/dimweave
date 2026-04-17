# 持久化 TaskAgent 的 model / effort 字段

## 目标

Edit Task 对话框在 edit 模式下回填 Model 和 Effort 两个下拉。

## 根因

- `TaskAgent` 表只存 `{agent_id, task_id, provider, role, display_name, order, created_at, updated_at}`，schema version 1
- Create/Edit 提交时，`model` 和 `effort` 存在于 dialog 的 `AgentDef` 局部 state，但 `addTaskAgent / updateTaskAgent` 只接受 `provider/role/displayName`
- TaskPanel 生成 `initialAgents` 时只拷贝 `provider/role/agentId/displayName`
- Session 字段不在本次范围，前端保持"每次从 provider history 里挑"

## 改动清单（按依赖顺序）

### Step 1 — Rust 类型扩展
- `src-tauri/src/daemon/task_graph/types.rs::TaskAgent` — 新增 `model: Option<String>` + `effort: Option<String>`（带 `#[serde(skip_serializing_if = "Option::is_none")]`）

### Step 2 — SQLite 迁移
- `src-tauri/src/daemon/task_graph/persist.rs::init_schema`
  - `task_agents` 表新增两列 `model TEXT` / `effort TEXT`
  - `SCHEMA_VERSION: 1 → 2`
  - 迁移逻辑：旧库 `ALTER TABLE task_agents ADD COLUMN model TEXT; ADD COLUMN effort TEXT;`（幂等写法，先读 `meta.schema_version`，<2 时执行 ALTER 再回写 version=2）
- `save_to_db` INSERT 调整为 10 个 `?` 占位
- `load_from_db` SELECT 补齐 model/effort 列

### Step 3 — Store CRUD
- `src-tauri/src/daemon/task_graph/store.rs`
  - `add_task_agent(task_id, provider, role, model, effort)` — 把 model/effort 直接写进 `TaskAgent`
  - `update_task_agent(agent_id, provider, role, display_name, model, effort)` — 四项一起写
  - `migrate_legacy_agents` 生成的 agent 两项都是 None（兼容）

### Step 4 — DaemonCmd + 命令
- `src-tauri/src/daemon/cmd.rs` — `AddTaskAgent` / `UpdateTaskAgent` 新增 `model: Option<String>` + `effort: Option<String>`
- `src-tauri/src/daemon/mod.rs` — 把新参数透传到 store CRUD
- `src-tauri/src/commands_task.rs::daemon_add_task_agent` / `daemon_update_task_agent` — 接受 `model/effort`

### Step 5 — TS 类型 + store
- `src/stores/task-store/types.ts::TaskAgentInfo` — 新增 `model?: string | null` + `effort?: string | null`
- `src/stores/task-store/types.ts` 的 action 签名也要跟上
- `src/stores/task-store/index.ts::addTaskAgent / updateTaskAgent` — 多传 model/effort 到 invoke

### Step 6 — TaskPanel 接入
- `src/components/TaskPanel/index.tsx`
  - `handleSetupSubmit` / `handleEditSubmit` 把 `def.model/def.effort` 传给 `addTaskAgent/updateTaskAgent`
  - `initialAgents` 的 map 加 `model: a.model ?? undefined, effort: a.effort ?? undefined`
  - 删掉已无用的 edit 调试 `console.log`

### Step 7 — TaskSetupDialog 清理
- 删除两处调试 `console.log("[TaskSetupDialog render]" / "[TaskSetupDialog state]")`

### Step 8 — 测试
- `src-tauri/src/daemon/task_graph/tests.rs`
  - `add_task_agent_persists_model_and_effort`
  - `update_task_agent_updates_model_and_effort`
- `src-tauri/src/daemon/task_graph/persist_tests.rs`（或 tests.rs）
  - `save_and_load_preserves_model_effort`
  - `legacy_schema_v1_migrates_to_v2_with_null_columns`

## 不做的事
- 不持久化 `historyAction` / session 绑定（Session 字段仍保持"每次挑"）
- 不改 `migrate_legacy_agents` 的生成逻辑（仍 None）
- 不动 Claude/Codex launch 链路 —— launch 用 `buildDraftConfigFromDef(agent)` 现在拿到的 def 已带 model/effort，无需修改

## 验证
- `cargo test -p dimweave`
- `bun run build` / `bun x tsc --noEmit -p tsconfig.app.json`
- 手工：创建 task，配置 claude coder 选 model=Opus / effort=High → 保存 → 点 edit → 两个下拉回填

## Commit 规划
1. `refactor(task_graph): add model/effort columns to TaskAgent schema v2`
2. `feat(task_graph): persist agent model/effort through daemon CRUD`
3. `feat(ui): backfill model/effort in edit task dialog`

## CM 回填区
<!-- 实施完成后回填 commit hash + 摘要 -->
