# Chat Draft Start Bottom Align Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure the chat viewport pins to the newest bottom edge as soon as the draft bubble starts and while it continues to grow, instead of only catching up after the final bubble completes.

**Architecture:** Keep the sticky-bottom controller from the prior chat autoscroll refactor, but change the draft-growth anchor strategy. When draft scrolling is needed and the real scroller element exists, scroll to `scrollHeight` directly rather than `scrollToIndex("LAST")`, because the latter only guarantees the last item is visible, not that its growing bottom edge is visible.

**Tech Stack:** React, TypeScript, react-virtuoso, Bun.

---

## Baseline Evidence

- Isolated worktree: `.claude/worktrees/chat-autoscroll-draft-start-fix` on branch `worktree-chat-autoscroll-draft-start-fix`
- User-reported real-env repro after prior merge:
  - bubble only scrolls fully into view after completion
  - bubble start/growth does not keep the viewport pinned to the latest bottom
- Root cause evidence:
  - `scrollToIndex("LAST")` only ensures the last item is present in viewport
  - when the draft row is already visible and continues growing, its bottom edge can extend below the viewport without changing index visibility

## Project Memory

### Recent related commits

- `c409552c` — initial sticky-bottom refactor
- `799c74f1` — generalized scroll-away detection
- `e1aa5ea3` — direction-only sticky clearing
- `ba9f8f7c` — programmatic-scroll immunity gate

### Lessons that constrain this plan

- Keep the existing sticky / search / immunity behavior intact.
- Fix only the draft start/growth bottom anchoring path.
- Prefer a directly testable pure helper for semantic choice when DOM assertions are limited.

## File Map

- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`

## CM Memory

| Task | Commit | Verification | Memory |
|------|--------|--------------|--------|
| Task 1 | `d1defd8d` fix: use scrollTo(scrollHeight) for draft anchor instead of scrollToIndex | `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx` ✅ 42 passed; `bun run build` ✅ | Replace draft anchor `scrollToIndex("LAST")` with absolute scroller-bottom pinning when the scroller element exists. This keeps the growing bottom edge of the draft bubble visible during streaming. |

---

### Task 1: Pin draft start/growth to absolute scroller bottom

**task_id:** `chat-autoscroll-draft-bottom-align-fix`

**Acceptance criteria:**

- Draft start scrolls to the newest bottom immediately when sticky mode is active.
- Draft growth continues pinning to the bottom edge during streaming.
- User scroll-away, sticky logic, immunity gate, and search freeze continue to behave as before.
- Existing message panel tests and build remain green.

**allowed_files:**

- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/MessageList.test.tsx`

**max_files_changed:** `2`

**max_added_loc:** `60`

**max_deleted_loc:** `25`

**verification_commands:**

- `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx`
- `bun run build`
