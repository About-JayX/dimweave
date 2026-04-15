# Task Agent Dialog Style And Provider Dropdowns Design

> **Status:** Proposed

> **Context:** The `New Task` / `Edit Task` dialog is already unified into a two-pane layout, but the current right pane still looks like a generic form instead of the previous provider-panel visual language. It also still allows configuration paths that are too loose for reliable startup behavior.

## Goal

Refine the unified task-agent dialog so it matches the prior provider-panel quality bar and constrains configuration to valid provider-specific options:

- left `Agents` list rows show provider logo/icon and compact summary data
- the first agent row exists by default, can remain blank, and cannot be deleted
- the right-side config pane uses the visual style of the older provider panels
- `provider`, `model`, and `effort` are all dropdowns
- `model` remains separate from `provider` and is unselected by default
- dropdown options change based on the selected provider and supported capabilities
- history selection keeps the original semantics:
  - no history selected = new session
  - selected history item = resume that history

## Why The Current UI Is Wrong

Two problems remain in the current dialog.

### 1. The styling is weaker than the provider panels it replaced

The right pane currently reads like a plain form. It does not carry the same visual hierarchy as the earlier Claude/Codex panels:

- weak section framing
- low summary density
- no provider-specific card feel

That makes the new dialog feel like a regression even though the structure is better.

### 2. Free-form configuration is still too permissive

If `model` or `effort` are treated as loose inputs instead of provider-scoped selections, users can construct invalid parameter combinations. That is a real product problem because invalid combos may fail at launch time.

## Product Decision

### Left Pane

The left pane remains the ordered `Agents` list, but each row becomes a richer summary row.

Each row shows:

- provider logo/icon
- provider name
- role
- selected model, or placeholder if unset
- selected effort, when present
- history summary:
  - `new session` when no history is selected
  - a short history/session label when one is selected

The first row is special:

- it exists by default
- it may stay empty until the user fills it
- it cannot be deleted

Additional rows:

- can be added
- can be reordered
- can be removed

### Right Pane

The right pane keeps the current “one selected agent at a time” editing model, but visually it should look like the earlier provider panels:

- stronger bordered card
- clearer header
- grouped controls
- concise helper text
- more legible control hierarchy

This is a style reuse, not a literal reuse of the older panel component tree.

## Field Model

### Provider

`provider` remains a dedicated dropdown and selects only the provider family, for example:

- `claude`
- `codex`

### Model

`model` is a separate dropdown.

It is not preselected by default for a new agent.

Options come from the selected provider’s actual available models.

### Effort

`effort` is also a dropdown.

Its option set depends on:

- the selected provider
- and, when required, the selected model

If a provider or provider/model combination does not support effort selection, the control should be hidden or disabled rather than left as a free-form field.

### History

History selection should preserve the existing mental model:

- empty history selection means `new session`
- selecting a history entry means resume that history

There should be no separate `Resume` toggle.

## Interaction Model

### New Task

- opens with one default locked row in the left pane
- the first row may have no provider/model/role yet
- selecting the row shows its config on the right
- users add more rows as needed
- `Create` / `Create & Connect` use the final ordered list

### Edit Task

- opens with the task’s current ordered agent list
- first row still follows the “cannot delete” rule if it is row 1
- reordering and edits update the same unified data model

## Architecture

### Data Flow

The dialog continues to own one ordered list of agent drafts plus one selected-agent editor.

The refinement in this design is not a new state model. It is:

- stronger UI composition
- richer row summaries
- provider-driven dropdown option derivation

### Reuse

The implementation should reuse:

- existing brand icons where available
- existing provider model / effort knowledge already present in the UI layer
- the current ordered submit path

But it should not resurrect the old standalone provider/runtime block.

## Non-Goals

- changing task-card layout again
- changing task ordering
- adding `Sessions` / `Artifacts` back to the pane
- redesigning routing, SQLite persistence, or Telegram behavior

## Risks

### Risk 1: A default first row may feel like pre-filled data

Mitigation:

- keep the row present but visibly empty/incomplete
- distinguish “required first slot” from “already configured agent”

### Risk 2: Provider capabilities drift from actual runtime support

Mitigation:

- source dropdown options from the same UI/runtime knowledge already used elsewhere for provider controls
- do not hardcode broad free-form text inputs where strict option lists exist

## Acceptance Criteria

- left agent rows include provider logo/icon and compact summary information
- the first row exists by default and cannot be deleted
- the right pane visually matches the quality and grouping style of the earlier provider panels
- `provider`, `model`, and `effort` are dropdowns rather than free-form inputs
- `model` is separate from `provider` and starts unselected
- option sets adapt correctly by provider, and history selection preserves the original `empty = new session` semantics
