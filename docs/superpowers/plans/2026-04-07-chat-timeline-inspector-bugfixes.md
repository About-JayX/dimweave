# Chat Timeline And Inspector Bugfixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the reported chat-shell regressions so transient thinking stays attached to the conversation, bottom navigation reaches the true bottom, search opens as a dedicated row, provider history selection is easier to recognize, and task context matches the product's lighter inspector style.

**Architecture:** Keep all changes frontend-only. Repair the conversation flow by moving transient stream UI into the message timeline itself, make search and history selection more readable through component-level UI changes, and simplify the task inspector by demoting dashboard-like chrome in favor of identity-first sections.

**Tech Stack:** React, TypeScript, Tailwind CSS, react-virtuoso, Bun tests

---

## File Map

- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`
- Modify: `src/components/MessagePanel/index.test.tsx`
- Modify: `src/components/ui/cyber-select.tsx`
- Modify: `src/components/AgentStatus/provider-session-view-model.ts`
- Add: `src/components/ui/cyber-select.test.tsx`
- Modify: `src/components/TaskPanel/index.tsx`
- Modify: `src/components/TaskPanel/TaskHeader.tsx`
- Modify: `src/components/TaskPanel/SessionTree.tsx`
- Modify: `src/components/TaskPanel/ArtifactTimeline.tsx`
- Modify: `src/components/TaskContextPopover.test.tsx`
- Modify: `src/components/TaskPanel/ArtifactTimeline.test.tsx`

---

### Task 1: Inline transient stream state into the chat tail and fix bottom anchoring

**Acceptance criteria:**
- Active Claude/Codex thinking appears as the tail of the message timeline rather than a detached footer zone.
- The stream tail is visually aligned like the rest of the assistant-side conversation.
- `Back to bottom` reaches the true rendered bottom even while a transient stream tail is visible.
- Existing empty-state behavior still works when there are no messages and no active stream indicators.

**Files:**
- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`

- [x] **Step 1: Add a focused regression test for inline stream-tail rendering**

Extend `MessageList.test.tsx` with a render case that seeds `claudeStream` or `codexStream`, renders `MessageList` with at least one real message, and asserts the stream indicator markup appears inside the list container rather than in a detached sibling footer wrapper.

- [x] **Step 2: Render stream indicators through the Virtuoso footer instead of a detached section**

In `MessageList.tsx`, keep `totalCount` bound to real messages but move `displayState.streamRailIndicators` into a `Virtuoso` footer renderer so the active stream UI is part of the scrollable chat flow. Keep message rows unchanged:

```tsx
<Virtuoso
  totalCount={messages.length}
  components={{
    Footer: () => (
      <div className="px-4 pb-2">
        {displayState.streamRailIndicators.map(...)}
        <div ref={footerAnchorRef} />
      </div>
    ),
  }}
/>
```

- [x] **Step 3: Make the bottom button target the true rendered tail**

Replace the current `scrollToIndex({ index: "LAST" })`-only behavior with footer-anchor scrolling when a stream footer is mounted, falling back to the last message only when no footer exists:

```tsx
if (footerAnchorRef.current) {
  footerAnchorRef.current.scrollIntoView({ behavior: "smooth", block: "end" });
} else {
  virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "smooth" });
}
```

- [x] **Step 4: Keep view-model semantics minimal**

Leave `getMessageListDisplayState()` message-count semantics tied to real chat messages, but preserve `streamRailIndicators` for deciding whether the footer should render.

- [x] **Step 5: Verify**

Run:

```bash
bun test src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/CodexStreamIndicator.test.ts
```

Expected: all tests pass.

Lead verification: `bun test src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/CodexStreamIndicator.test.ts` → `11 pass, 0 fail` on 2026-04-07.

- [x] **CM:** `fix: inline transient stream tail into message timeline` — commit `cd0bdd52`

---

### Task 2: Keep the search icon in the header and open a dedicated search row

**Acceptance criteria:**
- The search trigger remains visible in the header.
- Clicking search opens a persistent row beneath the header instead of replacing the entire header content.
- Closing search clears the query and removes the row.
- Search summary only appears while the search row is open.

**Files:**
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/MessagePanel/index.test.tsx`

- [x] **Step 1: Add a render test covering the open-search row layout**

Update `index.test.tsx` with a case that seeds at least one message, renders the panel, and verifies the default header shows the search trigger but not the inline search input. Add a second case that exercises the open-state markup through extracted rendering logic or stateful test setup and asserts the search row is present under the header border.

- [x] **Step 2: Split the current header into two layers**

Keep the compact header bar for actions only, and when `searchOpen` is true, render a second bordered row below it containing the input, summary, and close action:

```tsx
<div className="border-b ...">
  <div className="flex items-center justify-end px-4 py-1.5">...</div>
  {searchOpen && <div className="border-t ... px-4 py-2">...</div>}
