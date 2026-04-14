# Task Multi-Task Collapsible Panels Design

> **Context:** The `task_agents[]` model is already stage-complete on `main`. This follow-up changes only the task-pane presentation and task selection flow inside the existing shell.

## Goal

Turn the current single-task detail pane into a multi-task accordion list for the selected workspace:

- users can keep creating multiple tasks in the current workspace
- each task appears as a collapsible panel
- collapsed state shows only the task summary card
- expanded state shows that task’s `Agents`, `Sessions`, and `Artifacts`
- only one task panel is expanded at a time
- expanding a task also makes it the active task so the message panel on the right stays in sync

## Why The Current UI Is Wrong

The backend/store already supports multiple tasks per workspace, but the current pane still behaves like a single-detail inspector:

- `TaskContextPopover` renders one `TaskPanel`
- `TaskPanel` renders only `activeTaskId`
- after creating a task, the UI effectively treats that active task as the only visible task

That makes the product feel single-task even though the model is not.

## Product Behavior

### Task Pane Structure

The current `Task` pane becomes a task list container rather than a single task detail surface.

Inside the pane:

- the pane header remains unchanged
- the body becomes an ordered list of task panels for the selected workspace
- `New Task` remains available at the list level

### Task Panel Item

Each task panel has two states:

- **Collapsed:** show only the summary card
- **Expanded:** show the summary card plus the task body

The summary card is the current task header card:

- title
- workspace path
- task id
- status
- save state
- agent badges
- `Edit Task`

The expanded body contains the existing sections:

- `Agents`
- `Sessions`
- `Artifacts`

### Expansion Rules

- only one task panel may be expanded at a time
- the expanded task is always the current `activeTaskId`
- clicking a collapsed panel header:
  - expands it
  - collapses all others
  - sets it active
- clicking the already-expanded panel header collapses nothing; it stays open

This keeps the state model simple: the expanded panel is the active task.

### New Task Behavior

`New Task` still creates a task inside the selected workspace.

After creation:

- the new task is inserted into the visible task list
- it becomes `activeTaskId`
- it becomes the only expanded panel
- all previously expanded panels collapse

### Message Panel Sync

The message panel already filters by `activeTaskId`.

So the task-pane requirement is:

- when a task panel becomes active, update `activeTaskId`
- the message panel then follows automatically

No separate message-panel state should be introduced.

## Architecture

### State Model

Do not introduce a second expansion truth separate from active task.

Use:

- workspace task list derived from store `tasks`
- `activeTaskId` as the single expanded panel id

This avoids mismatch such as:

- left side expanded task A
- right side showing messages for task B

### Component Shape

Introduce a list-oriented composition:

- `TaskPanelList`
- `TaskPanelItem`
- existing sections reused inside each item body where possible

The current `TaskPanel` should become either:

- the list container
- or be split into list + item helpers

The preferred direction is to keep item responsibilities small rather than making one large file bigger.

### Ordering

The list should be deterministic.

Recommended ordering:

- newest task first by `updatedAt`

That keeps newly-created tasks visible at the top and matches the user expectation that the newest task becomes the current focus.

## Non-Goals

- multi-expand accordion behavior
- drag-reordering tasks themselves
- cross-workspace task surfacing
- changing task-agent routing/model behavior
- changing message filtering rules beyond `activeTaskId` synchronization

## Risks

### Risk 1: Active-task-only selectors are too deeply baked in

Some existing components assume they always read from `activeTaskId`.

Mitigation:

- either parameterize item-level selectors by `taskId`
- or create task-id-scoped view-model helpers for sessions/artifacts/agents

### Risk 2: New task insertion without visible list refresh

If the list derives only from stale local state, users may still feel like only one task exists.

Mitigation:

- ensure workspace task list is refreshed/derived live from store tasks
- do not rely on a one-time snapshot

## Acceptance Criteria

- users can create multiple tasks in the same workspace from the task pane
- the task pane renders a list of task panels instead of only the active task
- each task panel collapses to the summary card shown in the current header area
- only one task panel is expanded at a time
- expanding a task sets it active and synchronizes the message panel
- creating a new task expands that new task and collapses the previous one
