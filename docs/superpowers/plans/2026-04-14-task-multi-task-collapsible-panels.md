# Task Multi-Task Collapsible Panels Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Change the task pane from a single active-task inspector into a multi-task accordion list where one task panel is expanded at a time and active-task changes synchronize the message panel.

**Architecture:** Reuse the current task summary card as each panel header, derive the visible task list from workspace tasks in the store, and treat `activeTaskId` as the single expanded panel id so the message panel stays synchronized without introducing parallel expansion state.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Vite

---

## Memory

- Recent related commits:
  - `482e21fd` — removed legacy singleton task header fallback badges
  - `87d8a469` — preserved `agentId` / `displayName` in `Edit Task`
  - `977c8907` — allowed empty-task create and restored `Edit Task`
  - `343ae415` — added task-agent CRUD/reorder plumbing and task-pane agent list UI
  - `d24c1ced` / `fc426624` — hydrated `task_agents[]` into frontend state and switched reply targeting to dynamic roles
- Relevant prior plan:
  - `docs/superpowers/plans/2026-04-14-task-agent-identity-role-broadcast.md`
- Constraints carried forward:
  - `task_agents[]` remains the sole truth source for task-agent UI
  - this follow-up changes pane structure, not task-agent routing/model behavior
  - `activeTaskId` must remain the message-panel sync source

## Task 1: Add workspace task-list selectors for accordion rendering

**task_id:** `workspace-task-list-selectors`

**allowed_files:**

- `src/stores/task-store/selectors.ts`
- `src/stores/task-store/types.ts`
- `tests/task-store.test.ts`

**max_files_changed:** `3`
**max_added_loc:** `180`
**max_deleted_loc:** `60`

**acceptance criteria:**

- selectors exist for selected-workspace task list ordered newest-first
- active task expansion can be derived without introducing a second truth source
- tests prove multi-task list ordering and active-task sync assumptions

**verification_commands:**

- `bun test tests/task-store.test.ts`
- `git diff --check`

## Task 2: Replace single-task pane with accordion list

**task_id:** `task-pane-accordion-list`

**allowed_files:**

- `src/components/TaskContextPopover.tsx`
- `src/components/TaskContextPopover.test.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/view-model.ts`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `6`
**max_added_loc:** `420`
**max_deleted_loc:** `180`

**acceptance criteria:**

- task pane renders a list of task panels for the selected workspace
- summary card is used as the collapsed header
- only the active task is expanded
- clicking a collapsed task makes it active and expands it
- message panel sync continues through `activeTaskId`

**verification_commands:**

- `bun test src/components/TaskContextPopover.test.tsx src/components/TaskPanel/TaskHeader.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Make New Task insert and expand within the list

**task_id:** `new-task-list-integration`

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/stores/task-store/index.ts`
- `tests/task-store.test.ts`

**max_files_changed:** `5`
**max_added_loc:** `260`
**max_deleted_loc:** `120`

**acceptance criteria:**

- creating a task does not make the previous tasks disappear
- newly created task appears in the list and becomes the expanded/active task
- existing tasks remain in the workspace list
- tests prove multi-task creation behavior

**verification_commands:**

- `bun test tests/task-store.test.ts src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `bun run build`
- `git diff --check`

## Task 4: Final regression and doc close-out

**task_id:** `task-pane-accordion-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-14-task-multi-task-collapsible-panels-design.md`
- `docs/superpowers/plans/2026-04-14-task-multi-task-collapsible-panels.md`
- `src/components/ReplyInput/index.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `120`
**max_deleted_loc:** `40`

**acceptance criteria:**

- docs reflect accepted accordion behavior
- regression test proves active-task change still syncs reply/message behavior
- CM record is filled with accepted commits and verification evidence

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
