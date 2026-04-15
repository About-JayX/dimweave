# Saved Agent Connect Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make create/edit `Save & Connect` launch from the persisted saved agent list so new and existing agents always connect with real stable `agentId` values.

**Architecture:** Keep daemon-side explicit-`agentId` online/no-op semantics intact. Fix the remaining frontend gap by building connect targets only after persistence completes: create mode uses returned `addTaskAgent(...)` rows, and edit mode uses the final saved agent list after add/update/remove/reorder completes.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Bun test, Cargo build/check, Vite

---

## Memory

- Recent related commits:
  - `64954a8d` — bound edit connect to saved agents, but still not from the final persisted list in all cases
  - `16e5bc48` — finalized the prior edit-connect/delete revision
  - `e817eeaa` / `8cf9d2c3` — added daemon-backed per-agent runtime status for task cards
  - `343ae415` / `1dcb7895` — task-agent CRUD and edit ordering persistence
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-edit-task-connect-and-delete-revision.md`
  - `docs/superpowers/plans/2026-04-15-task-card-agent-status.md`
- Constraints carried forward:
  - keep daemon-owned online/no-op decision by explicit `agentId`
  - do not change delete confirmation again
  - do not touch task-card pill status work

## Baseline

- Worktree: `.worktrees/saved-agent-connect-flow`
- Baseline verification before changes:
  - `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 59 passed
  - `cargo build -p dimweave-bridge` ✅
  - `cargo check --manifest-path src-tauri/Cargo.toml` ✅
  - `bun run build` ✅

## Task 1: Launch from saved agents after persistence

**task_id:** `task-saved-agent-connect-flow`

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/ClaudePanel/launch-request.ts`
- `src/components/AgentStatus/codex-launch-config.ts`
- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`

**max_files_changed:** `8`
**max_added_loc:** `220`
**max_deleted_loc:** `100`

**acceptance criteria:**

- create/edit connect flows no longer launch from the raw draft payload
- newly added agents launch with persisted returned `agentId`
- newly created tasks launch from the saved task-agent list
- multiple same-provider agents remain distinct launch targets
- daemon explicit-`agentId` no-op semantics remain intact

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `cargo build -p dimweave-bridge`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `bun run build`
- `git diff --check`

## Task 2: Final docs and CM close-out

**task_id:** `task-saved-agent-connect-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-saved-agent-connect-flow-design.md`
- `docs/superpowers/plans/2026-04-15-saved-agent-connect-flow.md`

**max_files_changed:** `2`
**max_added_loc:** `30`
**max_deleted_loc:** `10`

**acceptance criteria:**

- docs reflect the accepted saved-agent connect flow
- CM record contains real commit hashes and verification evidence

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `ac703a8f` | Reworked create/edit `Save & Connect` so both flows launch from the persisted saved agent list, using returned `addTaskAgent(...)` identities for new agents and preserving existing `agentId` values for unchanged agents. | `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx` ✅ 62 passed; `cargo build -p dimweave-bridge` ✅; `cargo check --manifest-path src-tauri/Cargo.toml` ✅; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | pending close-out | Spec marked Accepted and CM finalized after Task 1 passed. | `git diff --check -- docs/superpowers/specs/2026-04-15-saved-agent-connect-flow-design.md docs/superpowers/plans/2026-04-15-saved-agent-connect-flow.md` pending | pending |
