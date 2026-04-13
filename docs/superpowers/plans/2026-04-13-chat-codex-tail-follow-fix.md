# Chat Codex Tail Follow Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the viewport follow the Codex thinking/footer tail as soon as it appears, while preserving the draft bubble bottom-pinning and user scroll-away protections.

**Architecture:** Split the chat bottom-pin strategy by UI structure. Growing draft rows should continue using absolute scroller-bottom pinning; footer/tail indicators such as Codex thinking should use a `LAST`-style tail follow path because they are appended as footer content rather than growing inline items.

**Tech Stack:** React, TypeScript, react-virtuoso, Bun.

---

## Baseline Evidence

- Isolated worktree: `.claude/worktrees/chat-codex-tail-follow-fix` on branch `worktree-chat-codex-tail-follow-fix`
- Baseline verification passed:
  - `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx`
  - `bun run build`
- User repro after prior merge:
  - Claude draft bubble now pins correctly during growth
  - Codex thinking/tail still does not move viewport to the latest position when it starts
- Current structural reason:
  - Claude draft is an inline extra timeline row
  - Codex thinking is rendered in `StreamTailFooter`, not as a timeline item
  - Current bottom-pin compensation only covers Claude draft start/growth

## Project Memory

### Recent related commits

- `d1defd8d` — switched draft anchor from `scrollToIndex("LAST")` to `scrollTo(scrollHeight)`
- `ba9f8f7c` — added programmatic-scroll immunity gate
- `e1aa5ea3` — direction-based sticky clearing

### Lessons that constrain this plan

- Keep sticky / immunity / search-freeze behavior intact.
- Do not regress the Claude draft fix.
- Fix only the missing Codex tail-follow path.

## File Map

- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`
- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/view-model.test.ts`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: follow codex tail indicators with last-item pinning` | `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx`; `bun run build` | Split bottom-pin behavior by structure: draft bubble growth uses scroller bottom, footer/tail indicators use last-item/tail pinning. |

---

### Task 1: Add dedicated tail-follow path for Codex thinking/footer indicators

**task_id:** `chat-codex-tail-follow-fix`

**Acceptance criteria:**

- Codex thinking/footer tail moves the viewport to the latest position when it appears, if sticky mode is active.
- Claude draft bubble growth fix remains intact.
- User scroll-away and search freeze remain intact.
- Tests and build remain green.

**allowed_files:**

- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/MessageList.test.tsx`
- `src/components/MessagePanel/view-model.ts`
- `src/components/MessagePanel/view-model.test.ts`

**max_files_changed:** `4`

**max_added_loc:** `80`

**max_deleted_loc:** `30`

**verification_commands:**

- `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx`
- `bun run build`
