# Chat Timeline And Inspector Bugfix Design

## Summary

This bugfix wave will repair five related UI issues in the chat shell:

1. expanded `thinking` content breaks the chat bubble layout
2. `Back to bottom` does not reach the real visual bottom
3. search should live as a persistent row after opening, not only as a header swap
4. history session selection should show title and task id on separate lines
5. the current `Task context` inspector feels over-heavy and mismatched with the rest of the product

The fixes stay frontend-only and focus on restoring a clean IDE-like conversation flow.

## Product Goal

- Keep the conversation timeline visually continuous.
- Make transient stream state feel attached to the active conversation, not detached from it.
- Ensure bottom-navigation targets the true bottom of the visible chat surface.
- Improve session recognition when resuming history.
- Rework the task inspector into a lighter, more project-consistent information layout.

## Scope

### In scope

- `MessagePanel` search layout changes
- `MessageList` bottom-anchor behavior changes
- `ClaudeStreamIndicator` and `CodexStreamIndicator` presentation changes
- provider-history option rendering changes in the select UI
- `TaskPanel` information architecture and visual hierarchy cleanup

### Out of scope

- daemon or transport changes
- new backend task/session fields
- a full shell navigation rewrite

## Root Cause Summary

### 1. Thinking bubble collapse

`thinking` / working-draft state is rendered in a separate rail below the virtualized list, so it visually detaches from the previous assistant bubble. When expanded, it reads like a second panel instead of a continuation of the active reply.

### 2. Back-to-bottom mismatch

The bottom action scrolls to the last virtualized message row, but the stream indicator rail lives outside that list. The button therefore reaches the end of the timeline data, not the actual rendered bottom of the chat surface.

### 3. Search row behavior

Search currently toggles inside the message header itself. Once opened, the control displaces header content instead of creating a stable search row.

### 4. History session readability

Provider history options currently expose only a single-line label, which makes similar sessions hard to distinguish.

### 5. Task context mismatch

`TaskPanel` currently uses duplicated labels, dashboard counters, and heavy card nesting. It feels more like an internal diagnostics panel than a focused project inspector.

## Design Decision

### A. Make stream state an inline timeline tail

Transient Claude/Codex stream indicators should render as the tail of the chat timeline rather than a detached footer rail. Visually, they should sit directly under the previous assistant region with the same horizontal rhythm as normal bubbles.

This change is the preferred fix over keeping a detached rail and only adjusting spacing, because the user explicitly wants `thinking` to feel connected to the prior bubble.

### B. Make bottom navigation target the rendered tail

`Back to bottom` should scroll to the true final rendered item, including any active inline stream tail. The button should remain visually above the composer and should not stop early because of separate footer content.

### C. Keep search trigger in the header, open a dedicated row below it

The compact search icon remains in the header as the trigger. After activation, the header stays intact and a dedicated search row appears below it until closed.

### D. Upgrade provider history options to two-line metadata

Each history option should show:

- first line: session title
- second line: normalized task id when present, otherwise the external id

This keeps the picker compact while making similar sessions distinguishable.

### E. Recompose Task context into a lighter inspector

`Task context` should be restructured into three clearer layers:

1. a compact task summary block
2. session list as the primary operational section
3. artifact timeline as the secondary evidence section

The redesign should remove redundant counters and reduce visual weight so the panel better matches the shell's calmer IDE direction.

## Frontend Design Notes

### Message timeline

- render stream indicators as part of the message flow
- keep bubble widths and left alignment consistent with existing assistant bubbles
- avoid creating a visually separate footer zone for stream state

### Search

- preserve the header icon button
- add a persistent search row beneath the header while search is open
- keep result summary adjacent to the row, not mixed into the icon-only state

### Provider history select

- extend select option rendering to support label + description
- ensure collapsed state still shows a compact readable summary

### Task inspector

- keep section titles concise
- reduce repeated “Task context / Session context / Active sessions” phrasing
- favor readable identity metadata over dashboard numerics

## Testing Strategy

- update message-panel render tests for inline stream-tail behavior
- add or update tests for search-row rendering
- add tests for two-line provider-history option rendering
- update task-panel rendering tests to match the lighter inspector structure

## Acceptance Criteria

- Expanding `thinking` no longer creates a detached broken-looking block.
- `thinking` content reads as a continuation at the bottom of the conversation flow.
- `Back to bottom` reaches the actual rendered bottom of the chat area.
- Opening search keeps the header icon and reveals a dedicated persistent row.
- History session options show title on line one and task id on line two when available.
- `Task context` presents cleaner, lighter, project-aligned structure and copy.
