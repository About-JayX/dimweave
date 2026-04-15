# Task Agent Dialog Style And Provider Dropdowns Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the unified task-agent dialog up to the visual and behavioral bar of the old provider panels by adding provider-branded list rows, a default locked first row, and provider-aware dropdown controls for model/effort/history.

**Architecture:** Keep the current two-pane dialog structure, upgrade the left pane into a richer branded summary list, restyle the right pane as a provider-card surface, and replace free-form provider fields with provider-scoped dropdown selections that derive valid launch config on submit.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Vite, `dnd-kit`

---

## Memory

- Recent related commits:
  - `75d1fce6` — unified create/edit into the two-pane task-agent dialog shell
  - `81fc11ac` / `48759edd` — added provider-aware field gating in the right pane
  - `51213dac` — switched dialog sorting to `dnd-kit`
  - `7d420733` — polished task card chrome
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-task-agent-list-dialog-unify.md`
  - `docs/superpowers/plans/2026-04-15-edit-dialog-dnd-kit-sort-fix.md`
  - `docs/superpowers/plans/2026-04-15-task-card-polish-and-agent-edit.md`
- Constraints carried forward:
  - the two-pane shell stays
  - the old standalone provider/runtime block must not return
  - `provider` and `model` remain separate fields
  - history selection keeps the existing semantics: no history selected = new session

## Task 1: Restyle the two-pane shell and branded agent list

**task_id:** `task-dialog-style-shell`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/AgentStatus/BrandIcons.tsx`

**max_files_changed:** `4`
**max_added_loc:** `260`
**max_deleted_loc:** `120`

**acceptance criteria:**

- left pane rows include provider logo/icon and richer summary text
- the first row exists by default in create mode and cannot be deleted
- the right pane uses stronger provider-card visual grouping instead of plain form styling
- two-pane layout and sorting behavior remain intact

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Replace free-form fields with provider-aware dropdowns

**task_id:** `provider-dropdown-config`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/AgentStatus/provider-session-view-model.ts`
- `src/components/ClaudePanel/launch-request.ts`
- `src/components/AgentStatus/codex-launch-config.ts`

**max_files_changed:** `6`
**max_added_loc:** `360`
**max_deleted_loc:** `160`

**acceptance criteria:**

- `provider`, `model`, and `effort` are all dropdown-based controls
- `model` starts unselected for a new agent
- model and effort option sets differ by provider
- invalid free-form parameter paths are removed
- history selection keeps the existing `empty = new session` behavior

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final regression and doc close-out

**task_id:** `task-dialog-style-dropdowns-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-agent-dialog-style-and-dropdowns-design.md`
- `docs/superpowers/plans/2026-04-15-task-agent-dialog-style-and-dropdowns.md`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `50`
**max_deleted_loc:** `20`

**acceptance criteria:**

- docs reflect the accepted styled dialog and dropdown behavior
- CM record contains accepted commits and verification evidence
- final regression still proves task card pills render in persisted order after the dialog restyle

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `92ab537a` | Restyled the unified dialog shell with branded left rows, provider-card right pane visuals, and a default locked first row that exists in create mode and cannot be deleted. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 40 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | _pending_ | Replace free-form provider fields with provider-aware dropdowns for provider/model/effort while preserving history selection semantics. | _pending_ | pending |
| Task 3 | _pending_ | Finalize docs and regression evidence for the styled task-agent dialog. | _pending_ | pending |
