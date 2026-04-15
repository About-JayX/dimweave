# Task Agent Dialog Live UX Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the remaining live task-agent dialog UX gaps by constraining role selection, reusing the old panel select styling, and wiring the real Codex model source into the running dialog path.

**Architecture:** Keep the accepted two-pane/styled dialog, replace the remaining permissive or weak controls with shared `CyberSelect`-based selects, and thread live Codex model/reasoning data from the existing account store into `TaskSetupDialog` through the real caller path.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Vite, `dnd-kit`

---

## Memory

- Recent related commits:
  - `92ab537a` — branded left rows, locked first row, provider-card right pane
  - `a7035b23` — converted model/effort to dropdowns
  - `2dbf70be` / `56ffb5f7` — restored history dropdown semantics and aligned provider option sources
  - `ef370979` — documented the prior review miss: acceptance criteria too narrow and verification too component-local
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-style-and-dropdowns.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-provider-source-fix.md`
- Constraints carried forward:
  - two-pane layout stays
  - styled provider-card pane stays
  - history dropdown semantics stay
  - this follow-up must explicitly cover the live integration boundary to avoid repeating the earlier review miss

## Task 1: Constrain role selection and switch dialog selects to panel-style components

**task_id:** `task-dialog-role-and-select-style`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/ui/cyber-select.tsx`
- `src/components/AgentStatus/RoleSelect.tsx`

**max_files_changed:** `5`
**max_added_loc:** `260`
**max_deleted_loc:** `140`

**acceptance criteria:**

- `role` becomes a dropdown with only `lead` / `coder`
- provider/model/effort/history controls use the stronger select treatment aligned with the old provider panels
- the dialog no longer relies on native `<select>` for these main controls

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Wire live Codex model and reasoning data into the dialog caller path

**task_id:** `task-dialog-codex-live-source`

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/stores/codex-account-store.ts`
- `src/components/AgentStatus/CodexPanel.tsx`
- `src/components/AgentStatus/codex-launch-config.ts`

**max_files_changed:** `7`
**max_added_loc:** `300`
**max_deleted_loc:** `140`

**acceptance criteria:**

- the live `TaskPanel -> TaskSetupDialog` path supplies Codex model options from the real Codex account store
- reasoning/effort options derive correctly from the selected live Codex model
- the dialog shows a valid loading/empty state when Codex model data is unavailable
- verification covers the live caller path rather than only injected component props

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final regression and doc close-out

**task_id:** `task-agent-dialog-live-fix-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-agent-dialog-live-fix-design.md`
- `docs/superpowers/plans/2026-04-15-task-agent-dialog-live-fix.md`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `40`
**max_deleted_loc:** `20`

**acceptance criteria:**

- docs reflect the accepted live-fix behavior
- CM record contains accepted commits and verification evidence
- final regression still proves task-card pills remain intact after the follow-up fixes

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `9c744547` | Converted `role` to a constrained `lead` / `coder` dropdown, replaced the dialog’s native `<select>` controls with `CyberSelect`-based controls aligned to the old provider panel styling, and removed the last free-form role/model/effort inputs from the live dialog. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 43 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `c456d7af` | Wired the live Codex model source from `TaskPanel` / `codex-account-store` into `TaskSetupDialog`, derived reasoning options from the selected live Codex model, and added a valid loading placeholder when Codex model data is unavailable. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 45 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 3 | `_this_` | Closed spec to Accepted; added live-fix regression test proving card pills render in persisted order after role constraint and live Codex model wiring. | `bun test src/components/TaskPanel/TaskHeader.test.tsx` ✅ 20 passed; `git diff --check` ✅ | accepted |
