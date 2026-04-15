# Edit Task Connect And Delete Design

## Summary

The task pane currently allows editing task agents, but edit mode stops at persistence:

- `Edit Task` only offers `Save`
- there is no edit-mode path to connect providers from the saved agent list
- tasks cannot be deleted from either the task card or the edit dialog

This leaves the edit flow incomplete for real task management. Users can update agents, but they cannot immediately connect them, and they cannot delete obsolete tasks.

## Product Goal

- Add `Save & Connect` to `Edit Task`.
- Make edit-mode connection target only the providers that still exist in the saved agent list.
- Add `Delete Task` to both the task card and the edit dialog.
- Require secondary confirmation for every task deletion.
- Automatically disconnect task-bound providers before deleting the task.

## Scope

### Included

- Edit-dialog footer changes for `Save & Connect` and `Delete Task`
- Task-card delete entry
- Frontend task-delete action and backend `daemon_delete_task` plumbing
- Backend task deletion cleanup, including provider disconnect-before-delete
- Auto-selecting the next remaining task after deleting the active task
- Focused frontend and store/backend tests for edit-connect and task deletion
- Plan and CM documentation

### Excluded

- Redesigning dialog layout or trigger styling
- Changing agent-option sourcing or provider-history semantics
- Changing task-card visual style beyond adding the delete affordance
- Introducing a custom confirmation modal system

## Root Cause

### Edit mode

`TaskSetupDialog` already returns `requestLaunch`, but `mode === "edit"` only renders `Save`, and `TaskPanel`'s edit submit path only persists agent CRUD/reorder changes. It never invokes Claude/Codex launch or resume helpers.

### Task deletion

The backend task graph can remove a task internally, but the frontend has:

- no Tauri command for task deletion
- no store action for task deletion
- no UI entry point for task deletion

The current `remove_task(...)` path also needs task-scoped cleanup behavior, not just top-level task record removal.

## Product Decision

### Edit dialog footer

Edit mode uses four actions:

- `Cancel`
- `Delete Task`
- `Save`
- `Save & Connect`

`Save` remains a pure persistence action.

`Save & Connect` first commits the agent-list edits, then connects only the providers still present in the saved list.

### Task deletion entries

`Delete Task` is available in both places:

- the task card
- the `Edit Task` dialog

Both entries use the same deletion flow and the same confirmation wording.

### Confirmation

Deletion requires secondary confirmation every time.

To keep scope small and avoid a new modal system, the initial implementation uses one shared confirmation helper built on the browser confirmation path.

### Disconnect-before-delete

If the task currently owns an active Claude and/or Codex provider binding:

- disconnect those providers first
- then delete the task

This is automatic. The user does not have to manually disconnect first.

### Selection after delete

If the deleted task is the active task:

- automatically select the next remaining task in the same workspace list order
- the order matches the task-pane list behavior: newest first by `createdAt`
- if no tasks remain in that workspace, clear the active task and show the empty state

If the deleted task is not active:

- keep the current active task unchanged

## Architecture

### Edit-mode connect

`Save & Connect` should reuse the same launch/resume helpers already used by create mode:

- Claude uses `daemon_launch_claude_sdk` or `resumeSession`
- Codex uses `applyConfig(buildCodexLaunchConfig(...))` or `resumeSession`

The connection target is computed from the saved agent list after edit persistence completes.

This avoids introducing a second provider-launch implementation path.

### Task deletion

Add a dedicated backend command and frontend store action:

- Tauri command: `daemon_delete_task`
- store action: `deleteTask(taskId)`

The backend command owns the authoritative delete sequence:

1. identify whether the task currently owns Claude and/or Codex runtime bindings
2. stop those providers for that task if needed
3. remove the task runtime entry
4. remove the task plus its task-scoped task agents, sessions, and artifacts
5. choose the next active task when the deleted task was active
6. persist the task graph

The frontend store action then refreshes local state so the pane and dialog stay consistent.

### Data cleanup

Task deletion must cascade task-scoped state:

- `tasks[taskId]`
- `taskAgents[taskId]`
- `sessions[taskId]`
- `artifacts[taskId]`
- `providerSummaries[taskId]`
- `replyTargets[taskId]`

This applies in both frontend store state and backend task graph/runtime state.

## Behavior Changes

### Edit mode

- Users can save edits without launching providers.
- Users can save edits and immediately connect the providers that remain in the task.

### Task card

- Each editable task card exposes a delete entry in addition to edit.

### Deletion

- Deleting a task always asks for confirmation.
- Active task deletion automatically disconnects task-owned providers first.
- Active selection moves to the next surviving task in the same workspace, or clears when none remain.

## File Plan

### Modified files

- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `tests/task-store.test.ts`
- `src-tauri/src/commands_task.rs`
- `src-tauri/src/main.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`

## Testing Strategy

- Extend dialog tests to cover edit-mode `Save & Connect` and delete-button presence.
- Extend task-card tests to cover delete affordance presence.
- Extend store tests to cover delete action state cleanup and next-task selection behavior.
- Extend backend task-graph / daemon tests to cover cascading task deletion and provider disconnect-before-delete behavior.
- Re-run focused frontend tests, focused Rust tests, and the full frontend build.

## Acceptance Criteria

- `Edit Task` shows `Save & Connect` and `Delete Task`.
- `Save & Connect` only connects providers still present in the saved agent list.
- Task card and edit dialog both expose `Delete Task`.
- Both delete entries require secondary confirmation.
- Deleting a task disconnects task-bound providers before removal.
- Deleting the active task auto-selects the next remaining task in current workspace list order, or clears active task if none remain.
- Focused tests and required verification commands pass.
