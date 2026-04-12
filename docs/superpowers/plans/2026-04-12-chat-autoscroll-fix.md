# Chat Autoscroll Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore automatic scrolling to the latest chat bubble when search is inactive, so new Claude/Codex output remains visible without manual scrolling.

**Architecture:** Keep the search freeze behavior introduced by `51497e5e`, but stop using `atBottom` to disable follow mode in normal chat usage. This is a minimal view-model fix: in non-search mode the message list always follows output smoothly; only active search disables follow.

**Tech Stack:** React, TypeScript, react-virtuoso, Bun.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/chat-autoscroll-fix` on branch `fix/chat-autoscroll-fix`
- Baseline verification passed:
  - `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx`
  - `bun run build`
- Root-cause evidence:
  - `51497e5e` changed `followOutput` from unconditional `"smooth"` to `getMessageListFollowOutputMode(searchActive, atBottom)`
  - `ClaudeStreamIndicator` grows in height during streaming without increasing `timelineCount`
  - Virtuoso updates `atBottom` to `false`
  - `getMessageListFollowOutputMode(false, false)` currently returns `false`, disabling auto-follow for later bubbles

## Project Memory

### Recent related commits

- `51497e5e` — freeze message auto-scroll during active search
- `0cdeb12c` — define search-active message scroll policy
- `0c1db752` — inline Claude draft into message timeline

### Lessons that constrain this plan

- Preserve search behavior: when search is active, auto-scroll must remain disabled.
- Do not change message rendering or bridge event flow.
- This is a minimal view-model fix only.

## File Map

- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/view-model.test.ts`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: restore chat auto-follow outside search` | `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx`; `bun run build` | Remove the stale `atBottom` gate from message-list follow mode so streaming bubble growth does not permanently disable auto-scroll. |

---

### Task 1: Restore auto-follow when search is inactive

**task_id:** `chat-autoscroll-fix`

**Acceptance criteria:**

- Non-search mode always returns `"smooth"` from `getMessageListFollowOutputMode(...)`.
- Search-active mode still returns `false`.
- Existing message-list tests and build stay green.

**allowed_files:**

- `src/components/MessagePanel/view-model.ts`
- `src/components/MessagePanel/view-model.test.ts`

**max_files_changed:** `2`

**max_added_loc:** `40`

**max_deleted_loc:** `20`

**verification_commands:**

- `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx`
- `bun run build`

- [ ] **Step 1: Write the failing regression test**
- [ ] **Step 2: Run verification and confirm failure**
- [ ] **Step 3: Implement the minimal fix**
- [ ] **Step 4: Re-run verification**
- [ ] **Step 5: Commit**

```bash
git add \
  src/components/MessagePanel/view-model.ts \
  src/components/MessagePanel/view-model.test.ts
git commit -m "fix: restore chat auto-follow outside search"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
