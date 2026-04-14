# Task Card Polish And Agent Edit Dialog Design

> **Status:** Accepted

> **Context:** The task pane is now intentionally card-only. The card is the primary task-switching surface, while `Edit Task` is now the only task-pane entry point for agent management.

## Goal

Polish the compact task card and upgrade `Edit Task` into a complete agent-management dialog:

- shrink the status badge, especially `Draft`, and place it in the lower-right corner of the task card
- keep `Edit Task` as icon-only affordance in the upper-right corner
- move agent ordering into the `Edit Task` dialog
- make the dialog support add, remove, edit, and drag reorder in one place
- keep task cards themselves dense and read-only except for selection and the edit icon

## Why The Current UI Is Wrong

After the card-only simplification, two gaps remain:

### 1. Task card chrome is still using the older detail-card layout

The status badge is visually heavy and occupies prime space in the upper-right area. The edit affordance is still rendered as icon plus text, which competes with the task title and agent pills.

That layout made more sense when the card was only the header for a larger expanded panel. Now that the card is the whole surface, the chrome should be more compact.

### 2. `Edit Task` is underpowered for its new responsibility

`Edit Task` is now the only in-pane place to manage agents, but the current dialog is still a minimal row form:

- provider select
- role input
- delete button

It does not provide ordering, and it does not feel like a complete agent-management surface. That is a mismatch between product responsibility and UI capability.

## Product Decision

### Task Card

Each task card remains compact and selection-focused.

Keep:

- title
- task id
- agent pills
- saved state indicator
- status badge
- edit affordance

Change:

- `Edit Task` becomes icon-only and sits in the upper-right corner
- the status badge becomes smaller and sits in the lower-right corner
- `Draft` is treated as a low-emphasis status chip rather than a dominant badge

### Edit Dialog

The `Edit Task` dialog becomes the full agent-management surface for a task.

It should support:

- add agent
- remove agent
- edit provider
- edit role
- preserve `agentId` / `displayName`
- drag reorder agents

The dialog should display agents as draggable rows instead of plain stacked form lines.

## Interaction Model

### Task Card

- clicking the card still selects the task through `activeTaskId`
- clicking the edit icon opens the edit dialog without selecting another task
- the card itself remains non-editable beyond that icon

### Edit Dialog

- rows are shown in current persisted agent order
- dragging rows changes the visual order immediately
- `Save` applies add/update/remove/reorder in one submit path
- the task card pills reflect the saved order afterward

## Architecture

### Card Surface

`TaskHeader.tsx` remains the task card surface.

It should be responsible for:

- card layout
- icon-only edit affordance
- compact status placement
- rendering task-scoped agent pills in stored order

### Edit Flow

`TaskSetupDialog.tsx` remains the create/edit dialog shell.

For edit mode, it now needs a richer agent-list section with drag reorder. The dialog submit path must continue to preserve existing `agentId` values and also persist final ordering.

That means the current `TaskPanel/index.tsx` edit submit logic must stop at not just add/update/remove. It must also apply the final order through the existing `reorderTaskAgents` command path.

### Existing Dead Agent Components

`TaskAgentList` and `TaskAgentEditor` are already outside the live task-pane path.

This change does not need to revive them. It is acceptable to leave them alone in this pass, as long as the live edit flow is correct and tested.

## Non-Goals

- bringing back inline agent management into the task card list
- restoring `Sessions` or `Artifacts` to the task pane
- changing task ordering
- changing task selection or reply/message sync architecture
- redesigning create-mode provider panels

## Risks

### Risk 1: Edit dialog reordering updates rows visually but does not persist order

The current edit submit path in `TaskPanel/index.tsx` handles add/update/remove only.

Mitigation:

- include explicit submit-path logic that computes the final ordered agent id list
- after add/update/remove complete, call the existing reorder action with the final order
- add interaction coverage proving saved order changes the task card pill order

### Risk 2: Card chrome becomes less discoverable

Making `Edit Task` icon-only and shrinking the status chip reduces visible text.

Mitigation:

- keep a tooltip / title on the edit icon
- keep the icon placed consistently in the upper-right
- keep the status chip visible, just lower emphasis

## Acceptance Criteria

- task card shows a smaller status chip in the lower-right area
- task card uses icon-only edit affordance in the upper-right area
- `Edit Task` dialog supports add, remove, edit, and drag reorder of agents
- saving the edit dialog persists agent order and the task card pills reflect that order
- task cards remain the only always-visible surface in the task pane
