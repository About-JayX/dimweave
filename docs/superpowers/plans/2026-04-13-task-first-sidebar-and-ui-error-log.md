# Task-First Sidebar And UI Error Log Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the current shell layout, but switch the product flow to `workspace -> task -> agents`, remove the standalone `Agents` pane, and stop UI crashes from appearing as full-page reload/reset loops by introducing a dedicated UI error log dialog and manual retry flow.

**Architecture:** First decouple selected workspace from active task so a workspace can exist without forcing task creation. Then add a task-configuration contract that lets `New Task` open a setup dialog, confirm task bindings, and only then create/select the task. Finally, separate UI-error storage from runtime logs and change the error boundary from automatic remounting to explicit fallback + retry.

**Tech Stack:** React 19, Zustand 5, Tauri 2, Rust async daemon, Bun test, Vite

---

## Memory

- Recent related commits:
  - `44e0cd29` — added the blocking workspace entry overlay
  - `737746b5` — made frontend task flows task-scoped
  - `636a4107` — added task-scoped provider-session summary
  - `94de71fd` — added the frontend crash boundary
  - `9c84b9d9` — changed crash handling to auto-recover on next frame
  - `d29fc009` — latest clean `main` baseline before this plan
- Relevant prior plan:
  - `docs/superpowers/plans/2026-04-13-task-scoped-runtime-redesign.md`
- Constraints carried forward:
  - Task-scoped runtime ownership remains the truth; this plan must not reintroduce global provider ownership semantics.
  - The first phase is single-workspace only; multi-workspace task orchestration is explicitly out of scope.
  - `Runtime logs` remains a user-facing surface; the new UI error view opens from that surface rather than replacing it.

## Scope Notes

- This plan intentionally keeps the shell frame and main chat/log surfaces.
- New task titles default to the returned `task_id`.
- `New Task` may create a task without starting any provider session.
- `Edit Task` reuses the same dialog as `New Task`.
- No active task is a valid state inside a selected workspace.

## Task 1: Decouple selected workspace from active task

**task_id:** `workspace-selection-without-task`

**Acceptance criteria:**

- The selected workspace exists as frontend state independent from `activeTaskId`.
- Entering a workspace no longer creates a task automatically.
- The workspace entry overlay only blocks when no workspace has been selected yet.
- Task store can load tasks for the selected workspace while `activeTaskId` remains `null`.
- Shell workspace label remains correct when a workspace is selected but no task is active.

**allowed_files:**

- `src/App.tsx`
- `src/components/WorkspaceEntryOverlay.tsx`
- `src/components/WorkspaceEntryOverlay.test.tsx`
- `src/components/WorkspaceSwitcher.tsx`
- `src/components/workspace-entry-state.ts`
- `src/components/workspace-entry-state.test.ts`
- `src/components/shell-layout-state.ts`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/selectors.ts`
- `tests/task-store.test.ts`

**max_files_changed:** `11`
**max_added_loc:** `360`
**max_deleted_loc:** `140`

**verification_commands:**

- `bun test src/components/workspace-entry-state.test.ts src/components/WorkspaceEntryOverlay.test.tsx`
- `bun test tests/task-store.test.ts`
- `bun run build`
- `git diff --check`

## Task 2: Add task create/edit configuration contract and store actions

**task_id:** `task-config-contract-and-store-actions`

**Acceptance criteria:**

- Frontend can create a task for the selected workspace only after dialog confirmation.
- Task creation accepts task-level provider bindings instead of relying on post-create global agent assumptions.
- Task creation succeeds even if no provider launch is requested.
- Frontend can edit the active task’s provider bindings through a dedicated update path.
- Task creation/edit actions are available through the task store with no forced provider launch side effects.

**allowed_files:**

- `src-tauri/src/main.rs`
- `src-tauri/src/commands_task.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/types.ts`
- `tests/task-store.test.ts`

**max_files_changed:** `9`
**max_added_loc:** `340`
**max_deleted_loc:** `120`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `bun test tests/task-store.test.ts`
- `bun run build`
- `git diff --check`

## Task 3: Replace the Agents pane with task-owned setup/edit flows

**task_id:** `task-setup-dialog-and-pane-merge`

**Acceptance criteria:**

- Shell navigation no longer exposes a standalone `Agents` pane.
- Task pane exposes `New Task` and `Edit Task`.
- `New Task` opens a task-setup dialog and does not create anything until confirmation.
- `Edit Task` reuses the same dialog for the active task.
- The dialog preserves the current agent-panel capabilities relevant to task setup/editing.
- Confirming `New Task` creates the task, applies bindings, and selects the new task.
- Canceling `New Task` creates nothing.
- Reply input remains visible with no active task, but is disabled and clearly instructs the user to create a task first.

**allowed_files:**

- `src/components/ShellContextBar.tsx`
- `src/components/ShellContextBar.test.tsx`
- `src/components/TaskContextPopover.tsx`
- `src/components/TaskContextPopover.test.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/TaskPanel/view-model.ts`
- `src/components/TaskPanel/ArtifactTimeline.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/ClaudePanel/index.tsx`
- `src/components/ClaudePanel/connect-state.test.ts`
- `src/components/ClaudePanel/launch-request.ts`
- `src/components/ClaudePanel/launch-request.test.ts`
- `src/components/AgentStatus/CodexPanel.tsx`
- `src/components/AgentStatus/codex-launch-config.ts`
- `src/components/AgentStatus/codex-launch-config.test.ts`
- `src/components/AgentStatus/provider-session-view-model.ts`
- `src/components/ReplyInput/index.tsx`
- `src/components/ReplyInput/index.test.tsx`
- `src/components/ReplyInput/Footer.tsx`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/selectors.ts`
- `src/stores/task-store/types.ts`

