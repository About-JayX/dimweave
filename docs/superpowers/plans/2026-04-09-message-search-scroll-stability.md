# Message Search Scroll Stability Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the chat timeline from flickering and auto-jumping while the user is actively filtering messages with search.

**Architecture:** Keep search query ownership in `MessagePanel`, derive a single `searchActive` flag from the trimmed deferred query, move message-list scroll policy into explicit view-model helpers, and have `MessageList` disable automated bottom-follow / zero-result reset behavior while search is active.

**Tech Stack:** React 19, TypeScript, react-virtuoso, bun test, Vite build

---

## File Map

### Modified files

- `docs/superpowers/specs/2026-04-09-message-search-scroll-stability-design.md`
- `src/components/MessagePanel/index.tsx`
- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/MessageList.test.tsx`
- `src/components/MessagePanel/view-model.ts`
- `src/components/MessagePanel/view-model.test.ts`

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `36cb7ad5` | `self-review` | `git diff --check -- docs/superpowers/specs/2026-04-09-message-search-scroll-stability-design.md docs/superpowers/plans/2026-04-09-message-search-scroll-stability.md` ✅ | The approved scroll policy must be documented before code changes so implementation and review use the same definition of "search-stable". |
| Task 2 | `PENDING` | `manual review` | `bun test src/components/MessagePanel/view-model.test.ts`; `git diff --check` | Search-active scroll policy belongs in testable helpers, not inline conditionals scattered through JSX. |
| Task 3 | `PENDING` | `manual review` | `bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx`; `bun run build`; `git diff --check` | Active search is an inspection mode: freeze auto-follow until the user clears the query or explicitly scrolls back down. |

## Baseline Notes

- Baseline in this worktree on `2026-04-09`:
  - `bun run build` ✅
  - `bun test src/components/MessagePanel/index.test.tsx src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/view-model.test.ts` ✅
  - `bun test src/components/MessagePanel/presentational.test.tsx` ❌ because the file still asserts the older transparent back-to-bottom background. That pre-existing failure is outside this bugfix scope and must not be misreported as introduced by this plan.

### Task 1: Record the approved design and execution contract

**Acceptance criteria**

- The design doc captures the verified root cause, recommended approach, scope boundaries, and verification strategy.
- The plan defines exact file paths, task-level acceptance criteria, verification commands, and CM tracking.

**Files:**
- Create: `docs/superpowers/specs/2026-04-09-message-search-scroll-stability-design.md`
- Create: `docs/superpowers/plans/2026-04-09-message-search-scroll-stability.md`

- [x] **Step 1: Write the design document**

Document the verified causes:

- search changes `filteredMessages.length`
- `MessageList` keeps `followOutput="smooth"`
- `react-virtuoso` documents count-change follow behavior
- zero-result searches currently re-arm the initial scroll jump

- [x] **Step 2: Write the execution plan**

Include:

- exact file list
- task acceptance criteria
- verification commands
- `## CM Memory`
- baseline note about the unrelated pre-existing presentational test failure

- [x] **Step 3: Verify doc formatting**

Run:

```bash
git diff --check -- docs/superpowers/specs/2026-04-09-message-search-scroll-stability-design.md docs/superpowers/plans/2026-04-09-message-search-scroll-stability.md
```

Expected: no whitespace or patch-format issues.

- [x] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-04-09-message-search-scroll-stability-design.md docs/superpowers/plans/2026-04-09-message-search-scroll-stability.md
git commit -m "docs: record message search scroll stability plan"
```

- [x] **Step 5: Update `## CM Memory`**

Replace the Task 1 placeholders with the real commit hash and verification evidence before code changes start.

### Task 2: Add explicit search-active scroll policy helpers

**Acceptance criteria**

- Search-active detection is derived from trimmed query text.
- Helper tests prove active search disables automated follow behavior.
- Helper tests prove zero-result searches do not reset the initial-scroll guard.

**Files:**
- Modify: `src/components/MessagePanel/view-model.ts`
- Modify: `src/components/MessagePanel/view-model.test.ts`

- [ ] **Step 1: Write the failing helper tests**

Extend `src/components/MessagePanel/view-model.test.ts` with focused cases like:

```ts
test("active search disables message-list follow output", () => {
  expect(getMessageListFollowOutputMode(true, true)).toBe(false);
});

test("zero-result search does not reset initial scroll state", () => {
  expect(shouldResetMessageListInitialScroll(true, 0)).toBe(false);
  expect(shouldResetMessageListInitialScroll(false, 0)).toBe(true);
});
```

