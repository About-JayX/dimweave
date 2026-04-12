# Chat Autoscroll Claude-Style Refactor Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the brittle `followOutput + atBottom state` chat auto-scroll policy with a Claude-style sticky-bottom controller that distinguishes user-initiated scroll-away from passive content growth.

**Architecture:** Keep Virtuoso for rendering, but move transcript follow policy out of `followOutput` state gating. Introduce a DOM-driven sticky-bottom controller using refs and layout-time measurement so passive draft growth still follows, while deliberate user scroll-up pauses follow until the user returns to bottom or clicks the back-to-bottom affordance.

**Tech Stack:** React, TypeScript, react-virtuoso, Bun.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/chat-autoscroll-claude-style` on branch `fix/chat-autoscroll-claude-style`
- Baseline verification passed:
  - `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx`
  - `bun run build`
- Root-cause evidence already confirmed:
  - `51497e5e` introduced `followOutput={getMessageListFollowOutputMode(searchActive, atBottom)}`
  - passive draft growth can flip `atBottom` false without user intent
  - stopgap `173d51c9` restores follow but now risks yanking users from history when they intentionally scroll up
- Claude Code reverse-engineering evidence:
  - VSCode bundle stores bottom-stickiness in refs, not React state
  - scroll decisions are made from synchronous DOM reads (`scrollTop / scrollHeight / clientHeight`)
  - no special wheel/selection cases; the architecture itself distinguishes user scroll-away from passive growth

## Project Memory

### Recent related commits

- `173d51c9` — stopgap unconditional non-search follow
- `51497e5e` — introduced stale `atBottom` gating bug
- `1622e419` — floating back-to-bottom button
- `0c1db752` — inline Claude draft row into the message timeline

### Lessons that constrain this plan

- Search mode must still freeze auto-follow.
- Do not regress the back-to-bottom button.
- Do not modify bridge event routing or message reduction.
- This is a front-end scroll-policy refactor; keep the write scope tight.

## File Map

- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`
- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/view-model.test.ts`
- Modify: `src/components/MessagePanel/index.test.tsx` (only if needed for regression coverage)

## CM Memory

| Task | Commit | Verification | Memory |
|------|--------|--------------|--------|
| Task 1 — initial refactor | `c409552c` fix: refactor chat auto-scroll to track sticky bottom via refs | 31 tests passed; build ✅ | Replaced `useState` atBottom with `useRef` stickyRef + wheel listener + followOutputFn callback. |
| Task 1 — rework #1 | `799c74f1` fix: generalize scroll-away detection beyond wheel-only | 31 tests passed; build ✅ | Added pointerdown + scroll intent heuristic to cover scrollbar drag. |
| Task 1 — rework #2 (final) | `e1aa5ea3` fix: use scroll direction instead of pointer intent for scroll-away | 31 tests passed; build ✅ | Replaced intent-based (wheel+pointerdown+timer) with scroll-direction detection. Only upward scroll clears sticky — eliminates click-to-select false positives while covering all user scroll methods. |

---

### Task 1: Refactor transcript follow policy to Claude-style sticky-bottom control

**task_id:** `chat-autoscroll-claude-style-refactor`

**Acceptance criteria:**

- Passive content growth (Claude draft expansion) does not break auto-follow.
- User-initiated scroll-up pauses auto-follow until they return to bottom or click back-to-bottom.
- Search-active mode still disables auto-follow.
- Back-to-bottom button behavior remains correct.
- Existing tests and build stay green.

**allowed_files:**

- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/MessageList.test.tsx`
- `src/components/MessagePanel/view-model.ts`
- `src/components/MessagePanel/view-model.test.ts`
- `src/components/MessagePanel/index.test.tsx`

**max_files_changed:** `5`

**max_added_loc:** `180`

**max_deleted_loc:** `90`

**verification_commands:**

- `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing tests first**

Add regression coverage for:
- non-search + passive growth should still follow
- explicit user scroll-away should pause follow
- search-active still freezes follow

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal sticky-bottom controller**

Expected implementation shape:
- replace React-state-based follow gating with ref-based sticky-bottom tracking
- measure bottom distance from the actual scroller DOM
- distinguish user scroll events from passive layout/content changes
- trigger scroll-to-bottom in layout effects when sticky mode is still active
- keep Virtuoso rendering; do not rewrite the whole list

Do not:
- touch bridge-store or daemon message flow
- change search semantics
- bundle unrelated UI cleanup

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src/components/MessagePanel/MessageList.tsx \
  src/components/MessagePanel/MessageList.test.tsx \
  src/components/MessagePanel/view-model.ts \
  src/components/MessagePanel/view-model.test.ts \
  src/components/MessagePanel/index.test.tsx
git commit -m "fix: refactor chat auto-scroll to track sticky bottom"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
