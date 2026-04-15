# Task Agent Dialog Live UX Fix Design

> **Status:** Proposed

> **Context:** The task-agent dialog has already been unified into a two-pane layout, restyled, and partially aligned with the older provider panels. The user still reports three live-product defects:
>
> 1. `role` remains a free-form text field instead of a constrained dropdown
> 2. dropdown controls do not visually match the old panel style
> 3. Codex model options do not appear in the live dialog even though component tests passed

## Goal

Close the remaining product gaps in the live task-agent dialog by:

- making `role` a fixed dropdown with only `lead` and `coder`
- replacing the dialog’s native `<select>` controls with the same styled dropdown component family used by the old provider panels
- wiring live Codex model / reasoning data into the dialog so the Codex model dropdown is populated in the running app

## Why The Current UI Is Wrong

The current dialog is structurally improved, but it is still not equivalent to the old provider-panel experience.

### 1. `role` is too permissive

The dialog still lets users type arbitrary role text. That is inconsistent with the current product expectation for this surface, where the active choices are only:

- `lead`
- `coder`

### 2. The select styling regressed

The old provider panels used `CyberSelect`, which has a stronger visual treatment and consistent menu behavior. The new dialog currently uses native `<select>` styling, which makes the UI feel like a downgrade.

### 3. Codex live integration is incomplete

The dialog component can accept Codex model data through props, but the live caller path does not currently prove that those options are supplied from the running store. That is why a component-level review passed while the real app still showed an empty Codex model dropdown.

## Product Decision

### Role

`role` becomes a dropdown with exactly two values:

- `lead`
- `coder`

No free-form role entry remains in this dialog.

### Dropdown Styling

The dialog should use the same select treatment as the old provider panels:

- `CyberSelect` for standard provider/model/effort selection
- `CyberSelect` history variant for session/history selection where appropriate

This is not just a cosmetic preference. It ensures the dialog and provider panels share the same interaction language.

### Codex Live Data

Codex model and reasoning choices must be sourced from the live Codex account/model store in the real app path, not only in tests.

That means the live caller must provide:

- model list
- reasoning options derived from the selected model

If the data is not available yet, the UI should show a correct loading/empty state rather than an empty broken control.

## Interaction Model

### Role Selection

- the user selects `lead` or `coder` from a dropdown
- the summary row on the left updates immediately

### Codex Model

- when provider is `codex`, model options come from the live Codex model source
- reasoning/effort options update from the selected Codex model’s supported reasoning levels
- if models are not loaded yet, the field communicates that state explicitly

### Claude Model

- Claude continues to use the same option semantics as the old Claude panel

## Architecture

### Shared UI Component Use

The dialog should stop using native `<select>` for the live configuration controls and instead use:

- `CyberSelect`
- the same option objects / display conventions used by the older provider panels

### Live Data Path

The acceptance path must include the actual caller chain:

- task pane / dialog open path
- Codex account store model availability
- dialog props / derived options

That is the missing integration boundary from the earlier accepted follow-ups.

## Non-Goals

- changing the two-pane layout again
- changing the default locked first row rule
- changing task-card layout
- changing sorting behavior
- redesigning routing or persistence

## Acceptance Criteria

- `role` is a dropdown with only `lead` and `coder`
- provider/model/effort/history selectors use the old panel’s stronger styled select treatment
- the live dialog shows Codex model options when Codex models are available in the running app
- component tests and integration-level verification both cover the live data path, not only isolated leaf components