Run:

```bash
bun test src/components/MessagePanel/view-model.test.ts
```

Expected: FAIL because the helpers do not exist yet.

- [ ] **Step 2: Implement the minimal helpers**

Add explicit exports in `src/components/MessagePanel/view-model.ts`:

```ts
export function isMessageSearchActive(searchQuery: string): boolean {
  return searchQuery.trim().length > 0;
}

export function getMessageListFollowOutputMode(
  searchActive: boolean,
  atBottom: boolean,
): false | "smooth" {
  return searchActive ? false : atBottom ? "smooth" : false;
}

export function shouldResetMessageListInitialScroll(
  searchActive: boolean,
  totalCount: number,
): boolean {
  return !searchActive && totalCount === 0;
}
```

- [ ] **Step 3: Re-run the helper tests**

Run:

```bash
bun test src/components/MessagePanel/view-model.test.ts
```

Expected: PASS.

- [ ] **Step 4: Verify diff hygiene**

Run:

```bash
git diff --check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/MessagePanel/view-model.ts src/components/MessagePanel/view-model.test.ts
git commit -m "fix: define search-active message scroll policy"
```

- [ ] **Step 6: Update `## CM Memory`**

Replace the Task 2 placeholders with the real commit hash and verification evidence.

### Task 3: Wire MessageList to freeze auto-follow during active search

**Acceptance criteria**

- `MessagePanel` passes a derived `searchActive` flag into `MessageList`.
- `MessageList` disables `Virtuoso` follow output while search is active.
- `MessageList` keeps the zero-result branch from resetting the initial-scroll guard during active search.
- Focused message-panel tests and `bun run build` pass.

**Files:**
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`

- [ ] **Step 1: Write the failing wiring regression test**

Extend `src/components/MessagePanel/MessageList.test.tsx` so the mocked `Virtuoso` captures the latest props:

```tsx
test("disables followOutput while search is active", async () => {
  installTauriStub();
  const { MessageList } = await import("./MessageList");

  renderToStaticMarkup(
    <MessageList
      messages={[{
        id: "msg_1",
        from: "claude",
        to: "user",
        content: "Found the root cause",
        timestamp: 1,
      }]}
      searchActive={true}
    />,
  );

  expect(lastVirtuosoProps?.followOutput).toBe(false);
});
```

Also keep a control assertion for `searchActive={false}` expecting `"smooth"`.

Run:

```bash
bun test src/components/MessagePanel/MessageList.test.tsx
```

Expected: FAIL because `MessageList` does not yet accept `searchActive`.

- [ ] **Step 2: Implement the wiring**

Update `src/components/MessagePanel/index.tsx`:

```tsx
const searchActive = isMessageSearchActive(deferredSearchQuery);

<MessageList
  messages={filteredMessages}
  searchActive={searchActive}
  emptyStateMessage={searchSummary ?? undefined}
  onOpenImage={setLightboxAttachment}
/>
```

Update `src/components/MessagePanel/MessageList.tsx`:

```tsx
interface Props {
  emptyStateMessage?: string;
  messages: BridgeMessage[];
  searchActive?: boolean;
  onOpenImage?: (attachment: Attachment) => void;
}

const followOutput = getMessageListFollowOutputMode(searchActive, atBottom);

useEffect(() => {
  if (shouldResetMessageListInitialScroll(searchActive, totalCount)) {
    didInitialScrollRef.current = false;
    return;
  }
  if (searchActive || totalCount === 0 || didInitialScrollRef.current) return;
  didInitialScrollRef.current = true;
  const raf = window.requestAnimationFrame(() => {
    virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "auto" });
  });
  return () => window.cancelAnimationFrame(raf);
}, [searchActive, totalCount]);

<Virtuoso followOutput={followOutput} ... />
```

- [ ] **Step 3: Re-run the focused regressions**

Run:

```bash
bun test src/components/MessagePanel/view-model.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx
```

Expected: PASS.

- [ ] **Step 4: Re-run build and diff verification**

Run:

```bash
bun run build
git diff --check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/MessagePanel/index.tsx src/components/MessagePanel/MessageList.tsx src/components/MessagePanel/MessageList.test.tsx
git commit -m "fix: freeze message auto-scroll during active search"
```

- [ ] **Step 6: Update `## CM Memory`**

Replace the Task 3 placeholders with the real commit hash and verification evidence.
