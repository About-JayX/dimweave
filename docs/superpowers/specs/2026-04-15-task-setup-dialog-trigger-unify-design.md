# Task Setup Dialog Trigger Unify Design

## Summary

The task setup dialog still renders one visually broken control group in the right pane:

- `Provider`, `Role`, `Model`, and `Effort` use compact trigger pills
- `Session` uses the shared history-select trigger, which expands to fill the row

That leaves the dialog with mismatched trigger widths, mismatched chrome, and a `Session` control that visually dominates the card. The user wants the full dialog control set to read as one unified family of dropdown triggers.

## Product Goal

- Make `Provider`, `Role`, `Model`, `Effort`, and `Session` look like one consistent trigger family in the task setup dialog.
- Remove the full-width `Session` trigger treatment in the dialog.
- Preserve middle-ellipsis behavior for long selected session titles.
- Keep dropdown menu row content unchanged.

## Scope

### Included

- Trigger styling and width behavior for dialog dropdown controls in the right pane of `TaskSetupDialog`.
- Shared `CyberSelect` support needed to let dialog controls share one trigger style while still preserving menu behavior differences.
- Focused tests for dialog trigger width/style consistency and history-trigger truncation in the dialog context.
- Plan and CM documentation for this follow-up.

### Excluded

- Any change to dropdown menu item content, spacing, or option sourcing.
- Any change to task setup dialog layout outside the control rows.
- Any change to provider-history loading, resume semantics, or stored values.

## Root Cause

The current mismatch is structural, not cosmetic.

In `src/components/ui/cyber-select.tsx`:

- the history trigger path uses `flex min-w-0 flex-1`
- the history button itself also uses `flex-1`
- standard triggers use compact `inline-flex` sizing

So in `TaskSetupDialog`, the `Session` row always receives a fill-width trigger while the other rows render compact pills. The screenshot the user provided is consistent with this exact layout contract.

## Product Decision

### Unified dialog trigger chrome

All dropdown triggers in the task setup dialog should use one compact right-aligned visual style:

- same height
- same radius
- same border and background treatment
- same text size
- same icon spacing
- same width strategy

This includes the `Session` trigger.

### Width strategy

The dialog should no longer let the `Session` trigger fill the available row width.

Instead:

- dialog triggers should use a controlled compact width
- long labels should truncate inside that width
- `Session` should visually align with `Provider`, `Role`, `Model`, and `Effort`

### History behavior

The `Session` trigger should keep its history-specific value behavior:

- `New session` placeholder still works
- selected long history labels still use middle ellipsis
- history dropdown menu rows remain unchanged

## Architecture

### Shared component support

The correct fix stays centered in `CyberSelect`, but the visual target is the task setup dialog:

- keep menu behavior split by `variant`
- add a dialog-oriented trigger treatment that both `default` and `history` can use
- have `TaskSetupDialog` opt all of its right-pane selects into that shared trigger treatment

This avoids a hard fork between history and non-history trigger rendering while keeping the change scoped to the dialog surface the user reported.

### Dialog application

`TaskSetupDialog` should apply the same trigger treatment to:

- `Provider`
- `Role`
- `Model`
- `Effort`
- `Session`

The surrounding row layout remains `label left / trigger right`.

## Behavior Changes

### Dialog controls

- All right-pane dropdown triggers look visually consistent.
- `Session` is no longer the only full-width control.
- Long session labels still preserve the beginning and end via middle ellipsis.

### Menus

- Dropdown menu content stays unchanged.
- History-specific menu rows remain history-specific.

## File Plan

### Modified files

- `src/components/ui/cyber-select.tsx`
- `src/components/ui/cyber-select.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`

## Testing Strategy

- Extend `cyber-select` tests to cover the dialog trigger-style contract.
- Extend `TaskSetupDialog` tests to prove:
  - the dialog applies the shared trigger treatment across provider/role/model/effort/session
  - the session trigger is no longer full-width
  - long session labels still use middle ellipsis
- Re-run focused dialog/select tests plus `bun run build`.

## Acceptance Criteria

- `Provider`, `Role`, `Model`, `Effort`, and `Session` share one compact trigger style in the task setup dialog.
- The `Session` trigger no longer expands to full row width in the dialog.
- Long selected session titles still use middle ellipsis.
- Dropdown menu items remain unchanged.
- Focused frontend tests and `bun run build` pass.
