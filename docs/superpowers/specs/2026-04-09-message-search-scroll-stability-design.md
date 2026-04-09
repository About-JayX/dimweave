# Message Search Scroll Stability Design

## Summary

The chat message search flow currently causes severe viewport flicker while the user types. The current `MessageList` keeps `react-virtuoso` in `followOutput="smooth"` mode even when the visible message list is being filtered by the search query, and the `react-virtuoso` type contract says `followOutput` scrolls when the total count changes. In this codepath, every search update changes `filteredMessages.length`, so the list can keep trying to animate itself while the user is searching.

There is a second re-arming path in `MessageList`: when `totalCount === 0`, the component resets `didInitialScrollRef`. During search, a zero-match query can therefore re-enable the one-time initial bottom jump, so when the next query produces matches again the list can snap back to the end of the timeline instead of preserving the user's current viewport.

This fix keeps normal live-follow behavior for the chat timeline when search is inactive, but freezes automated scrolling while a non-empty search query is active so the search viewport stays visually stable.

## Evidence

- `src/components/MessagePanel/index.tsx` filters chat rows with `filterMessagesByQuery(chatMessages, deferredSearchQuery)`, so search updates directly change the rendered message count.
- `src/components/MessagePanel/MessageList.tsx` currently renders `<Virtuoso followOutput="smooth" ... />`, which means search result count changes can trigger animated bottom-follow behavior.
- `node_modules/react-virtuoso/dist/index.d.ts` documents `followOutput` as scrolling when `totalCount` changes and allows returning `false` to suppress that behavior.
- `src/components/MessagePanel/MessageList.tsx` resets `didInitialScrollRef` whenever `totalCount === 0`, which can happen transiently for zero-result searches.
- Baseline verification on `2026-04-09` in the isolated worktree:
  - `bun run build` ✅
  - `bun test src/components/MessagePanel/index.test.tsx src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/view-model.test.ts` ✅
  - `bun test src/components/MessagePanel/presentational.test.tsx` ❌ pre-existing stale assertions still expect the older transparent back-to-bottom styling; this plan does not change that visual treatment.

## Product Goal

- Keep the message list visually stable while a user is actively filtering chat messages.
- Preserve the existing automatic bottom-follow behavior for normal live chat updates outside active search.
- Prevent zero-result searches from re-arming the initial auto-scroll jump.

## Scope

### Included

- Message-panel search filtering behavior in chat mode.
- Shared scroll-behavior helpers for message-search state.
- Focused regression tests covering search-active scroll behavior.
- Design/plan/CM documentation for the fix.

### Excluded

- Log-surface scrolling behavior.
- Search-result navigation, highlighting, or jumping to the first match.
- Message-search disclosure layout or button styling.
- Repairing the unrelated pre-existing `presentational.test.tsx` background assertions.

## Options Considered

### Option 1: Freeze automated scrolling while search is active (recommended)

Treat any non-empty search query as an explicit "inspection mode". While active, disable `followOutput` and skip any logic that re-arms the one-time initial bottom jump. Resume the normal bottom-follow rules immediately after the query clears.

**Pros**
- Directly matches the user complaint.
- Minimal change surface.
- Keeps default live-chat behavior untouched outside active search.

**Cons**
- New live messages will not auto-scroll into view while search is active.

### Option 2: Keep followOutput, but downgrade from `"smooth"` to `"auto"` during search

This removes the animation but still allows search result count changes to reposition the list.

**Pros**
- Very small code change.

**Cons**
- Does not actually preserve the user's viewport.
- Still causes jumps when the filtered count changes.

### Option 3: Auto-jump to the first search match on every query change

This reframes search as a result-navigation flow rather than a filter-preservation flow.

**Pros**
- Makes match location explicit.

**Cons**
- Larger UX change than requested.
- More complex than needed for the current bug.
- Can feel even more aggressive than the current flicker.

## Recommended Design

Use Option 1.

Define search-active behavior from the trimmed, effective search query (`query.trim().length > 0`). When search is active:

- `MessageList` must not ask `react-virtuoso` to follow output.
- `MessageList` must not clear the initial-scroll guard when search temporarily yields zero visible rows.
- The current viewport should stay where the user left it until search is cleared or they explicitly press "Back to bottom".

When search is inactive, keep the existing live-chat behavior:

- initial load can jump to the latest message once
- live stream output can continue using the current bottom-follow behavior

## Architecture

### Search state ownership

`MessagePanel` already computes `deferredSearchQuery` and owns the filtered message list. It should also derive a single `searchActive` boolean from that query and pass it down to `MessageList`.

### Scroll policy helpers

Keep the policy in `src/components/MessagePanel/view-model.ts` so the logic is unit-testable instead of being buried in JSX:

- `isMessageSearchActive(searchQuery)`
- `getMessageListFollowOutputMode(searchActive, atBottom)`
- `shouldResetMessageListInitialScroll(searchActive, totalCount)`

This keeps the scroll rules explicit and gives the tests a small surface to validate.

### MessageList behavior

`MessageList` should consume `searchActive` and:

- compute `followOutput` through the new helper instead of hard-coding `"smooth"`
- skip the zero-count reset path while search is active
- keep the existing manual "Back to bottom" affordance unchanged

## File Plan

### Modified files

- `src/components/MessagePanel/index.tsx`
- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/MessageList.test.tsx`
- `src/components/MessagePanel/view-model.ts`
- `src/components/MessagePanel/view-model.test.ts`

### New files

- None

## Testing Strategy

- Add unit coverage for the new search-active scroll helpers in `view-model.test.ts`.
- Extend `MessageList.test.tsx` to assert the `Virtuoso` `followOutput` prop is disabled while search is active and remains smooth when search is inactive.
- Re-run focused message-panel tests:
  - `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx`
- Re-run `bun run build`.
- Keep the pre-existing `presentational.test.tsx` failure explicitly out of task acceptance because it is unrelated baseline noise.

## Acceptance Criteria

- Typing a non-empty message search query no longer causes the chat list to animate toward the bottom.
- Transitioning through zero-result search states does not re-arm the initial auto-scroll jump.
- Clearing the query restores the existing non-search bottom-follow behavior.
- Focused regression tests and `bun run build` pass.
