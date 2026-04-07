# Session History UI Polish Design

## Summary

Dimweave's shared provider-history picker currently uses the generic `CyberSelect` presentation for both Claude and Codex. That works for short labels, but session titles and ids are much longer, so the dropdown looks cramped and visually broken in the advanced panel. The message search affordance is also always occupying its own header row when enabled, and the "Back to bottom" pill uses a heavy filled background that clashes with the rest of the chat chrome.

This design keeps all existing data flow and provider-history behavior intact. It only polishes the frontend presentation for the three issues the user called out.

## Product Goal

- Make the Claude/Codex History dropdown look stable and readable for long session entries.
- Keep search hidden by default, with only a header search icon visible until the user opens it.
- Make the "Back to bottom" control transparent instead of using a filled background.

## Scope

### Included

- Styling and layout changes for the shared `CyberSelect` history variant.
- Applying the same history-picker polish to both `ClaudePanel` and `CodexPanel`.
- Updating the message header search affordance to a disclosure flow.
- Updating the chat "Back to bottom" button to a transparent visual treatment.
- Focused frontend tests for the new history-select and message-panel presentation states.
- Plan/CM documentation for this work.

### Excluded

- Changing provider-history fetch, resume, or attach behavior.
- Adding search inside the session dropdown itself.
- Reworking the advanced-panel structure outside the History control.
- Changing message filtering logic or scroll-follow behavior.

## Architecture

### Shared history-select polish

The root cause, inferred from the current code and the screenshot, is that `CyberSelect` is optimized for short generic options while provider-history entries carry much longer text. The fix should stay shared instead of forking Claude and Codex behavior:

- extend `CyberSelect` with a dedicated history-oriented presentation mode
- keep generic selects unchanged
- let the history mode use a wider menu, more forgiving row spacing, and a trigger layout that does not try to show too much metadata in the collapsed control

This keeps the dropdown behavior in one place while letting the provider-history pickers opt into the more robust UI.

### Search disclosure flow

The search affordance should remain in the message header, but only as an icon in the resting state. Clicking it reveals the existing search input directly below the header. Closing the disclosure collapses the row completely so search is not permanently occupying layout space.

The filtering logic stays unchanged; only the disclosure/presentation changes.

### Back-to-bottom presentation

The existing bottom-jump control should keep its behavior and click target, but lose the filled pill styling. It becomes a transparent/ghost-style control so it reads like lightweight chat chrome instead of a primary CTA.

## Behavior Changes

### History dropdown

- Claude and Codex History selectors use the same improved history-select variant.
- Long session titles no longer feel squeezed into the generic compact layout.
- Secondary metadata remains available in the menu without making the closed trigger look crowded.

### Message search

- Header shows only a search icon by default.
- Clicking the icon reveals the search input beneath the header.
- Closing search fully removes the row again.

### Back to bottom

- The button remains visible only when the list is not at the bottom.
- Its background becomes transparent while preserving readability and clickability.

## File Plan

### Modified files

- `src/components/ui/cyber-select.tsx`
- `src/components/ui/cyber-select.test.tsx`
- `src/components/ClaudePanel/index.tsx`
- `src/components/AgentStatus/CodexPanel.tsx`
- `src/components/MessagePanel/index.tsx`
- `src/components/MessagePanel/MessageList.tsx`

### New files

- `src/components/MessagePanel/presentational.test.tsx`

## Testing Strategy

- Extend the existing `CyberSelect` test file with coverage for the history-select presentation mode.
- Add focused message-panel presentation tests that check:
  - closed search state shows only the icon
  - open search state renders the input row
  - the back-to-bottom control uses transparent styling
- Re-run the focused frontend tests plus `bun run build`.

## Acceptance Criteria

- Both Claude and Codex advanced panels show the polished History dropdown.
- The shared history dropdown no longer looks visually broken for long session rows.
- Message search is collapsed to a header icon until opened.
- The search input appears below the header only while search is open.
- The "Back to bottom" button is transparent instead of filled.
- Focused frontend tests and `bun run build` pass.
