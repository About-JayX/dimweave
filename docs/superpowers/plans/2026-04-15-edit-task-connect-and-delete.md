# Edit Task Connect And Delete Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete task management by adding `Save & Connect` to edit mode and enabling confirmed task deletion from both the task card and the edit dialog, with automatic disconnect-before-delete behavior.

**Architecture:** Reuse the existing create-mode launch/resume helpers for edit-mode `Save & Connect`. Add a backend `daemon_delete_task` flow plus a frontend `deleteTask` store action so deletion becomes task-scoped and authoritative: stop bound providers for that task, cascade task-state removal, persist, and advance the active task selection to the next remaining task in the workspace list.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Rust, Bun test, Cargo test, Vite

---

## Memory

- Recent related commits:
  - `653208d5` — cleaned up dialog model/effort option assembly without touching trigger styling
  - `561d8ac0` — unified dialog trigger styling and compact session trigger behavior
  - `c456d7af` — wired live Codex model data into the dialog caller path
  - `9c744547` — moved dialog controls to shared `CyberSelect`
  - `2769cb31` / `d041dd4a` — collapsed the task pane to card-only and kept task-card selection/edit behavior
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-task-pane-card-only.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-list-dialog-unify.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-live-fix.md`
  - `docs/superpowers/plans/2026-04-15-task-setup-dialog-option-cleanup.md`
- Constraints carried forward:
  - do not redesign dialog layout or trigger chrome again
  - edit mode must reuse existing launch/resume semantics rather than inventing a second connection path
  - task cards remain the primary task-management surface in the pane
  - deletion must be task-scoped and must not leave orphaned task agents/sessions/artifacts behind

## Baseline

- Worktree: `.worktrees/edit-connect-delete`
- Baseline verification before changes:
  - `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx src/components/TaskPanel/TaskHeader.test.tsx` ✅ 72 passed
  - `bun run build` ✅

## Task 1: Add backend task deletion command and store cleanup path

**task_id:** `task-delete-backend-store`

**allowed_files:**

- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/commands_task.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `tests/task-store.test.ts`

**max_files_changed:** `9`
**max_added_loc:** `320`
**max_deleted_loc:** `100`

**acceptance criteria:**

- frontend has a `deleteTask(taskId)` action
- backend exposes `daemon_delete_task`
- deleting a task disconnects task-bound Claude/Codex providers before removal
- deleting a task cascades removal of task-scoped task agents, sessions, artifacts, provider summary, and reply target state
- deleting the active task selects the next remaining task in the same workspace list order, or clears active task if none remain

**verification_commands:**

- `bun test tests/task-store.test.ts`
- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `bun run build`
- `git diff --check`

## Task 2: Add edit-mode Save & Connect and delete UI entry points

**task_id:** `task-edit-dialog-connect-and-delete-ui`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/TaskPanel/index.tsx`

**max_files_changed:** `6`
**max_added_loc:** `260`
**max_deleted_loc:** `90`

**acceptance criteria:**

- edit dialog footer shows `Delete Task`, `Save`, and `Save & Connect`
- `Save` keeps pure persistence behavior
- `Save & Connect` first saves, then only connects providers still present in the saved agent list
- task card exposes a delete entry in addition to edit
- both task-card and dialog delete entries require secondary confirmation before deleteTask runs

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx src/components/TaskPanel/TaskHeader.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final docs and CM close-out

**task_id:** `task-edit-connect-delete-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-edit-task-connect-and-delete-design.md`
- `docs/superpowers/plans/2026-04-15-edit-task-connect-and-delete.md`

**max_files_changed:** `2`
**max_added_loc:** `30`
**max_deleted_loc:** `10`

**acceptance criteria:**

- docs reflect the accepted edit-connect and delete behavior
- CM record contains real commit hashes and verification evidence

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `414edabd` | Added backend/frontend task deletion plumbing with a new `daemon_delete_task` command, task-graph cascade removal, task-scoped runtime disconnect-before-delete handling, and a `deleteTask(taskId)` store action that cleans task-scoped frontend state and falls back active selection to the next task in the same workspace list order. | `bun test tests/task-store.test.ts` ✅ 78 passed; `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture` ✅ 58 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | not started | Execution has not started yet. | No task-local verification yet. | not started |
| Task 3 | not started | Execution has not started yet. | No task-local verification yet. | not started |
