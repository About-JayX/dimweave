# Task Agent List And Config Dialog Unification Design

> **Status:** Accepted

> **Context:** The task pane is already card-only, the task card is now the only always-visible task surface, and `Edit Task` is the only in-pane entry point for task-agent management. The current create/edit dialog still mixes an `Agents` list with a second provider/runtime block, which does not match the desired product flow.

## Goal

Unify `New Task` and `Edit Task` into the same two-pane agent-management dialog:

- left side shows the ordered agent list only
- right side shows configuration for exactly one selected or newly-added agent
- `Add Agent` creates a new editable agent config instead of exposing a second provider/runtime section
- `provider` is chosen first
- `model` is a separate field and starts unselected
- `role`, `model`, `effort`, and session-related controls change based on the chosen provider and current mode

## Why The Current UI Is Wrong

The current dialog still exposes two different mental models at once:

- an `Agents` list at the top
- a separate provider/runtime control area below

That creates two product problems:

### 1. Create and edit are not actually the same workflow

The current create flow still feels like:

- define task agents in one place
- configure provider runtime in another place

That separation makes create mode feel structurally different from edit mode.

### 2. “Agent” and “runtime config” are visually split even though they belong to the same object

In the desired model, an agent owns:

- provider
- role
- model
- effort or equivalent reasoning control when supported
- session behavior (`new` or `resume`) when supported

So showing agent rows in one place and runtime config in another place is a mismatch between UI structure and data structure.

## Product Decision

### Shared Create/Edit Surface

`New Task` and `Edit Task` use the same layout and interaction model.

Differences are limited to:

- initial data
- footer button copy

Everything else stays the same.

### Left Pane: Agent List

The left side is an ordered list of current agents for the task.

Each row should:

- be draggable for ordering
- be selectable
- support edit and remove actions
- display a compact summary only

Recommended row summary fields:

- provider
- role
- model
- effort, when present
- session mode summary (`New session` or `Resume <id/title>`)

The list starts empty in `New Task`.

### Right Pane: Agent Config

The right side edits one agent at a time.

Two entry paths:

- `Add Agent` creates a new draft agent and selects it
- clicking edit on an existing row loads that row’s config

The right-side form should support:

- `provider` dropdown
- `role` input
- `model` selector
- `effort` selector, when supported by the chosen provider
- session mode selector (`new` / `resume`), when supported
- resume session picker, when `resume` is chosen

## Field Rules

### Provider

`provider` is a standalone dropdown.

It selects the provider family only, for example:

- `claude`
- `codex`

### Model

`model` is separate from `provider`.

It is not preselected by default. The user chooses it explicitly.

### Conditional Fields

The right panel should only show fields that make sense for the selected provider and current choices.

Examples:

- show `effort` only if the provider exposes it
- show resume-specific fields only when session mode is `resume`
- keep the form stable when switching provider, but clear invalid selections

## Interaction Model

### New Task

- dialog opens with an empty agent list
- right pane initially shows an empty placeholder state
- user clicks `Add Agent`
- a new draft agent appears and becomes selected
- user configures it in the right pane
- user can add more agents, sort them, then `Create` or `Create & Connect`

### Edit Task

- dialog opens with the existing ordered agent list
- first agent may be auto-selected, or no selection if preferred by implementation simplicity
- user edits one agent at a time in the right pane
- row order can be changed from the left pane
- `Save` persists both config updates and final order

## Architecture

### Dialog Structure

`TaskSetupDialog.tsx` should become the unified shell for both create and edit.

It should own:

- selected agent identity
- ordered list of agent drafts
- right-pane draft editing state
- submit assembly for create/edit flows

### Existing Task Card

The task card remains unchanged by this design except for continuing to render agent pills in persisted order.

This work is strictly about the dialog experience.

### Existing Agent Components

The old `TaskAgentEditor` should not continue as a parallel editing surface if the unified dialog replaces it functionally.

It is acceptable either to:

- reuse internal logic from it
- or leave it unused temporarily if removing it is out of scope

But the live create/edit flow should converge on one dialog surface.

## Non-Goals

- reintroducing `Sessions` or `Artifacts` to the task pane
- redesigning task card chrome again
- changing task ordering rules
- changing message routing or reply targeting
- redesigning provider connection semantics beyond the fields surfaced in the agent form

## Risks

### Risk 1: Create flow becomes too heavy

Adding more fields into the right pane could make `New Task` feel slower.

Mitigation:

- keep the left list summary compact
- show only provider-relevant fields
- keep the right pane focused on one selected agent at a time

### Risk 2: Provider switches leave invalid config behind

Switching provider after a model/session choice may leave stale incompatible values.

Mitigation:

- explicitly clear or reset fields that are not valid under the newly selected provider
- cover provider-switch behavior in tests

## Acceptance Criteria

- `New Task` and `Edit Task` use the same two-pane structure
- left pane is only an ordered agent list
- right pane configures one selected agent at a time
- `provider` is a dropdown and `model` is a separate field with no default selection
- `role`, `model`, `effort`, and session controls adapt to the chosen provider and relevant mode
- the old separate provider/runtime block is removed from the dialog
