# Task Setup Dialog Option Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the fake `Select model` menu entry and eliminate duplicate `Default` effort options in the task setup dialog while preserving the current unified trigger styling.

**Architecture:** Keep this fix local to `TaskSetupDialog`: normalize model and effort option arrays before passing them to `CyberSelect`, and verify the behavior with focused dialog render tests.

**Tech Stack:** React 19, TypeScript, Bun test, Vite

---

## Memory

- Recent related commits:
  - `561d8ac0` — unified task dialog trigger styling
  - `995c9456` — doc close-out for unified trigger follow-up
  - `9c744547` — switched dialog controls to `CyberSelect`
  - `a7035b23` — made provider/model/effort/history dropdown-driven in the dialog
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-task-setup-dialog-trigger-unify.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-live-fix.md`
- Constraints carried forward:
  - do not touch trigger styling again
  - keep the fix local to dialog option assembly
  - preserve current provider option sources

## Baseline

- Worktree: `.worktrees/dialog-option-cleanup`
- Baseline verification before changes:
  - `bun test src/components/ui/cyber-select.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 60 passed
  - `bun run build` ✅

## Task 1: Clean up model and effort options in the dialog

**task_id:** `task-dialog-option-cleanup`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`

**max_files_changed:** `2`
**max_added_loc:** `120`
**max_deleted_loc:** `60`

**acceptance criteria:**

- `Model` still shows `Select model` when unset, but that text is not inserted as a real menu option
- `Effort` shows only one `Default` option
- trigger styling and menu layout remain unchanged
- tests explicitly cover the dialog option contract

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Final docs and CM close-out

**task_id:** `task-dialog-option-cleanup-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-setup-dialog-option-cleanup-design.md`
- `docs/superpowers/plans/2026-04-15-task-setup-dialog-option-cleanup.md`

**max_files_changed:** `2`
**max_added_loc:** `30`
**max_deleted_loc:** `10`

**acceptance criteria:**

- docs reflect the accepted local option-cleanup scope
- CM record contains real commit hashes and verification evidence

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `653208d5` | Removed the fake `Select model` menu entry by using a placeholder-only unset sentinel while preserving the real Claude `Default` model option, deduped `Default` in effort options, and added dialog tests that inspect the actual options passed into `CyberSelect`. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx` ✅ 42 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `06b1436e` | Closed the spec to Accepted and updated the plan CM record for the accepted local option-cleanup scope. | `git diff --check -- docs/superpowers/specs/2026-04-15-task-setup-dialog-option-cleanup-design.md docs/superpowers/plans/2026-04-15-task-setup-dialog-option-cleanup.md` ✅ | accepted |
