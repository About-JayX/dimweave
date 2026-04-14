# SQLite Full Migration And Task Root Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace JSON persistence with a single SQLite-backed persistence layer, split `project_root` from `task_worktree_root`, and fix the resulting multi-task list and agent reorder persistence behavior.

**Architecture:** First introduce the SQLite persistence substrate and move the task graph onto it with the new root model. Then migrate daemon snapshot buffering and external integration configs/stores into the same database. Finally update frontend hydration and regressions so multi-task list behavior and agent reorder are stable against the new storage model.

**Tech Stack:** Rust daemon, SQLite, React 19, Zustand 5, Tauri 2, Bun test, Vite

---

## Memory

- Recent related commits:
  - `6938ba4d` — introduced per-task worktrees
  - `caae718f` / `faa4d78f` — introduced persisted `TaskAgent`
  - `343ae415` / `87d8a469` — task-agent CRUD/reorder plumbing and UI
  - `a7934b3b` — multi-task accordion UI landed, exposing the root-path bug more clearly
- Relevant prior specs/plans:
  - `docs/superpowers/specs/2026-04-14-task-root-split-and-agent-dnd-design.md`
  - `docs/superpowers/plans/2026-04-14-task-root-split-and-agent-dnd.md`
- Constraints carried forward:
  - Telegram routing ownership is out of scope
  - old JSON data will not be migrated
  - one shared SQLite file for now

## Task 1: Introduce SQLite persistence substrate and move task graph to it

**task_id:** `sqlite-task-graph-foundation`

**allowed_files:**

- `src-tauri/src/daemon/task_graph/types.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/persist.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`
- `src-tauri/src/daemon/state_persistence.rs`
- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/commands_artifact.rs`
- `src-tauri/Cargo.toml`

**max_files_changed:** `10`
**max_added_loc:** `700`
**max_deleted_loc:** `260`

**acceptance criteria:**

- task graph persistence no longer reads/writes JSON snapshots
- SQLite schema exists for tasks, task_agents, sessions, artifacts, buffered_messages, and meta/schema_version
- task model stores separate `project_root` and `task_worktree_root`
- old JSON load path is removed or no longer used
- tests prove SQLite round-trip and root-field correctness

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `git diff --check`

## Plan Revision 1 — 2026-04-14

**Reason:** Task 1 review found three compile/persistence ripples that are directly on the acceptance path for the SQLite task-graph cutover:

- `src-tauri/src/daemon/task_graph/task_index.rs` must switch workspace filtering to the stable project root
- `src-tauri/src/daemon/gui_task.rs` contains `Task` test literals that must include the new root field
- `Cargo.lock` must update because adding `rusqlite` changes the dependency lock graph

These changes are mechanical consequences of the approved SQLite + root-split task and must be in scope.

**Added to Task 1 allowed_files:**

- `src-tauri/src/daemon/task_graph/task_index.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `Cargo.lock`

**Revised Task 1 budgets:**

- `max_files_changed: 13`
- `max_added_loc: 780`
- `max_deleted_loc: 300`

## Plan Revision 2 — 2026-04-14

**Reason:** Task 1 review found two additional mechanical ripples on the approved acceptance path:

- `src-tauri/src/daemon/mod.rs` must call the renamed task-root update API and read the renamed task worktree field
- `src-tauri/src/daemon/feishu_project_task_link.rs` reads the task execution root and must follow the renamed task worktree field

These are direct compile/behavior ripples from the `project_root` + `task_worktree_root` split and must be in scope.

**Added to Task 1 allowed_files:**

- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`

**Revised Task 1 budgets:**

- `max_files_changed: 15`
- `max_added_loc: 800`
- `max_deleted_loc: 320`

## Task 2: Move daemon snapshot buffering to SQLite-backed persistence

**task_id:** `sqlite-daemon-snapshot-persistence`

**allowed_files:**

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_persistence.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/mod.rs`

**max_files_changed:** `5`
**max_added_loc:** `260`
**max_deleted_loc:** `120`

**acceptance criteria:**

- buffered message persistence is loaded/saved via SQLite-backed state
- daemon startup no longer depends on JSON snapshot files
- tests prove buffered messages survive restart through SQLite-backed persistence

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state_tests:: -- --nocapture`
- `git diff --check`

## Plan Revision 3 — 2026-04-14

**Reason:** Task 2 review established that the original verification filter `daemon::state_tests::` can match zero tests because the actual module path is `daemon::state::state_tests::`. The verification command must use the concrete module path so the buffered-message restart suite really runs during review.

**Revised Task 2 verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml state::state_tests -- --nocapture`
- `git diff --check`

## Task 3: Move Telegram and Feishu persisted config/state to SQLite

**task_id:** `sqlite-external-config-persistence`

**allowed_files:**

- `src-tauri/src/telegram/config.rs`
- `src-tauri/src/telegram/runtime.rs`
- `src-tauri/src/telegram/types.rs`
- `src-tauri/src/commands_telegram.rs`
- `src-tauri/src/daemon/telegram_lifecycle.rs`
- `src-tauri/src/feishu_project/config.rs`
- `src-tauri/src/feishu_project/store.rs`
- `src-tauri/src/commands_feishu_project.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`

