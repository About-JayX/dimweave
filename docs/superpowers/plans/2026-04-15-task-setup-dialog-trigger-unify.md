# Task Setup Dialog Trigger Unify Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unify the task setup dialog's dropdown trigger chrome so provider/role/model/effort/session read as one compact control family, and remove the full-width session trigger behavior while preserving session middle ellipsis and unchanged menu rows.

**Architecture:** Keep option sourcing and menu behavior intact. Add shared `CyberSelect` support for a dialog-oriented trigger treatment, then opt all right-pane dialog selects into it so `default` and `history` triggers share one width and chrome strategy in the task setup dialog.

**Tech Stack:** React 19, TypeScript, Bun test, Vite, shared frontend components

---

## Memory

- Recent related commits:
  - `0b74d722` — made history select auto-width via flex layout, which is the root of the current full-width `Session` trigger
  - `201af986` — applied middle ellipsis to history trigger text in an earlier iteration
  - `9c744547` — moved dialog controls to shared `CyberSelect`
  - `c456d7af` — verified live `TaskPanel -> TaskSetupDialog` dialog integration
  - `0eca7bd3` — polished history trigger ellipsis/height but left the full-width session layout intact
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-live-fix.md`
  - `docs/superpowers/plans/2026-04-15-session-history-trigger-polish.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-provider-source-fix.md`
- Constraints carried forward:
  - preserve history dropdown semantics and menu rows
  - keep the fix focused on dialog trigger chrome and width
  - verify the dialog path directly, not only the leaf select component

## Baseline

- Worktree: `.worktrees/dialog-trigger-unify`
- Baseline verification before changes:
  - `bun test src/components/ui/cyber-select.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 55 passed
  - `bun run build` ✅

## Task 1: Unify task dialog trigger chrome and width

**task_id:** `task-dialog-trigger-unify`

**allowed_files:**

- `src/components/ui/cyber-select.tsx`
- `src/components/ui/cyber-select.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`

**max_files_changed:** `4`
**max_added_loc:** `220`
**max_deleted_loc:** `120`

**acceptance criteria:**

- `Provider`, `Role`, `Model`, `Effort`, and `Session` use one compact trigger style in the task setup dialog
- the dialog `Session` trigger no longer expands to full row width
- long selected session titles still use middle ellipsis
- dropdown menu items remain unchanged
- tests explicitly cover the dialog width/style contract rather than only the shared component in isolation

**verification_commands:**

- `bun test src/components/ui/cyber-select.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Final docs and CM close-out

**task_id:** `task-dialog-trigger-unify-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-setup-dialog-trigger-unify-design.md`
- `docs/superpowers/plans/2026-04-15-task-setup-dialog-trigger-unify.md`

**max_files_changed:** `2`
**max_added_loc:** `30`
**max_deleted_loc:** `10`

**acceptance criteria:**

- docs reflect the accepted unified-trigger scope
- CM record contains real commit hashes and verification evidence

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `561d8ac0` | Added a compact dialog-trigger treatment in `CyberSelect`, opted the task dialog `Session` control into it, and added focused tests proving the dialog now uses one compact trigger family while preserving history middle ellipsis. | `bun test src/components/ui/cyber-select.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx` ✅ 50 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | not started | Execution has not started yet. | No task-local verification yet. | not started |
