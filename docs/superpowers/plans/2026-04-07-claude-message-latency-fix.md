# Claude Message Latency Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Claude’s live draft appear inline in the chat timeline so the user sees message progress as soon as `stream_event` preview text is available, instead of waiting for the terminal final message.

**Architecture:** Keep the existing Claude SDK transport and preview batching pipeline unchanged. The fix is a UI-model change: Claude’s transient live draft moves from the footer-only stream rail into the actual message timeline as a synthetic timeline item, while the persisted final Claude message path stays exactly the same. Codex keeps its existing footer stream rail behavior in this change set.

**Tech Stack:** React, TypeScript, Zustand, react-virtuoso, Bun tests

---

## Verified Root Cause Summary

- Claude Code 2.1.89 already provides partial stream data when launched with `--verbose --include-partial-messages --output-format stream-json`.
- Dimweave already receives and stores that data in `claudeStream.previewText`.
- The visible latency comes from presentation: Claude preview is rendered only as a transient footer rail (`ClaudeStreamIndicator`), while real Claude chat messages are intentionally suppressed until terminal status in `event_handler_delivery.rs`.
- Therefore the highest-leverage fix is to move the Claude draft into the message timeline, not to rewrite transport.

## File Map

- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/view-model.test.ts`
- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`
- Modify: `CLAUDE.md`

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `41765645` | `manual diff review` | `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx`; `git diff --check HEAD~1 HEAD` | Lock the target UX with explicit RED tests before moving Claude draft rendering out of the footer rail. |

## Task 1: Lock the intended timeline behavior with failing frontend tests

**Acceptance criteria:**
- Tests describe one extra inline Claude timeline item while Claude is thinking.
- Tests confirm Codex remains footer-only.
- Tests fail before the implementation change.

**Files:**
- Modify: `src/components/MessagePanel/view-model.test.ts`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`

**Planned CM:** `test: lock Claude inline draft timeline behavior`

- [x] **Step 1: Add a failing view-model test for Claude draft timeline items**

Add tests that describe the target timeline model:

```ts
import { describe, expect, test } from "bun:test";
import { getMessageListDisplayState } from "./view-model";

describe("getMessageListDisplayState", () => {
  test("adds one inline Claude draft item when Claude is thinking", () => {
    const state = getMessageListDisplayState({
      messageCount: 2,
      hasClaudeDraft: true,
      streamRailIndicators: ["codex"],
    });

    expect(state.timelineCount).toBe(3);
    expect(state.streamRailIndicators).toEqual(["codex"]);
    expect(state.hasContent).toBe(true);
  });

  test("does not inflate timeline count for footer-only codex indicators", () => {
    const state = getMessageListDisplayState({
      messageCount: 2,
      hasClaudeDraft: false,
      streamRailIndicators: ["codex"],
    });

    expect(state.timelineCount).toBe(2);
  });
});
```

- [x] **Step 2: Run the focused view-model test to verify RED**

Run:

```bash
bun test src/components/MessagePanel/view-model.test.ts
```

Expected: FAIL because `getMessageListDisplayState()` does not yet accept the object shape / Claude-draft signal.

- [x] **Step 3: Add a failing message-list rendering test**

Extend `MessageList.test.tsx` with a rendering-level expectation:

```tsx
test("renders Claude working draft inline when only stream state is active", async () => {
  installTauriStub();
  const [{ MessageList }, { useBridgeStore }] = await Promise.all([
    import("./MessageList"),
    import("@/stores/bridge-store"),
  ]);

  useBridgeStore.setState((state) => ({
    ...state,
    claudeStream: {
      thinking: true,
      previewText: "Reviewing the daemon event path",
      lastUpdatedAt: 1,
    },
  }));

  const html = renderToStaticMarkup(<MessageList messages={[]} />);

  expect(html).toContain("Reviewing the daemon event path");
  expect(html).toContain("working draft");
});
```

- [x] **Step 4: Run the focused message-list test to verify RED**

Run:

```bash
bun test src/components/MessagePanel/MessageList.test.tsx
```

Expected: FAIL because Claude’s draft still renders in the footer rail path, not as an inline timeline item.

- [x] **Step 5: Commit the red tests**

```bash
git add src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx
git commit -m "test: lock Claude inline draft timeline behavior"
```

- [x] **Step 6: Update `## CM Memory`**

## Task 2: Move Claude’s live draft from the footer rail into the timeline

**Acceptance criteria:**
- Claude draft renders inline in the timeline while `claudeStream.thinking` is true.
- Codex keeps the footer rail.
- No persisted-message semantics change.

