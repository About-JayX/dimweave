# Task Setup Dialog Option Cleanup Design

## Summary

The task setup dialog still has two option-list bugs in the right-pane controls:

- the `Model` dropdown includes a fake `Select model` entry in the menu
- the `Effort` dropdown can show `Default` twice for Claude

These are not rendering bugs in `CyberSelect`. They come from dialog-local option assembly in `TaskSetupDialog`.

## Product Goal

- Keep `Select model` as trigger placeholder text only, not as a selectable menu option.
- Ensure `Effort` shows a single `Default` option.
- Preserve the recently unified trigger styling and current provider/history behavior.

## Scope

### Included

- Dialog-local model/effort option assembly in `TaskSetupDialog`
- Focused render tests for model placeholder behavior and effort default deduplication
- Plan and CM documentation for this fix

### Excluded

- Any trigger-style changes
- Any dropdown menu layout changes
- Any provider-history changes
- Any provider option source changes outside `TaskSetupDialog`

## Root Cause

In `src/components/TaskPanel/TaskSetupDialog.tsx`:

- `modelWithDefault = [{ value: "", label: modelPlaceholder }, ...mOpts]` injects the placeholder as a real menu option
- `effortWithDefault = [{ value: "", label: "Default" }, ...eOpts]` always prepends `Default`, even when the source options already include `{ value: "", label: "Default" }`

For Claude, `CLAUDE_EFFORT_OPTIONS` already contains its own empty default entry, so the dialog creates a duplicate.

## Product Decision

### Model

- The trigger may still display `Select model` when the value is empty.
- The model menu itself must contain only real selectable options from the provider source.

### Effort

- The menu must contain exactly one `Default` item.
- If the provider source already includes an empty-value default option, reuse it.
- If the provider source does not include one, inject a single `Default` option.

## Architecture

This fix stays local to `TaskSetupDialog`:

- keep `CyberSelect` unchanged
- normalize model/effort options before passing them into `CyberSelect`
- prove behavior with focused dialog render tests

## Acceptance Criteria

- `Model` trigger still shows `Select model` when unset, but that text is not duplicated as a menu option.
- `Effort` shows only one `Default` option.
- Trigger styling and menu layout remain unchanged.
- Focused dialog tests and `bun run build` pass.
