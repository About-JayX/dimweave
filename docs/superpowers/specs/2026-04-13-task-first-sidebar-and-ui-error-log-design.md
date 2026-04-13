# Task-First Sidebar And UI Error Log Design

## Goal

Reshape the current task/agent workflow so the product behaves as `workspace -> task -> agents`, while also stopping UI crashes from presenting as full-page reload/reset loops and giving UI errors a permanent, dedicated review surface.

## Scope

This design covers three tightly related changes:

1. Move from the current “workspace selection creates a task immediately” flow to a single-workspace, multi-task flow.
2. Remove the standalone `Agents` sidebar entry and fold agent configuration into task creation/editing.
3. Separate UI error capture from runtime logs so UI failures are durable, inspectable, and do not trigger endless automatic remount attempts.

## Non-Goals

- Multi-workspace task management. This phase keeps one active workspace at a time.
- Task renaming UX. New tasks will use `task_id` as the initial title.
- A brand-new shell layout. The current shell structure stays in place.
- Reworking runtime/provider internals beyond the interface changes needed to support task-first creation and task-scoped agent editing.

## Current Problems

### 1. Workspace selection and task creation are incorrectly coupled

Today the app shows the workspace entry overlay whenever there is no active task. Continuing from that overlay calls `startWorkspaceTask()`, which immediately creates a task. That blocks the intended “single workspace, multiple tasks” model because “no active task” is currently treated as “no workspace selected”.

### 2. Agents are treated as a parallel object instead of task configuration

The shell still exposes a standalone `Agents` pane. That makes the product model ambiguous: users can enter an agent-oriented surface before they have intentionally established task context, even though runtime ownership is already task-scoped underneath.

### 3. UI errors are mixed into runtime logs and endlessly retried

The current `ErrorBoundary` logs UI errors into the same `terminalLines` queue used for runtime/system logs, then automatically resets itself on the next animation frame. When the underlying bug is persistent, React immediately throws again, making the app feel like it is constantly reloading and resetting state.

## Product Model

### Workspace

- The app has one selected workspace at a time.
- Workspace selection remains available from the current shell header/switcher.
- Selected workspace becomes frontend state independent from `activeTaskId`.
- The workspace entry overlay only appears when no workspace has been selected yet.

### Task

- A task is the primary object inside the selected workspace.
- The sidebar’s `Task` pane becomes the only place to create, select, and edit tasks.
- A task may exist without any provider launched yet.
- New tasks are created only after the user confirms the task-setup dialog.
- New task titles default to the generated `task_id`.

### Agents

- Agents are no longer a first-class sibling navigation item.
- Agent/provider setup becomes task configuration owned by the task lifecycle.
- The standalone `Agents` pane is removed.
- Existing agent configuration capabilities are preserved, but they move into the task setup/edit flow.

## Information Architecture

### Sidebar

- Keep the current shell frame and navigation model.
- Remove the `Agents` nav item from the shell context bar.
- Keep `Task`, `Approvals`, `Tools`, and `Logs`.

### Task Pane

The task pane becomes responsible for:

- listing tasks for the selected workspace
- switching the active task
- opening `New Task`
- opening `Edit Task` for the active task
- continuing to show task sessions/artifacts/history for the active task

### Task Setup Dialog

One reusable dialog handles both create and edit modes.

#### Create Mode

- Triggered by `New Task`.
- Opens before any task is created.
- Contains the current agent/provider configuration controls now living in the `Agents` pane.
- Confirmation creates the task and stores the chosen task-level provider bindings/configuration.
- The user is allowed to create the task without starting any provider session.

#### Edit Mode

- Triggered by `Edit Task`.
- Loads the active task’s current agent/provider configuration.
- Saves updates back onto the active task without creating a new task.

### Main Chat Surface

- Keep the current message surface and input placement.
- If there is no active task, the reply input stays visible but disabled.
- The disabled state must explicitly instruct the user to create a task first.

### Runtime Logs + UI Error Dialog

- Keep the current runtime log page.
- Keep the current top-bar error badge location within the runtime log surface.
- Clicking the error badge opens a dedicated `Error Log Dialog`.
- The dialog shows only persistent UI error records, not the rolling runtime log stream.

## State Model

### Frontend Workspace State

Add a selected-workspace state independent from `activeTaskId`.

