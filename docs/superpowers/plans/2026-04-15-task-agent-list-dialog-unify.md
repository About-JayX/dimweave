# Task Agent List And Config Dialog Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the split create/edit task dialog with a unified two-pane agent-management dialog where the left pane is the sortable agent list and the right pane edits one selected agent’s provider/model/role/session configuration.

**Architecture:** Keep task-pane cards unchanged, rework `TaskSetupDialog` into a shared create/edit shell with one ordered agent list plus one selected-agent config pane, and thread the resulting provider-aware agent config through the existing create/edit submit paths without preserving the old standalone provider/runtime block.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Vite, `dnd-kit`

---

## Memory

- Recent related commits:
  - `51213dac` — replaced edit dialog native drag with `dnd-kit`
  - `7d420733` — polished task card chrome and kept the card compact
  - `1dcb7895` — persisted edit dialog ordering through `reorderTaskAgents`
  - `2769cb31` / `d041dd4a` — made the task pane card-only and locked active-task sync behavior
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-edit-dialog-dnd-kit-sort-fix.md`
  - `docs/superpowers/plans/2026-04-15-task-card-polish-and-agent-edit.md`
  - `docs/superpowers/plans/2026-04-15-task-pane-card-only.md`
- Constraints carried forward:
  - task cards remain compact and read-only except for selection and edit entry
  - `Create` and `Edit` should converge on one dialog structure
  - `provider` and `model` are separate fields; `model` has no default selection
  - sorting stays in the left agent list

## Task 1: Redesign dialog shell into left-list/right-config layout

**task_id:** `task-dialog-two-pane-shell`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/TaskPanel/dom-test-env.ts`

**max_files_changed:** `4`
**max_added_loc:** `320`
**max_deleted_loc:** `160`

**acceptance criteria:**

- dialog no longer renders the old standalone provider/runtime block
- left pane shows only the ordered agent list and add/select controls
- right pane shows the selected-agent config panel
- create and edit share the same two-pane layout
- create mode starts with an empty list and empty right-pane placeholder

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Add provider-aware selected-agent config behavior

**task_id:** `selected-agent-provider-config`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/components/ClaudePanel/launch-request.ts`
- `src/components/AgentStatus/codex-launch-config.ts`
- `src/components/AgentStatus/provider-session-view-model.ts`

**max_files_changed:** `7`
**max_added_loc:** `420`
**max_deleted_loc:** `180`

**acceptance criteria:**

- selected-agent form uses separate `provider` and `model` fields
- `model` is initially unselected for new agents
- `role`, `model`, `effort`, and session controls adapt to provider capabilities
- create/edit submit paths use the unified agent config data model
- provider switches clear invalid dependent selections

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final regression and doc close-out

**task_id:** `task-dialog-unify-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-agent-list-dialog-unify-design.md`
- `docs/superpowers/plans/2026-04-15-task-agent-list-dialog-unify.md`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `60`
**max_deleted_loc:** `20`

**acceptance criteria:**

- docs reflect the accepted unified dialog behavior
- CM record contains accepted commits and verification evidence
- final regression proves task card pills still follow persisted agent order after the dialog redesign

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `75d1fce6` | Replaced the split create/edit shell with a unified two-pane dialog: left ordered agent list, right selected-agent config pane, empty create-mode default state, and no standalone provider/runtime block. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 26 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `81fc11ac`, `48759edd` | Added provider-aware selected-agent configuration in the right pane with separate provider/model fields, empty default model selection, provider-dependent effort/session controls, config derivation into `claudeConfig` / `codexConfig`, and follow-up provider-capability UI gating so unsupported or provider-specific controls render correctly. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 37 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 3 | _this commit_ | Closed spec to Accepted; added dialog-redesign regression test proving card pills render both providers in persisted store order after the unified two-pane redesign. | `bun test src/components/TaskPanel/TaskHeader.test.tsx` ✅ 16 passed; `git diff --check` ✅ | candidate |
