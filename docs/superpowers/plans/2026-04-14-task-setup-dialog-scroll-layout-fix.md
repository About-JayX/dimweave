# Task Setup Dialog Scroll Layout Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix `TaskSetupDialog` so the action bar stays pinned to the bottom, only the lower provider-panel region scrolls, and the scrollbar styling looks correct.

**Architecture:** Keep the existing task-agent dialog logic intact. Only restructure the dialog shell into fixed top / scrollable middle / fixed bottom sections, then add focused regression assertions for the layout contract.

**Tech Stack:** React 19, Tailwind utility styling, Bun test, Vite

---

## Memory

- Recent related commits:
  - `482e21fd` — removed legacy singleton fallback badges from the task header
  - `87d8a469` — preserved `agentId` / `displayName` in `Edit Task` and added interaction tests
  - `977c8907` — allowed empty-task create and restored `Edit Task`
  - `e2500da4` — converted `TaskSetupDialog` to the agent-array model
  - `8a15a782` — originally introduced the task setup dialog
- Relevant prior plan:
  - `docs/superpowers/plans/2026-04-14-task-agent-identity-role-broadcast.md`
- Constraints carried forward:
  - `task_agents[]` remains the sole truth source
  - This fix is layout-only; no task-agent model, routing, or provider-launch semantics may change
  - `happy-dom` remains untouched in this follow-up

## Task 1: Split dialog into fixed header / scroll body / fixed footer

**task_id:** `task-setup-dialog-scroll-shell`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`

**max_files_changed:** `2`
**max_added_loc:** `120`
**max_deleted_loc:** `60`

**acceptance criteria:**

- outer dialog shell is no longer the vertical scroll container
- provider panels live inside a dedicated inner scroll region
- action buttons remain in a bottom footer section
- inner scroll region uses improved scrollbar styling
- dialog behavior/copy remain otherwise unchanged

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Final regression/doc close-out

**task_id:** `task-setup-dialog-scroll-shell-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-14-task-setup-dialog-scroll-layout-fix-design.md`
- `docs/superpowers/plans/2026-04-14-task-setup-dialog-scroll-layout-fix.md`

**max_files_changed:** `2`
**max_added_loc:** `40`
**max_deleted_loc:** `20`

**acceptance criteria:**

- spec/plan reflect the final accepted layout behavior
- CM record is filled with the accepted commit hash and verification evidence

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `454ee305` | Split `TaskSetupDialog` into a fixed header, dedicated inner scroll region, and fixed bottom footer so only the provider-panel area scrolls while the action bar stays pinned. Added focused regression tests for the new layout contract and inner-scroll styling. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx` ✅ 13 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | _pending_ | _pending_ | _pending_ | pending |
