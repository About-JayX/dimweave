# Session History Trigger Polish Design

## Summary

Dimweave's shared `CyberSelect` history variant is now used in three live surfaces: `ClaudePanel`, `CodexPanel`, and the task setup dialog. The dropdown menu rows already prioritize readability, but the collapsed trigger still has two visual problems:

- selected session titles are end-truncated instead of using the approved middle-ellipsis style
- the trigger pill is too short vertically, especially in the task setup dialog

This design keeps the history menu content unchanged and only polishes the shared history trigger chrome.

## Product Goal

- Make selected history/session labels read more gracefully by preserving both the beginning and end of the title.
- Increase the visual height of the shared history trigger so it no longer looks cramped.
- Apply the polish once in the shared component so Claude, Codex, and the dialog stay aligned.

## Scope

### Included

- Styling and display logic changes for the shared `CyberSelect` history trigger.
- Applying the same trigger polish to all current `variant="history"` call sites.
- Focused frontend tests for middle ellipsis and taller history-trigger chrome.
- Plan and CM documentation for this work.

### Excluded

- Any change to history dropdown menu item content, height, or spacing.
- Any change to provider-history fetch, resume, or attach behavior.
- Any change to non-history `CyberSelect` variants.

## Architecture

### Shared history trigger polish

The correct fix is centralized in `src/components/ui/cyber-select.tsx`, not duplicated in each caller:

- keep the history dropdown menu rows exactly as they are today
- change only the collapsed history trigger text treatment and padding
- continue using the shared `variant="history"` path so every current caller inherits the same behavior

This preserves the existing provider-history data flow and avoids a dialog-only fork that would drift from the panels again.

### Text treatment

The history trigger should use middle ellipsis for long selected labels:

- preserve the beginning and end of the selected label
- remove characters from the middle
- leave short labels unchanged
- keep the existing placeholder behavior for `New session`

This matches the previously approved style direction from the April 7 history polish work, but now applies it to the collapsed trigger instead of only the menu item title.

### Sizing treatment

The history trigger should become visibly taller without changing menu rows:

- increase vertical padding and/or minimum height of the shared history trigger
- keep the rounded pill visual language
- avoid widening the menu or changing its row density as part of this fix

## Behavior Changes

### History trigger

- Selected long session titles use middle ellipsis in the closed trigger.
- `New session` remains unchanged when selected.
- The trigger pill is taller and less cramped in Claude, Codex, and TaskSetupDialog.

### History menu

- Menu row content, spacing, and readability remain unchanged.

## File Plan

### Modified files

- `src/components/ui/cyber-select.tsx`
- `src/components/ui/cyber-select.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`

## Testing Strategy

- Extend `src/components/ui/cyber-select.test.tsx` with history-trigger coverage for:
  - middle ellipsis on long selected labels
  - unchanged `New session` placeholder behavior
  - taller history-trigger class contract
- Add or update a focused dialog render test to prove the dialog inherits the shared history-trigger label treatment for a selected long history item.
- Re-run the focused tests plus `bun run build`.

## Acceptance Criteria

- All live `variant="history"` triggers use middle ellipsis for long selected labels.
- The shared history trigger is visually taller than before.
- History dropdown menu items are unchanged by this fix.
- Focused frontend tests and `bun run build` pass.