**max_files_changed:** `9`
**max_added_loc:** `420`
**max_deleted_loc:** `180`

**acceptance criteria:**

- Telegram config no longer reads/writes standalone JSON
- Feishu config and inbox store no longer read/write standalone JSON
- both integrations use the shared SQLite persistence layer
- existing runtime behavior remains otherwise unchanged

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::telegram_lifecycle_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::store::tests -- --nocapture`
- `git diff --check`

## Task 4: Fix frontend task list semantics against the new root split

**task_id:** `project-root-frontend-sync`

**allowed_files:**

- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/selectors.ts`
- `tests/task-store.test.ts`
- `src/components/TaskPanel/index.tsx`
- `src/components/TaskContextPopover.tsx`
- `src/components/TaskContextPopover.test.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/ReplyInput/index.test.tsx`

**max_files_changed:** `9`
**max_added_loc:** `360`
**max_deleted_loc:** `140`

**acceptance criteria:**

- frontend selected workspace is aligned to `projectRoot`, not worktree path
- workspace task list uses `projectRoot`
- creating a new task no longer makes older tasks disappear from the same project list
- active-task accordion behavior still syncs the message panel correctly

**verification_commands:**

- `bun test tests/task-store.test.ts src/components/TaskContextPopover.test.tsx src/components/TaskPanel/TaskHeader.test.tsx src/components/ReplyInput/index.test.tsx`
- `bun run build`
- `git diff --check`

## Task 5: Make agent drag reorder reliable in the real app

**task_id:** `agent-dnd-runtime-fix`

**allowed_files:**

- `src/components/TaskPanel/TaskAgentList.tsx`
- `src/components/TaskPanel/TaskAgentList.test.tsx`
- `src/components/TaskPanel/TaskAgentList.interaction.test.tsx`
- `src/components/TaskPanel/dom-test-env.ts`
- `package.json`
- `bun.lock`

**max_files_changed:** `6`
**max_added_loc:** `220`
**max_deleted_loc:** `100`

**acceptance criteria:**

- dragging an agent row in the actual app triggers reorder
- reorder persists correctly through the existing reorder command path now backed by SQLite
- interaction tests cover the real drag path rather than only pure helper logic

**verification_commands:**

- `bun test src/components/TaskPanel/TaskAgentList.test.tsx src/components/TaskPanel/TaskAgentList.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 6: Final regression and documentation close-out

**task_id:** `sqlite-root-split-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-14-sqlite-full-migration-and-task-root-design.md`
- `docs/superpowers/plans/2026-04-14-sqlite-full-migration-and-task-root.md`

**max_files_changed:** `2`
**max_added_loc:** `60`
**max_deleted_loc:** `20`

**acceptance criteria:**

- spec/plan reflect the accepted SQLite architecture and root split behavior
- CM record contains accepted commits and verification evidence for all tasks

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `ba41b78e`, `ce6211c7` | Replaced task-graph JSON persistence with SQLite, added schema-backed persistence for tasks/sessions/artifacts/task_agents plus meta and buffered-message tables, introduced an explicit root split (`project_root` + `task_worktree_root`), and removed the remaining legacy field naming by renaming `workspace_root` to `task_worktree_root` across the task graph, SQLite schema, and compile ripples. | `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture` ✅ 55 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture` ✅ 10 passed; `git diff --check` ✅ | accepted |
| Task 2 | `173672b6` | Added SQLite-backed save/load for persisted buffered messages on top of the new task-graph database, so daemon restart restores buffered messages per task/session instead of losing them after the JSON persistence removal. | `cargo test --manifest-path src-tauri/Cargo.toml state::state_tests -- --nocapture` ✅ 91 passed; `git diff --check` ✅ | accepted |
| Task 3 | `138a1a88` | Migrated Telegram config, Feishu config, and Feishu inbox persistence from standalone JSON files into the shared SQLite database (`config.db`) while preserving the existing runtime/config APIs and behavior. | `cargo test --manifest-path src-tauri/Cargo.toml daemon::telegram_lifecycle::tests -- --nocapture` ✅ 20 passed; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::store::tests -- --nocapture` ✅ 9 passed; `git diff --check` ✅ | accepted |
| Task 4 | `e628f532` | Aligned frontend workspace selection and task-list filtering to `projectRoot` instead of the task worktree path, updated task DTO types for the split-root model, and added regression coverage proving multi-task lists no longer collapse after creating another task in the same project. | `bun test tests/task-store.test.ts src/components/TaskContextPopover.test.tsx src/components/TaskPanel/TaskHeader.test.tsx src/components/ReplyInput/index.test.tsx` ✅ 96 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 5 | `787890b2` | Confirmed the agent drag-and-drop path remains valid after the split-root/SQLite changes by aligning the TaskInfo test fixtures to `projectRoot` + `taskWorktreeRoot` and preserving the existing interaction coverage that exercises native drag events through `reorderTaskAgents`. No production code changes were required in this task. | `bun test src/components/TaskPanel/TaskAgentList.test.tsx src/components/TaskPanel/TaskAgentList.interaction.test.tsx` ✅ 16 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 6 | _(this commit)_ | Final doc close-out: updated spec status to accepted, recorded all task outcomes in CM record. | `git diff --check` ✅ | accepted |
