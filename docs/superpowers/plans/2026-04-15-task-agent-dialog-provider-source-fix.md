# Task Agent Dialog Provider Source And Session Selector Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align the new task-agent dialog with the old provider panels by restoring provider-correct dropdown sources and replacing the radio-plus-input session flow with the original history dropdown behavior.

**Architecture:** Keep the current two-pane styled dialog intact, but swap dialog-local option behavior for old-panel-aligned provider sources and reintroduce the existing history-dropdown model using the current provider-history helpers.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Vite, `dnd-kit`

---

## Memory

- Recent related commits:
  - `92ab537a` — branded left rows, locked first row, provider-card right pane
  - `a7035b23` — converted free-form model/effort to dropdowns
  - `75d1fce6` — unified create/edit into the two-pane task-agent dialog shell
  - `81fc11ac` / `48759edd` — provider-aware config form and capability gating
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-style-and-dropdowns.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-list-dialog-unify.md`
- Constraints carried forward:
  - keep the two-pane shell
  - keep the styled right pane
  - do not reintroduce the old standalone provider/runtime block
  - preserve the default locked first row behavior

## Task 1: Restore history dropdown behavior in the dialog

**task_id:** `task-dialog-history-dropdown-restore`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/AgentStatus/provider-session-view-model.ts`

**max_files_changed:** `4`
**max_added_loc:** `220`
**max_deleted_loc:** `120`

**acceptance criteria:**

- the `New session / Resume session` radio controls are removed from the dialog
- the session section uses a history dropdown with `New session` sentinel semantics
- no history selected means `new session`
- selecting a history item produces the correct resume action

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Align provider dropdown sources with old panel semantics

**task_id:** `task-dialog-provider-source-align`

**allowed_files:**

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/AgentStatus/provider-session-view-model.ts`
- `src/components/ClaudePanel/ClaudeConfigRows.tsx`
- `src/components/AgentStatus/CodexConfigRows.tsx`
- `src/components/AgentStatus/CodexPanel.tsx`

**max_files_changed:** `7`
**max_added_loc:** `320`
**max_deleted_loc:** `140`

**acceptance criteria:**

- Claude model/effort dropdown content matches the old Claude panel semantics
- Codex model/reasoning dropdown content matches the old Codex panel semantics
- the dialog no longer uses a divergent local option definition where a shared or legacy source should be authoritative
- provider switching still clears invalid dependent selections

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final regression and doc close-out

**task_id:** `task-dialog-provider-source-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-task-agent-dialog-provider-source-fix-design.md`
- `docs/superpowers/plans/2026-04-15-task-agent-dialog-provider-source-fix.md`
- `src/components/TaskPanel/TaskHeader.test.tsx`

**max_files_changed:** `3`
**max_added_loc:** `40`
**max_deleted_loc:** `20`

**acceptance criteria:**

- docs reflect the accepted provider-source and history-dropdown fix
- CM record contains accepted commits and verification evidence
- final regression still proves task-card pills remain intact after the follow-up fix

**verification_commands:**

- `bun test src/components/TaskPanel/TaskHeader.test.tsx`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `2dbf70be` | Restored the old history-dropdown session behavior in the unified task-agent dialog by removing the radio-plus-input flow and reusing the existing `New session` sentinel plus provider-history dropdown helpers. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 42 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `56ffb5f7` | Aligned dialog dropdown sources with the old provider panels by reusing Claude option lists from `ClaudeConfigRows`, threading dynamic Codex model/reasoning options into the dialog, and preserving the restored history-dropdown semantics from Task 1. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 42 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 3 | `94974feb` | Closed spec to Accepted; added provider-source-fix regression test proving card pills render in persisted order after history dropdown restore and source alignment. | `bun test src/components/TaskPanel/TaskHeader.test.tsx` ✅ 19 passed; `git diff --check` ✅ | accepted |

## Post-Release Addendum — Review Miss Analysis

The user reported that this follow-up was also accepted while the live dialog still behaved incorrectly. That report was valid.

The concrete miss in this plan was:

- Task 2 verified that the dialog **could** consume dynamic Codex model data when the prop was provided
- review did **not** verify that the live caller actually passed that data into `TaskSetupDialog`

So the review approved a component-level capability, but the product still failed in the real app because the integration boundary was not part of the verified acceptance path.

Additional process error:

- the plan focused on restoring old-panel *semantics* for option sources and session selection
- it did not explicitly require verification of the final live UX gaps the user still cared about, such as:
  - role remaining a text input instead of a constrained dropdown
  - the dialog still using plain native `<select>` styling rather than the prior panel’s stronger select treatment

Operational lesson:

- when the defect is user-visible in the running app, at least one acceptance criterion must verify the full live data path, not only the leaf component
- when the user expectation depends on a specific existing surface (“same as the old panel”), the review must trace both semantics and visual component reuse instead of assuming one implies the other