This state drives:

- whether the workspace entry overlay should render
- which workspace’s tasks are listed in the task pane
- which workspace new tasks belong to
- the shell workspace label when no task is active

### Frontend Task State

Task store responsibilities expand from “active task snapshot” to “selected workspace + task collection + active task”.

The task store must support:

- loading tasks for the selected workspace without forcing an active task
- creating a task in the selected workspace only after dialog confirmation
- keeping `activeTaskId = null` as a valid state inside an already-selected workspace
- opening edit flows against the currently selected task

### Frontend UI Error State

UI errors move out of `bridge-store.terminalLines` into a dedicated persistent error queue.

This state must hold:

- a stable error identifier
- timestamp
- summary message
- optional component stack / metadata
- whether the error dialog is open (component-local state is also acceptable for visibility)

The queue must not be truncated by the runtime log rolling window.

## Data Flow

### Workspace Entry

1. User selects or picks a workspace.
2. Frontend stores that workspace as the current workspace.
3. Task store loads tasks for that workspace.
4. If no task exists yet, the shell remains usable but the reply input stays disabled.

### New Task

1. User clicks `New Task` in the task pane.
2. Frontend opens `Task Setup Dialog` in create mode.
3. User configures task-level agent/provider settings.
4. User confirms.
5. Frontend sends a create request using the selected workspace plus the chosen task configuration.
6. Backend creates the task and returns the new `task_id`.
7. Frontend sets the new task as active.
8. Optional provider launch/resume actions run only after creation succeeds.

### Edit Task

1. User clicks `Edit Task`.
2. Frontend opens the same dialog in edit mode.
3. User adjusts provider bindings/session-start preferences.
4. Frontend persists the task configuration update.
5. If the user requested a live connect/resume action, that starts after the edit is saved.

### Sending Messages

- Sending requires an active task.
- No active task means no send action reaches the daemon.
- The disabled input and helper text become the first line of enforcement.

### UI Error Capture

1. UI subtree throws.
2. `ErrorBoundary` records a structured UI error into the dedicated UI-error queue.
3. The boundary renders a fallback state instead of immediately remounting the subtree.
4. The user may click `Retry` to remount intentionally.
5. The runtime log surface badge reflects the UI-error queue count and opens the dialog for review.

## Error Handling

### Task Setup Errors

- Validation or create/edit failures stay inside the task setup dialog.
- Canceling the dialog must not create partial tasks.
- If provider launch fails after task creation, the task remains valid; the failure is surfaced as task configuration/runtime feedback, not as a failed create transaction.

### UI Error Recovery

- Remove the “retry next frame” behavior from `ErrorBoundary`.
- Recovery becomes explicit and user-driven through a `Retry` action.
- This prevents persistent React errors from masquerading as full app reloads.

## Backend Contract Changes

The existing `create task` contract must expand so task creation can carry task-level agent configuration from the dialog instead of relying on post-hoc global agent setup.

The implementation may choose either of these internal shapes:

- a single `create_task_with_config` style command
- or a transactionally equivalent `create_task` + `apply_task_config` flow

But the user-facing interaction must behave like one confirmation action.

## Testing Strategy

### Frontend Interaction Tests

- workspace selected + no active task renders disabled reply input
- `New Task` opens the task setup dialog
- canceling the dialog creates nothing
- confirming the dialog creates exactly one task and selects it
- `Edit Task` reopens the same dialog for the active task
- shell nav no longer exposes the standalone `Agents` pane

### Store Tests

- selected workspace can exist with `activeTaskId = null`
- task list hydration works independently from active-task hydration
- task creation uses current workspace rather than implicit active task state

### Error Log Tests

- UI errors append to the dedicated UI-error queue
- runtime log queue remains separate
- error badge count reflects UI-error entries
- clicking the badge opens the dialog with persistent entries
- `ErrorBoundary` no longer enters an automatic remount loop

### Regression Tests

- the previous `Maximum update depth exceeded` class of error does not cause endless auto-retry from the boundary
- task-scoped runtime state remains intact after task/agent UI reshaping

## Rollout Notes

- This is intentionally phase-scoped to one workspace at a time.
- The design keeps current shell structure to minimize user retraining and reduce implementation risk.
- Multi-workspace task orchestration can be layered on later once workspace and active-task state are no longer conflated.
