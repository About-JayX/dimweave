# UI Polish Wave

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix 4 user-reported UI issues: message bubble overflow, back-to-bottom button occlusion, session title truncation, and always-visible search bar.

**Architecture:** Frontend-only changes across MessagePanel and TaskPanel components. No backend changes.

**Tech Stack:** React, TypeScript, Tailwind CSS, react-virtuoso, lucide-react

---

## Root Cause Summary

1. **Message bubble overflow**: `MessageBubble` renders `MessageMarkdown` without height constraint. Long agent output expands the bubble unboundedly, causing Virtuoso's virtual item height to spike and breaking scroll interaction.
2. **Back to bottom button occlusion**: Button uses `absolute bottom-3` positioning within a container that also has a flow-positioned stream indicator section below the Virtuoso list. When stream indicators are visible, the button overlaps or hides behind them.
3. **Session title truncation**: `SessionTree` row layout has `flex items-center gap-2` with title using `truncate` but role/provider badges lacking `shrink-0`. Flex distributes space evenly, squeezing the title to a few characters when badges are present.
4. **Search bar always visible**: `MessagePanel` renders a full-width `<input>` search bar permanently above the message list, consuming vertical space even when unused.

## File Map

### Modified files

- `src/components/MessagePanel/MessageBubble.tsx` — wrap content in scrollable container
- `src/components/MessagePanel/MessageList.tsx` — reposition back-to-bottom button
- `src/components/MessagePanel/index.tsx` — collapsible search icon
- `src/components/TaskPanel/SessionTree.tsx` — fix flex shrink on badges

---

### Task 1: Constrain message bubble content height

**Files:**
- Modify: `src/components/MessagePanel/MessageBubble.tsx`

- [x] **Step 1: Wrap MessageMarkdown in scrollable container**

Add `max-h-[60vh] overflow-y-auto` wrapper around `<MessageMarkdown>` so long content scrolls within the bubble instead of expanding it unboundedly.

- [x] **Step 2: Verify** — frontend tests pass, long messages scroll within bubble.

### Task 2: Fix back-to-bottom button position

**Files:**
- Modify: `src/components/MessagePanel/MessageList.tsx`

- [x] **Step 1: Move button from absolute to flow layout**

Replace `absolute bottom-3` positioned button with a flow-positioned `<div className="flex justify-center py-1.5">` placed ABOVE the stream indicator section, so it's never occluded.

- [x] **Step 2: Verify** — button visible above stream indicators when scrolled up.

### Task 3: Fix session title truncation

**Files:**
- Modify: `src/components/TaskPanel/SessionTree.tsx`

- [x] **Step 1: Add shrink-0 to badges, title tooltip**

Add `shrink-0` to role and provider badge spans so they don't compress. Add `title` attribute to session title span for hover tooltip. Add `min-w-0` to title span for proper truncation.

- [x] **Step 2: Verify** — session title shows more text, hover shows full title.

### Task 4: Collapse search into icon

**Files:**
- Modify: `src/components/MessagePanel/index.tsx`

- [x] **Step 1: Add searchOpen state and lucide icons**

Import `Search` and `X` from lucide-react. Add `searchOpen` boolean state and `searchInputRef`.

- [x] **Step 2: Replace always-visible input with toggle**

Default: render a small `<Search>` icon button. On click: expand to full input with `<X>` close button. Close resets query and collapses.

- [x] **Step 3: Verify** — search icon visible, expands on click, collapses on X.

---

## Implementation (2026-04-06)

Commit trail:

| Commit | Summary |
|--------|---------|
| `76350feb` | fix: address 4 UI issues — bubble scroll, back-to-bottom, session title, search icon |

All 4 tasks implemented in a single commit. Frontend 162 tests pass, 0 failures.

Note: This plan was written retroactively to comply with the plan-first mandatory workflow added to CLAUDE.md in commit `45fe6307`. Future UI changes will follow plan-first order.

## Final Acceptance Criteria

- [x] Long message bubbles scroll within a max-height container instead of breaking the list.
- [x] Back-to-bottom button is always visible above stream indicators.
- [x] Session title in TaskPanel shows more text with tooltip on hover.
- [x] Search bar is collapsed to an icon by default, expands on click.
- [x] All existing frontend tests pass.
