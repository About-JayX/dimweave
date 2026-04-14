# Task Pane Card-Only Simplification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce the left task pane to a compact list of task cards by removing the separate `Agents`, `Sessions`, and `Artifacts` panels while preserving multi-task selection and message-panel sync.

**Architecture:** Reuse `TaskHeader` as the single visible task card for each task, remove the task-pane render path for `TaskAgentList`, `SessionTree`, and `ArtifactTimeline`, and keep `activeTaskId` as the only selection state so the right-side message panel continues to follow task changes.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Vite

---

## Memory

- Recent related commits:
  - `e628f532` — aligned frontend workspace selection to `projectRoot`
  - `968aea04` / `1f635f4c` — converted the pane into a multi-task accordion and fixed task-scoped header agents
  - `45d67569` — added reply-target regression proving `activeTaskId` still syncs reply behavior
  - `7c3d3337` — finalized SQLite migration and current `main` baseline
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-14-task-multi-task-collapsible-panels.md`
  - `docs/superpowers/plans/2026-04-14-task-agent-identity-role-broadcast.md`
  - `docs/superpowers/plans/2026-04-14-sqlite-full-migration-and-task-root.md`
- Lessons carried forward:
  - `task_agents[]` remains the only task-agent truth source
  - `activeTaskId` remains the only task selection / message sync source
  - UI simplification must not reintroduce singleton lead/coder fallback behavior
  - this change removes task-pane surfaces only; it does not delete session/artifact persistence or daemon events

## Task 1: Collapse the task pane to card-only rendering

**task_id:** `task-pane-card-only-render`

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/view-model.ts`

**max_files_changed:** `3`
**max_added_loc:** `180`
**max_deleted_loc:** `140`

**acceptance criteria:**

- `TaskPanel` no longer renders `TaskAgentList`
- `TaskPanel` no longer renders `SessionTree`
- `TaskPanel` no longer renders `ArtifactTimeline`
- active and inactive tasks are both represented only by the task card surface
- `New Task` remains available at list level

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `bun run build`
- `git diff --check`

## Plan Revision 1 — 2026-04-15

**Reason:** Task 1 requires a focused regression test to lock the card-only surface contract while removing the `Agents`, `Sessions`, and `Artifacts` render path. That coverage lives in `TaskHeader.test.tsx`, so the test file must be in scope for the required TDD workflow.

**Added to Task 1 allowed_files:**

- `src/components/TaskPanel/TaskHeader.test.tsx`

**Revised Task 1 budgets:**

- `max_files_changed: 4`
- `max_added_loc: 200`
- `max_deleted_loc: 140`

## Task 2: Preserve card-level task management and selection behavior

**task_id:** `task-card-selection-and-edit-flow`

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskContextPopover.tsx`
- `src/components/TaskContextPopover.test.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/ReplyInput/index.test.tsx`

**max_files_changed:** `6`
**max_added_loc:** `220`
**max_deleted_loc:** `90`

**acceptance criteria:**

- clicking a task card still sets `activeTaskId`
- right-side reply/message targeting still follows the newly active task
- `Edit Task` remains accessible from the task card
- no `Sessions` / `Artifacts` copy remains visible in the task pane

**verification_commands:**

- `bun test src/components/TaskContextPopover.test.tsx src/components/TaskPanel/TaskHeader.test.tsx src/components/ReplyInput/index.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final regression and doc close-out

**task_id:** `task-pane-card-only-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-pane-card-only-design.md`
- `docs/superpowers/plans/2026-04-15-task-pane-card-only.md`

**max_files_changed:** `2`
**max_added_loc:** `40`
**max_deleted_loc:** `20`

**acceptance criteria:**

- spec/plan reflect the accepted card-only behavior
- CM record captures accepted commits and verification evidence

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `2769cb31` | Collapsed `TaskPanel` to card-only rendering by removing the task-pane render path for `TaskAgentList`, `SessionTree`, and `ArtifactTimeline`. Active and inactive tasks now both render only through the task card surface, while `New Task` remains at list level. | `bun test src/components/TaskPanel/TaskHeader.test.tsx` ✅ 11 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `d041dd4a` | Added explicit contract coverage proving collapsed cards remain keyboard-selectable, the active card remains the edit surface, the task pane no longer renders `Sessions` or `Artifacts` headings, and right-side reply targeting still follows `activeTaskId` without any additional production changes. | `bun test src/components/TaskContextPopover.test.tsx src/components/TaskPanel/TaskHeader.test.tsx src/components/ReplyInput/index.test.tsx` ✅ 28 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 3 | _pending_ | Finalize docs and CM evidence for the accepted task-pane simplification. | _pending_ | pending |
