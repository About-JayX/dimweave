# Task Card Agent Status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make each task-card agent pill reflect daemon-owned per-agent online status instead of always rendering a gray dot.

**Architecture:** Extend task-context DTOs/events with per-agent runtime status keyed by `agentId`, store that data by `taskId` in the frontend task store, and have `TaskHeader` color each persisted agent pill from that daemon-owned map.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Rust, Bun test, Cargo check, Vite

---

## Memory

- Recent related commits:
  - `16e5bc48` — finalized edit/delete/connect revision on current `main`
  - `1f635f4c` — fixed `TaskHeader` to read agents from its own task rather than the active task
  - `d24c1ced` / `737746b5` — hydrated task agents and task-scoped frontend state from daemon snapshots
  - `b6b62e2b` / `64954a8d` — task card and edit dialog now manage agent-bound actions, exposing the need for pill-level runtime truth
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-edit-task-connect-and-delete-revision.md`
  - `docs/superpowers/plans/2026-04-15-task-pane-card-only.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-list-dialog-unify.md`
- Constraints carried forward:
  - runtime status remains daemon-owned
  - do not infer multi-agent status from `lead/coder` summary flags
  - keep task-card pill text/order unchanged

## Baseline

- Worktree: `.worktrees/task-card-agent-status`
- Baseline verification before changes:
  - `bun test src/components/TaskPanel/TaskHeader.test.tsx tests/task-store.test.ts` ✅ 101 passed
  - `cargo build -p dimweave-bridge` ✅
  - `cargo check --manifest-path src-tauri/Cargo.toml` ✅
  - `bun run build` ✅

## Task 1: Add daemon-backed per-agent task status data flow

**task_id:** `task-agent-status-dto-store`

**allowed_files:**

- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/events.ts`
- `src/stores/task-store/index.ts`
- `tests/task-store.test.ts`

**max_files_changed:** `7`
**max_added_loc:** `220`
**max_deleted_loc:** `80`

**acceptance criteria:**

- task snapshot / task context includes per-agent runtime status keyed by `agentId`
- frontend store hydrates and updates per-task agent status from daemon-owned data
- no frontend logic relies on `leadOnline/coderOnline` to color individual pills

**verification_commands:**

- `bun test tests/task-store.test.ts`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `bun run build`
- `git diff --check`

## Task 2: Render task-card pill dots from daemon-owned agent status

**task_id:** `task-header-agent-status-render`

**allowed_files:**

- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `2`
**max_added_loc:** `120`
**max_deleted_loc:** `40`

**acceptance criteria:**

- each task-card pill looks up status by `agentId`
- online agent pills render a green dot
- offline agent pills render a gray dot
- two same-provider agents can render different dot states independently

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final docs and CM close-out

**task_id:** `task-card-agent-status-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-card-agent-status-design.md`
- `docs/superpowers/plans/2026-04-15-task-card-agent-status.md`

**max_files_changed:** `2`
**max_added_loc:** `30`
**max_deleted_loc:** `10`

**acceptance criteria:**

- docs reflect the accepted daemon-backed per-agent pill status behavior
- CM record contains real commit hashes and verification evidence

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `e817eeaa` | Added daemon-backed per-agent runtime status to task snapshots and task context events, hydrated that data into frontend store state by `taskId`, and added store tests proving multi-agent independent status handling and cleanup. | `bun test tests/task-store.test.ts` ✅ 85 passed; `cargo build -p dimweave-bridge` ✅; `cargo check --manifest-path src-tauri/Cargo.toml` ✅; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `ccd79531` | Updated `TaskHeader` to color each agent pill dot from daemon-backed per-agent runtime status by `agentId`, including independent same-provider agent states, with focused task-header regression coverage. | `bun test src/components/TaskPanel/TaskHeader.test.tsx` ✅ 28 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 3 | not started | Execution has not started yet. | No task-local verification yet. | not started |
