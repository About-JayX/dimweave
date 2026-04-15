# Edit Dialog DnD-Kit Sort Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the unreliable native edit-dialog drag implementation with `dnd-kit` so agent reordering works in the real app.

**Architecture:** Add `dnd-kit` as the sortable interaction layer, move edit-mode rows in `TaskSetupDialog` to handle-based sortable items with stable item ids, and keep the existing `reorderTaskAgents` save path as the persistence mechanism.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Vite, `dnd-kit`

---

## Memory

- Recent related commits:
  - `1dcb7895` — first edit-dialog drag reorder implementation using native drag events
  - `7d420733` — task card chrome polish
  - `2769cb31` / `d041dd4a` — task pane simplified to card-only while preserving active-task sync
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-task-card-polish-and-agent-edit.md`
  - `docs/superpowers/plans/2026-04-15-task-pane-card-only.md`
- Lessons carried forward:
  - `Edit Task` is now the sole agent-management entry point in the pane
  - dialog-only synthetic drag tests are not enough if they do not match real WebView constraints
  - ordering must still persist through the existing `reorderTaskAgents` path

## Task 1: Replace native edit-dialog drag with `dnd-kit`

**task_id:** `edit-dialog-dnd-kit-runtime-fix`

**allowed_files:**

- `package.json`
- `bun.lock`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/TaskPanel/dom-test-env.ts`

**max_files_changed:** `6`
**max_added_loc:** `280`
**max_deleted_loc:** `120`

**acceptance criteria:**

- edit mode uses `dnd-kit` sortable interaction instead of native `draggable` DOM events
- drag interaction is started from the grip handle, not the full row
- interaction tests cover the sortable reorder path and ordered submit payload
- create mode behavior remains intact

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Final regression and doc close-out

**task_id:** `edit-dialog-dnd-kit-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-edit-dialog-dnd-kit-sort-fix-design.md`
- `docs/superpowers/plans/2026-04-15-edit-dialog-dnd-kit-sort-fix.md`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `50`
**max_deleted_loc:** `20`

**acceptance criteria:**

- docs reflect the accepted `dnd-kit`-based fix
- CM record contains accepted commits and verification evidence
- final regression still proves task card pills reflect persisted store order

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | _pending_ | Replace native edit-dialog drag with `dnd-kit` sortable behavior and keep ordered save payloads intact. | _pending_ | pending |
| Task 2 | _pending_ | Finalize docs and regression evidence for the `dnd-kit` sort fix. | _pending_ | pending |
