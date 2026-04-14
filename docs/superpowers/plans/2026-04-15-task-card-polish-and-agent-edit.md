# Task Card Polish And Agent Edit Dialog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the task card visually lighter and move full agent management, including ordering, into the `Edit Task` dialog.

**Architecture:** Keep `TaskHeader` as the only always-visible task surface, move card chrome to a compact upper-right/lower-right layout, and extend edit-mode `TaskSetupDialog` plus the existing submit path so add/update/remove/reorder all happen in one ordered save flow.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Vite

---

## Memory

- Recent related commits:
  - `2769cb31` — removed `Agents`, `Sessions`, and `Artifacts` from the live task-pane render path
  - `d041dd4a` — locked the card-only selection and edit-flow contract in tests
  - `454ee305` — fixed dialog shell layout with dedicated inner scroll region and fixed footer
  - `87d8a469` — preserved `agentId` / `displayName` through edit flows
  - `e2500da4` / `977c8907` — earlier task-agent CRUD/reorder UI work and edit-flow restoration
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-task-pane-card-only.md`
  - `docs/superpowers/plans/2026-04-14-task-multi-task-collapsible-panels.md`
  - `docs/superpowers/plans/2026-04-14-task-agent-identity-role-broadcast.md`
- Constraints carried forward:
  - `activeTaskId` remains the only task selection truth source
  - task cards stay compact; inline agent management must not return to the list surface
  - `Edit Task` is now the sole agent-management entry point inside the pane
  - `Sessions` / `Artifacts` stay removed from the task pane

## Task 1: Polish task card chrome

**task_id:** `task-card-chrome-polish`

**allowed_files:**

- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `2`
**max_added_loc:** `120`
**max_deleted_loc:** `60`

**acceptance criteria:**

- `Edit Task` becomes icon-only in the upper-right corner
- the status badge is visually smaller and anchored in the lower-right area
- task card title, id, save indicator, and agent pills remain intact
- tests cover the icon-only edit affordance and compact status placement contract

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Upgrade edit dialog into full agent manager with ordering

**task_id:** `edit-dialog-agent-manager`

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/TaskPanel/dom-test-env.ts`

**max_files_changed:** `5`
**max_added_loc:** `320`
**max_deleted_loc:** `140`

**acceptance criteria:**

- edit mode shows draggable agent rows instead of plain static row inputs
- add/remove/edit still work in edit mode
- saving edit mode persists final agent order through the existing `reorderTaskAgents` path
- persisted order is reflected by task card pills after save
- create mode behavior is unchanged apart from any required shared layout adjustments

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final regression and doc close-out

**task_id:** `task-card-polish-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-card-polish-and-agent-edit-design.md`
- `docs/superpowers/plans/2026-04-15-task-card-polish-and-agent-edit.md`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `60`
**max_deleted_loc:** `20`

**acceptance criteria:**

- docs reflect the accepted card-polish and edit-dialog behavior
- CM record contains accepted commits and verification evidence
- final regression coverage proves the saved order is visible on the task card

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `7d420733` | Polished the task card chrome by making the edit affordance icon-only in the upper-right, shrinking the status chip into a compact lower-right badge for expanded cards, and reducing the visual weight of `Draft` while preserving title, id, save indicator, and agent pills. | `bun test src/components/TaskPanel/TaskHeader.test.tsx` ✅ 15 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `1dcb7895` | Upgraded edit mode to use draggable agent rows, preserved add/remove/edit behavior, and persisted final agent order through the existing `reorderTaskAgents` path after add/update/remove mutations complete. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 20 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 3 | `61bd4545` | Closed spec to Accepted; added a2 (codex/coder) to TaskHeader mock and order-persistence regression test proving agent pills render in store order (leadIdx < coderIdx). | `bun test src/components/TaskPanel/TaskHeader.test.tsx` ✅ 16 passed; `git diff --check` ✅ | accepted |
