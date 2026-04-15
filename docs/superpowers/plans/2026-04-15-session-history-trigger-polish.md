# Session History Trigger Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish the shared history-select trigger so long selected session labels use middle ellipsis and the trigger pill is taller across Claude, Codex, and the task setup dialog, without changing dropdown menu rows.

**Architecture:** Keep provider-history behavior and menu row rendering intact. Apply the trigger-only polish centrally in `CyberSelect`'s `history` variant and verify it with shared-component tests plus a task-dialog integration test.

**Tech Stack:** React 19, TypeScript, Bun test, Vite, shared frontend components

---

## Memory

- Recent related commits:
  - `63dc959d` — restored readable history dropdown rows and wider history menu layout
  - `201af986` — applied middle ellipsis to history menu titles and matched panel width to trigger width
  - `2dbf70be` — restored history dropdown behavior in `TaskSetupDialog`
  - `9c744547` — switched task-dialog controls to shared `CyberSelect`
  - `c456d7af` — verified the live `TaskPanel -> TaskSetupDialog` integration path
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-07-session-history-ui-polish.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-provider-source-fix.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-live-fix.md`
- Constraints carried forward:
  - keep the history dropdown menu items unchanged
  - keep the shared `variant="history"` implementation path
  - verify the dialog path explicitly because this surface previously passed component-only review but missed live UX defects

## Baseline

- Worktree: `.worktrees/session-history-trigger-polish`
- Baseline verification on `lead/session-history-trigger-polish` before changes:
  - `bun test src/components/ui/cyber-select.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 52 passed
  - `bun run build` ✅

## Task 1: Polish the shared history trigger

**task_id:** `history-trigger-polish`

**allowed_files:**

- `src/components/ui/cyber-select.tsx`
- `src/components/ui/cyber-select.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `140`
**max_deleted_loc:** `70`

**acceptance criteria:**

- long selected labels in the history trigger use middle ellipsis rather than tail truncation
- `New session` remains unchanged when selected
- the shared history trigger is visibly taller via trigger-only styling changes
- history dropdown menu item layout and content remain unchanged
- the task dialog render path proves it inherits the shared history-trigger treatment

**verification_commands:**

- `bun test src/components/ui/cyber-select.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Final docs and CM close-out

**task_id:** `history-trigger-polish-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-session-history-trigger-polish-design.md`
- `docs/superpowers/plans/2026-04-15-session-history-trigger-polish.md`

**max_files_changed:** `2`
**max_added_loc:** `30`
**max_deleted_loc:** `10`

**acceptance criteria:**

- docs reflect the accepted trigger-only scope
- CM record contains the real commit hash and verification evidence
- any review notes about unchanged menu rows are recorded if relevant

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `0eca7bd3` | Polished the shared history trigger by applying middle ellipsis to selected long labels, increasing history-trigger height, and adding shared plus task-dialog coverage while leaving dropdown menu rows unchanged. | `bun test src/components/ui/cyber-select.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx` ✅ 45 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `f946708c` | Closed the spec to Accepted and updated the plan CM record for the accepted trigger-only scope. | `git diff --check -- docs/superpowers/specs/2026-04-15-session-history-trigger-polish-design.md docs/superpowers/plans/2026-04-15-session-history-trigger-polish.md` ✅ | accepted |
