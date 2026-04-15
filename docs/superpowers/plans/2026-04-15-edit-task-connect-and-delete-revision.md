# Edit Task Connect And Delete Revision Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the unreliable browser-native delete confirmation with a React confirmation dialog and fix edit-mode `Save & Connect` so it binds launches to existing saved agents, including multiple same-provider agents, while only connecting offline agents.

**Architecture:** Keep the accepted delete semantics and edit-mode actions, but revise the implementation details. Move confirmation into a shared React dialog owned by `TaskPanel`. Revise edit-mode connect from provider-family launch to agent-bound launch by carrying explicit `agentId` through the frontend and daemon launch paths so existing task agents are reused instead of recreated.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Rust, Bun test, Cargo test, Vite

---

## Memory

- Recent related commits:
  - `414edabd` — added backend/store delete flow
  - `b6b62e2b` — added edit-mode `Save & Connect` plus delete entry points, but used `window.confirm` and provider-family connect logic
  - `561d8ac0` — unified task dialog trigger styling
  - `653208d5` — local dialog option cleanup
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-15-edit-task-connect-and-delete.md`
  - `docs/superpowers/plans/2026-04-15-task-pane-card-only.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-list-dialog-unify.md`
- Lessons carried forward:
  - runtime-critical UX needs real in-app validation, not only component tests
  - provider family (`claude` / `codex`) is not a sufficient identity when one task can contain multiple same-provider agents
  - edit-mode reconnect must reuse saved `agentId`, not create a fresh one

## Baseline

- Worktree: `.worktrees/edit-connect-delete-revision`
- Baseline verification before changes:
  - `bun test tests/task-store.test.ts src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx src/components/TaskPanel/TaskHeader.test.tsx` ✅ 157 passed
  - `cargo build -p dimweave-bridge` ✅
  - `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture` ✅ 58 passed
  - `bun run build` ✅

## Task 1: Replace delete confirmation with shared React dialog

**task_id:** `task-react-delete-confirmation`

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/ui/confirm-dialog.tsx`
- `src/components/ui/confirm-dialog.test.tsx`

**max_files_changed:** `6`
**max_added_loc:** `240`
**max_deleted_loc:** `80`

**acceptance criteria:**

- no delete path uses `window.confirm(...)`
- task card and edit dialog both open the same React confirmation dialog
- confirming delete still calls the already-accepted `deleteTask(taskId)` path
- canceling confirmation leaves task state unchanged

**verification_commands:**

- `bun test src/components/ui/confirm-dialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx src/components/TaskPanel/TaskHeader.test.tsx`
- `bun run build`
- `git diff --check`

## Task 2: Make edit Save & Connect agent-bound and offline-only

**task_id:** `task-agent-bound-edit-connect`

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/ClaudePanel/launch-request.ts`
- `src/components/AgentStatus/codex-launch-config.ts`
- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`

**max_files_changed:** `9`
**max_added_loc:** `280`
**max_deleted_loc:** `120`

**acceptance criteria:**

- edit-mode `Save & Connect` iterates over saved task agents, not provider family shortcuts
- only offline saved agents are launched
- explicit `agentId` is carried through launch so existing agents are reused
- multiple same-provider agents remain distinct sessions and do not create duplicate task-agent records

**verification_commands:**

- `bun test src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `bun run build`
- `git diff --check`

## Task 3: Final docs and CM close-out

**task_id:** `task-edit-connect-delete-revision-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-edit-task-connect-and-delete-revision-design.md`
- `docs/superpowers/plans/2026-04-15-edit-task-connect-and-delete-revision.md`

**max_files_changed:** `2`
**max_added_loc:** `30`
**max_deleted_loc:** `10`

**acceptance criteria:**

- docs reflect the revised confirmation and agent-bound connect behavior
- CM record contains real commit hashes and verification evidence

**verification_commands:**

- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `34b45c6d` | Replaced browser-native delete confirmation with a shared React `ConfirmDialog` component and routed both task-card and edit-dialog delete triggers through the same confirmation state in `TaskPanel`. | `bun test src/components/ui/confirm-dialog.test.tsx src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx src/components/TaskPanel/TaskHeader.test.tsx` ✅ 44 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | not started | Execution has not started yet. | No task-local verification yet. | not started |
| Task 3 | not started | Execution has not started yet. | No task-local verification yet. | not started |