</div>
```

- [x] **Step 3: Keep current focus and reset behavior**

Preserve the existing autofocus and `requestAnimationFrame` focus behavior when opening search. Closing search must still call:

```tsx
setSearchQuery("");
setSearchOpen(false);
```

- [x] **Step 4: Verify**

Run:

```bash
bun test src/components/MessagePanel/index.test.tsx src/components/MessagePanel/MessageList.test.tsx
```

Expected: all tests pass.

Lead verification: `bun test src/components/MessagePanel/index.test.tsx src/components/MessagePanel/MessageList.test.tsx` → `9 pass, 0 fail` on 2026-04-07.

- [x] **CM:** `fix: move message search into dedicated panel row` — commit `05645621`

---

### Task 3: Show provider history selection as two-line metadata in the select

**Acceptance criteria:**
- History options render title on the first line.
- The second line shows normalized task id when available, otherwise the external session id.
- The collapsed selected value also shows two lines for history entries.
- Non-history selects that only provide a label continue to render compactly.

**Files:**
- Modify: `src/components/ui/cyber-select.tsx`
- Modify: `src/components/AgentStatus/provider-session-view-model.ts`
- Add: `src/components/ui/cyber-select.test.tsx`

- [x] **Step 1: Extend the option model to support description text**

Keep the existing shape but actually use the optional `description?: string` field for history entries:

```ts
export interface CyberSelectOption {
  value: string;
  label: string;
  description?: string;
}
```

- [x] **Step 2: Populate description for provider history items**

In `buildProviderHistoryOptions()`, preserve `"New session"` as a single-line option and map history entries to:

```ts
{
  value: entry.externalId,
  label: entry.title?.trim() || `${provider} session`,
  description: entry.normalizedTaskId ?? entry.externalId,
}
```

- [x] **Step 3: Update CyberSelect rendering for optional two-line options**

Render the selected value and dropdown options with stacked text when `description` exists, but keep the old single-line compact path when it does not:

```tsx
{selected?.description ? (
  <span className="flex min-w-0 flex-col text-left">
    <span className="truncate font-medium">{selected.label}</span>
    <span className="truncate text-[9px] text-muted-foreground">{selected.description}</span>
  </span>
) : (
  <span className="truncate max-w-28">{displayLabel}</span>
)}
```

- [x] **Step 4: Add a focused component test**

Create `src/components/ui/cyber-select.test.tsx` that renders one single-line option and one two-line option, then asserts that:
- the two-line selected state contains both label and description
- the single-line option does not render an unnecessary second line

- [x] **Step 5: Verify**

Run:

```bash
bun test src/components/ui/cyber-select.test.tsx
```

Expected: all tests pass.

Lead verification: `bun test src/components/ui/cyber-select.test.tsx` → `5 pass, 0 fail` on 2026-04-07.

- [x] **CM:** `fix: show provider history with title and task metadata` — commit `554550d6`

---

### Task 4: Recompose Task context into a lighter inspector layout

**Acceptance criteria:**
- The task inspector reads as a compact summary + sessions + artifacts structure.
- Redundant counters and repeated “context” wording are removed or reduced.
- Task identity information is easier to scan than before.
- Existing artifact detail behavior remains intact.

**Files:**
- Modify: `src/components/TaskPanel/index.tsx`
- Modify: `src/components/TaskPanel/TaskHeader.tsx`
- Modify: `src/components/TaskPanel/SessionTree.tsx`
- Modify: `src/components/TaskPanel/ArtifactTimeline.tsx`
- Modify: `src/components/TaskContextPopover.test.tsx`
- Modify: `src/components/TaskPanel/ArtifactTimeline.test.tsx`

- [x] **Step 1: Simplify the top-level TaskPanel structure**

Remove the current dashboard-style metric grid from `TaskPanel/index.tsx`. Keep one compact summary card followed by the sessions and artifacts sections:

```tsx
<section className="space-y-3">
  <TaskHeader ... />
  <div className="rounded-2xl ..."><SessionTree ... /></div>
  <div className="rounded-2xl ..."><ArtifactTimeline ... /></div>
</section>
```

- [x] **Step 2: Rewrite TaskHeader around identity-first metadata**

Adjust `TaskHeader.tsx` so the summary emphasizes:
- task title
- status badge
- workspace path
- task id
- review badge when present

Do not reintroduce large counters or duplicate section labels.

- [x] **Step 3: Lighten the section copy inside SessionTree and ArtifactTimeline**

Rename headings to shorter inspector-style labels such as `Sessions` and `Artifacts`, keep metadata readable, and preserve resume / selection actions. Avoid repeated all-caps dashboard language except for small section eyebrows where already used consistently.

- [x] **Step 4: Update tests to match the lighter structure**

Adjust `TaskContextPopover.test.tsx` and `ArtifactTimeline.test.tsx` so they assert the new lighter copy without expecting removed dashboard wording.

- [x] **Step 5: Verify**

Run:

```bash
bun test src/components/TaskContextPopover.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx
```

Expected: all tests pass.

Lead verification:
- `bun test src/components/TaskContextPopover.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx src/components/TaskPanel/TaskHeader.test.tsx` → `10 pass, 0 fail` on 2026-04-07
- `bun test src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx src/components/MessagePanel/CodexStreamIndicator.test.ts src/components/ui/cyber-select.test.tsx src/components/TaskContextPopover.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx` → `27 pass, 0 fail` on 2026-04-07
- `bun run build` → success on 2026-04-07

- [x] **CM:** `refactor: simplify task inspector structure` — commit `238b1d54`

---

## Final verification

After Task 4 is complete, run the focused frontend regression suite for all touched areas:

```bash
bun test \
  src/components/MessagePanel/MessageList.test.tsx \
  src/components/MessagePanel/index.test.tsx \
  src/components/MessagePanel/CodexStreamIndicator.test.ts \
  src/components/ui/cyber-select.test.tsx \
  src/components/TaskContextPopover.test.tsx \
  src/components/TaskPanel/ArtifactTimeline.test.tsx
```

Expected: all tests pass before final review.