**Files:**
- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/MessageList.tsx`

**Planned CM:** `fix: inline Claude draft into message timeline`

- [ ] **Step 1: Update the message-list display-state contract**

Change `getMessageListDisplayState()` to accept explicit display inputs:

```ts
export interface MessageListDisplayStateInput {
  messageCount: number;
  hasClaudeDraft: boolean;
  streamRailIndicators: StreamIndicatorId[];
}

export function getMessageListDisplayState(
  input: MessageListDisplayStateInput,
): MessageListDisplayState {
  const { messageCount, hasClaudeDraft, streamRailIndicators } = input;
  return {
    timelineCount: messageCount + (hasClaudeDraft ? 1 : 0),
    streamRailIndicators,
    hasContent:
      messageCount > 0 || hasClaudeDraft || streamRailIndicators.length > 0,
  };
}
```

- [ ] **Step 2: Make Claude a timeline item instead of a footer indicator**

In `MessageList.tsx`, compute Claude draft visibility from store and reserve the last timeline row for `ClaudeStreamIndicator`:

```tsx
const claudeThinking = useBridgeStore((s) => s.claudeStream.thinking);
const claudePreviewText = useBridgeStore((s) => s.claudeStream.previewText);
const hasClaudeDraft = claudeThinking || claudePreviewText.length > 0;

const streamRailIndicators = useMemo(
  () => [
    ...(codexVisible ? (["codex"] as const) : []),
  ],
  [codexVisible],
);

const displayState = useMemo(
  () =>
    getMessageListDisplayState({
      messageCount: messages.length,
      hasClaudeDraft,
      streamRailIndicators,
    }),
  [messages.length, hasClaudeDraft, streamRailIndicators],
);

itemContent={(index) => {
  const isClaudeDraftRow = hasClaudeDraft && index === messages.length;
  if (isClaudeDraftRow) {
    return (
      <div className="px-4">
        <ClaudeStreamIndicator />
      </div>
    );
  }
  return (
    <div className="px-4">
      <MessageBubble msg={messages[index]} onOpenImage={onOpenImage} />
    </div>
  );
}}
```

- [ ] **Step 3: Keep the footer rail for Codex only**

Ensure `StreamTailFooter` only renders `CodexStreamIndicator` entries:

```tsx
{indicators.map((indicator) =>
  indicator === "codex" ? (
    <CodexStreamIndicator key={indicator} />
  ) : null,
)}
```

There should no longer be a footer Claude rail after this task.

- [ ] **Step 4: Run focused tests to verify GREEN**

Run:

```bash
bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx
```

Expected: PASS.

- [ ] **Step 5: Run the frontend build for regression safety**

Run:

```bash
bun run build
```

Expected: PASS.

- [ ] **Step 6: Commit the implementation**

```bash
git add src/components/MessagePanel/view-model.ts src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.tsx src/components/MessagePanel/MessageList.test.tsx
git commit -m "fix: inline Claude draft into message timeline"
```

## Task 3: Correct the Claude architecture notes so they match reality

**Acceptance criteria:**
- `CLAUDE.md` no longer claims that the frontend does not display `claude_stream.preview` text.
- The note clearly distinguishes transient draft rendering from persisted final Claude messages.

**Files:**
- Modify: `CLAUDE.md`

**Planned CM:** `docs: refresh Claude streaming UX notes`

- [ ] **Step 1: Update the stale limitation note**

Replace the current limitation text with wording consistent with the implemented behavior:

```md
- Claude SDK `stream_event` preview text is rendered as a transient inline draft in the chat timeline.
- Persisted Claude chat messages still finalize on bridge terminal reply or SDK terminal result ownership.
- The transient draft is UI-only state; it does not create an extra persisted task/session message.
```

- [ ] **Step 2: Verify the stale wording is gone**

Run:

```bash
rg -n "不展示 `claude_stream.preview`|只消费稳定的 `thinking…` / 最终结果" CLAUDE.md
```

Expected: no matches.

- [ ] **Step 3: Commit the doc correction**

```bash
git add CLAUDE.md
git commit -m "docs: refresh Claude streaming UX notes"
```

## Final Verification

- [ ] **Step 1: Run the focused frontend test set**

```bash
bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx
```

- [ ] **Step 2: Run the production frontend build**

```bash
bun run build
```

- [ ] **Step 3: Run diff hygiene**

```bash
git diff --check
```

## Done Criteria

- Claude live draft is visible inline in the chat timeline.
- The visible delay is no longer dominated by “final message only” presentation.
- Codex footer rail behavior is unchanged.
- No new persisted-message duplication is introduced.
- Docs reflect the real Claude streaming UX.