**max_files_changed:** `25`
**max_added_loc:** `920`
**max_deleted_loc:** `320`

**verification_commands:**

- `bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx`
- `bun test src/components/TaskPanel/TaskHeader.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `bun test src/components/ReplyInput/index.test.tsx`
- `bun test src/components/ClaudePanel/connect-state.test.ts src/components/ClaudePanel/launch-request.test.ts src/components/AgentStatus/codex-launch-config.test.ts`
- `bun run build`
- `git diff --check`

## Task 4: Separate UI error logs from runtime logs and stop automatic remount loops

**task_id:** `ui-error-log-and-boundary-recovery`

**Acceptance criteria:**

- UI errors are stored in a dedicated persistent queue, not in the rolling runtime log queue.
- Runtime logs remain available exactly as a separate stream.
- The runtime-log error badge count reflects the dedicated UI error queue.
- Clicking the error badge opens an `Error Log Dialog` that displays only UI errors.
- `ErrorBoundary` no longer auto-remounts the subtree on the next animation frame.
- `ErrorBoundary` shows a fallback with explicit `Retry`, and retry is user-driven.

**allowed_files:**

- `src/App.tsx`
- `src/components/ErrorBoundary.tsx`
- `src/components/ErrorBoundary.test.tsx`
- `src/components/ShellTopBar.tsx`
- `src/components/ShellTopBar.test.tsx`
- `src/components/ErrorLogDialog.tsx`
- `src/components/ErrorLogDialog.test.tsx`
- `src/stores/bridge-store/types.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/helpers.ts`
- `src/stores/bridge-store/selectors.ts`
- `src/stores/bridge-store/listener-setup.test.ts`

**max_files_changed:** `12`
**max_added_loc:** `520`
**max_deleted_loc:** `180`

**verification_commands:**

- `bun test src/components/ErrorBoundary.test.tsx src/components/ShellTopBar.test.tsx src/components/ErrorLogDialog.test.tsx`
- `bun test src/stores/bridge-store/listener-setup.test.ts`
- `bun run build`
- `git diff --check`

## Task 5: Final integration guard and documentation sync

**task_id:** `task-first-sidebar-final-integration`

**Acceptance criteria:**

- The final UI flow is `select workspace -> view task pane -> new task/edit task dialog -> optional provider launch`.
- No-task state is stable and does not trigger a blocking workspace overlay once a workspace is already selected.
- The standalone agents pane is fully removed from user-facing navigation.
- UI error logging and retry behavior work without polluting runtime log history.
- Spec and plan CM records are updated with final commit hashes and verification evidence.

**allowed_files:**

- `docs/superpowers/specs/2026-04-13-task-first-sidebar-and-ui-error-log-design.md`
- `docs/superpowers/plans/2026-04-13-task-first-sidebar-and-ui-error-log.md`
- `src/components/TaskContextPopover.test.tsx`
- `src/components/ShellContextBar.test.tsx`
- `src/components/ErrorBoundary.test.tsx`
- `tests/task-store.test.ts`

**max_files_changed:** `6`
**max_added_loc:** `180`
**max_deleted_loc:** `120`

**verification_commands:**

- `bun test tests/task-store.test.ts src/components/TaskContextPopover.test.tsx src/components/ShellContextBar.test.tsx src/components/ErrorBoundary.test.tsx`
- `bun run build`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `a4f6767a`, `c30d84bf` | Decoupled `selectedWorkspace` from `activeTaskId`, stopped workspace entry from auto-creating a task, fixed shell workspace labeling with no active task, and added workspace task-list hydration through `daemon_list_tasks`. | `bun test src/components/workspace-entry-state.test.ts src/components/WorkspaceEntryOverlay.test.tsx` ✅ 9 passed; `bun test tests/task-store.test.ts` ✅ 25 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 2 | `629e711e` | Added the task-config contract at the daemon/store layer: task creation can carry explicit lead/coder provider bindings, task bindings can be updated later through a dedicated command/store action, and both paths avoid any implicit provider launch side effects. | `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture` ✅ 29 passed; `bun test tests/task-store.test.ts` ✅ 27 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 3 | _pending_ | _pending_ | _pending_ | pending |
| Task 4 | _pending_ | _pending_ | _pending_ | pending |
| Task 5 | _pending_ | _pending_ | _pending_ | pending |
