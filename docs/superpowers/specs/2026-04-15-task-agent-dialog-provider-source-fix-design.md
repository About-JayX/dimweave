# Task Agent Dialog Provider Source And Session Selector Fix Design

> **Status:** Proposed

> **Context:** The task-agent dialog has already been unified into a two-pane layout, styled to match the prior provider panels, and constrained to dropdown-driven model/effort selection. Two remaining mismatches are still visible compared with the older provider panels:
>
> 1. the dropdown option sources are not fully aligned with the older panel implementations
> 2. session selection was changed to radio-plus-input instead of preserving the earlier history dropdown behavior

## Goal

Bring the new task-agent dialog into semantic alignment with the older provider panels by:

- sourcing Claude model/effort options from the same logic used in the old Claude panel
- sourcing Codex model/reasoning options from the same logic used in the old Codex panel
- restoring the old history dropdown behavior so:
  - selecting no history entry means `new session`
  - selecting a history entry means resume that history
- removing the current `New session / Resume session` radio controls from the dialog

## Why The Current UI Is Wrong

The dialog is visually close to the target now, but the behavior still diverges from the known-good panel model.

### 1. Dropdown option sources are only approximately correct

The current implementation uses a local `PROVIDER_CAPS` option map inside the dialog layer. That is workable, but it is not yet the same source model as the older provider panels:

- Claude previously used `ClaudeConfigRows`
- Codex previously used `CodexPanel` model data and reasoning-level derivation

This creates a maintenance risk: dialog options may drift from the main provider controls.

### 2. Session selection semantics regressed

The old provider-panel interaction used a history dropdown where:

- `New session` was just the sentinel default option
- any selected historical entry meant “resume that one”

The current dialog changed that into:

- a radio choice between `New session` and `Resume session`
- plus a free-form id input for resume

That is a worse UX and diverges from the prior product behavior.

## Product Decision

### Model / Effort Sources

The dialog should follow the old panel sourcing model, not a purely dialog-local approximation.

Specifically:

- **Claude** keeps the old model/effort option semantics from `ClaudeConfigRows`
- **Codex** keeps the old model list and reasoning option semantics from `CodexPanel`

The dialog may still use shared helper functions, but the option content must match those older surfaces.

### Session Selection

The dialog returns to the old history-dropdown model.

Per provider:

- the dropdown first option is `New session`
- subsequent options are provider-specific history entries
- empty/default/new selection means `new session`
- selected history item means resume that item

There is no separate `Resume` radio control.

## Interaction Model

### Right Pane

The right pane remains a provider-styled config card, but the session section changes to:

- one dropdown labeled something like `Session` or `History`
- `New session` as the default option
- provider-specific history entries beneath it

### Create and Edit

Both create and edit use the same mechanism:

- choose provider
- choose model and effort from provider-valid dropdowns
- optionally choose a history item

If no history item is chosen, the resulting config means `new session`.

## Architecture

### Shared Option Builders

The preferred direction is to move toward shared option-builder helpers so the dialog and the legacy provider controls are reading from one semantic source rather than duplicating option lists in multiple files.

At minimum:

- Claude options must match the existing `ClaudeConfigRows` content
- Codex options must match the existing `CodexPanel` / `CodexConfigRows` derivation
- history options must continue using `provider-session-view-model.ts`

### History Data

The dialog should use the existing provider history machinery:

- `NEW_PROVIDER_SESSION_VALUE`
- `buildProviderHistoryOptions()`
- `findProviderHistoryEntry()`
- `resolveProviderHistoryAction()`

That avoids inventing a second session-resume representation.

## Non-Goals

- changing the two-pane layout again
- changing card styling again
- changing agent ordering
- changing task-pane card behavior
- redesigning runtime routing or persistence

## Risks

### Risk 1: Codex option data is not directly available in the dialog

The old Codex panel derives some options from account store state.

Mitigation:

- thread the required model/reasoning option sources into shared helpers or the dialog in a narrow way
- if live model fetch is unavailable in the dialog context, keep behavior consistent with the currently accepted store-backed path already used elsewhere in the app

### Risk 2: History dropdown introduces provider-store coupling into the dialog

Mitigation:

- reuse existing provider-history selectors/helpers already proven in provider panels
- keep the dialog as a consumer of that data, not a second source of truth

## Acceptance Criteria

- Claude dropdown content matches the old Claude panel semantics
- Codex dropdown content matches the old Codex panel semantics
- session selection uses the old history-dropdown model rather than radio-plus-input
- no-history-selected still means `new session`
- selecting a history option still resumes that provider history entry
