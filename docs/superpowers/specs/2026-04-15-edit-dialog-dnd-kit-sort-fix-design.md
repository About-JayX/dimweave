# Edit Dialog DnD-Kit Sort Fix Design

> **Status:** Proposed

> **Context:** The task pane is already card-only, and `Edit Task` is now the only in-pane agent-management surface. The first drag-reorder implementation used native HTML drag events and passed synthetic tests, but it does not behave reliably in the real Tauri app.

## Goal

Make agent reordering inside `Edit Task` reliably work in the real app by replacing the current native drag implementation with a library-backed sortable interaction.

## Why The Current UI Is Wrong

The current edit dialog exposes rows that appear draggable, but the real app does not consistently reorder them.

The underlying problem is structural:

- the dialog uses native HTML drag events
- the implementation depends on local index bookkeeping rather than a robust sortable abstraction
- the current interaction test simulates synthetic drag events and therefore does not prove that the real WebView behavior is stable

So the bug is not “some styling is missing.” The current interaction contract is incomplete.

## Product Decision

Use `dnd-kit` to implement dialog row sorting.

Specifically:

- `@dnd-kit/core`
- `@dnd-kit/sortable`
- `@dnd-kit/utilities`

The drag affordance should be restricted to the grip handle rather than the full row. That avoids conflicts with the provider select and role input.

## Interaction Model

### Edit Mode

- each agent row remains editable
- each row has a visible drag handle
- only the drag handle starts sort interaction
- dragging a row reorders rows immediately in dialog state
- `Save` persists the final order through the existing `reorderTaskAgents` path

### Create Mode

Create mode is not being redesigned here.

It keeps its current add/remove row behavior and provider panel flow. Shared row markup can be reused if useful, but create mode should not suddenly become a full sortable editor unless that falls out naturally from the reused implementation and stays low-risk.

## Architecture

### Dialog

`TaskSetupDialog.tsx` stays the dialog shell.

The edit-mode agent list should move from native `draggable` DOM events to a `dnd-kit` sortable list. Each row should have:

- stable item id based on `agentId` when present
- a temporary stable client id for newly added rows without persisted `agentId`

That keeps ordering stable before save.

### Save Path

`TaskPanel/index.tsx` already handles add/update/remove and then calls `reorderTaskAgents`.

That path remains correct in principle and should be preserved. The fix is about making the dialog produce a reliable ordered payload from real drag behavior, not about inventing a second persistence mechanism.

## Non-Goals

- redesigning task card chrome again
- reintroducing inline agent management into the task card
- changing task ordering
- changing `Sessions` / `Artifacts`
- changing Telegram or SQLite behavior

## Risks

### Risk 1: New unsaved rows lose stable ordering keys

Rows created during the dialog session may not yet have `agentId`.

Mitigation:

- generate dialog-local stable ids for unsaved rows
- use those ids only for sortable interaction, not as persisted ids

### Risk 2: Dependency addition expands scope

Adding `dnd-kit` touches package metadata and test/runtime behavior.

Mitigation:

- keep the dependency addition tightly scoped to this fix
- verify with the existing dialog tests plus focused reorder coverage

## Acceptance Criteria

- `Edit Task` dialog drag reorder works reliably in the real app
- drag start is constrained to the grip handle
- saved order persists through the existing `reorderTaskAgents` path
- task card agent pills continue reflecting persisted order
- create mode behavior is not regressed
