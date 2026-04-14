# Task Root Split And Agent DnD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split stable project-root vs task worktree semantics so multi-task lists stop collapsing, and make agent drag reorder reliable in the real app.

**Architecture:** Add explicit task root fields to the task graph and frontend DTO/state so project grouping never depends on worktree paths. Then fix the agent drag interaction path with real component-level coverage while keeping the existing reorder persistence command.

**Tech Stack:** Rust daemon/task graph, React 19, Zustand 5, Tauri 2, Bun test, Vite

---

## Memory

- Recent related commits:
  - `6938ba4d` — added per-task git worktree creation and task runtime
  - `caae718f` / `faa4d78f` — introduced persisted `TaskAgent`
  - `343ae415` / `87d8a469` — added task-agent CRUD/reorder UI and preserved edit identity
  - `a7934b3b` — landed multi-task accordion list on `main`
- Relevant prior plan:
  - `docs/superpowers/plans/2026-04-14-task-multi-task-collapsible-panels.md`
- Constraints carried forward:
  - Telegram is out of scope
  - keep task ordering newest-first by creation time
  - task expansion still follows `activeTaskId`

## Task 1: Split task root semantics in backend and snapshots

**task_id:** `task-root-split-backend`

**allowed_files:**

- `src-tauri/src/daemon/task_graph/types.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/persist.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/commands_artifact.rs`

**max_files_changed:** `10`
**max_added_loc:** `520`
**max_deleted_loc:** `180`

**acceptance criteria:**

- task model has explicit `project_root` and `task_worktree_root`
- task creation stores stable project root and separate worktree root
- persisted snapshots round-trip both fields
- task snapshot DTO exposes both fields needed by the frontend
- regression tests prove multiple tasks from the same project remain grouped by project root

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `git diff --check`

## Task 2: Stop frontend workspace state from being overwritten by worktree path

**task_id:** `project-root-frontend-sync`

**allowed_files:**

- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/selectors.ts`
- `tests/task-store.test.ts`
- `src/components/TaskPanel/index.tsx`

**max_files_changed:** `5`
**max_added_loc:** `260`
**max_deleted_loc:** `100`

**acceptance criteria:**

- selected workspace is derived from/stays aligned to `project_root`
- workspace task list filtering uses `project_root`
- creating a new task preserves older tasks from the same project in the visible list
- tests prove the new task no longer overwrites the old list

**verification_commands:**

- `bun test tests/task-store.test.ts`
- `bun run build`
- `git diff --check`

## Task 3: Make agent drag-and-drop reliable in the real app

**task_id:** `agent-dnd-runtime-fix`

**allowed_files:**

- `src/components/TaskPanel/TaskAgentList.tsx`
- `src/components/TaskPanel/TaskAgentList.test.tsx`
- `src/components/TaskPanel/TaskAgentList.interaction.test.tsx`
- `src/components/TaskPanel/dom-test-env.ts`
- `package.json`
- `bun.lock`

**max_files_changed:** `6`
**max_added_loc:** `260`
**max_deleted_loc:** `120`

**acceptance criteria:**

- dragging an agent row in the actual app triggers reorder
- reorder still persists through the existing `reorderTaskAgents` action/command path
- interaction tests cover the real drag contract instead of only pure helper logic
- no change to task-agent data model or reorder command semantics

**verification_commands:**

- `bun test src/components/TaskPanel/TaskAgentList.test.tsx src/components/TaskPanel/TaskAgentList.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 4: Final regression/doc close-out

**task_id:** `task-root-split-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-14-task-root-split-and-agent-dnd-design.md`
- `docs/superpowers/plans/2026-04-14-task-root-split-and-agent-dnd.md`
- `src/components/ReplyInput/index.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `120`
**max_deleted_loc:** `40`

**acceptance criteria:**

- docs reflect accepted root-split + drag-fix behavior
- regression test proves active-task reply sync still works after the root split
- CM record is filled with accepted commit hashes and verification evidence

**verification_commands:**

- `bun test src/components/ReplyInput/index.test.tsx`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | _pending_ | _pending_ | _pending_ | pending |
| Task 2 | _pending_ | _pending_ | _pending_ | pending |
| Task 3 | _pending_ | _pending_ | _pending_ | pending |
| Task 4 | _pending_ | _pending_ | _pending_ | pending |
