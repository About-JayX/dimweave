# Session History UI Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Polish the shared provider-history dropdown, message search disclosure, and back-to-bottom control so the session UI matches the approved compact interaction design.

**Architecture:** Keep provider-history data flow unchanged. Add a history-specific presentation mode to the shared `CyberSelect`, opt both provider panels into it, then update the message panel so search is a disclosure beneath the header and the bottom-jump control uses transparent chrome.

**Tech Stack:** React 19, TypeScript, bun test, Vite build, shared frontend components

---

## File Map

### New files

- `src/components/MessagePanel/presentational.test.tsx`
- `src/components/MessagePanel/search-chrome.tsx`

### Modified files

- `src/components/ui/cyber-select.tsx`
- `src/components/ui/cyber-select.test.tsx`
- `src/components/ClaudePanel/index.tsx`
- `src/components/AgentStatus/CodexPanel.tsx`
- `src/components/MessagePanel/index.tsx`
- `src/components/MessagePanel/MessageList.tsx`
- `docs/superpowers/specs/2026-04-07-session-history-ui-polish-design.md`

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `441e8f4a` | `self-review` | `git diff --check -- docs/superpowers/specs/2026-04-07-session-history-ui-polish-design.md docs/superpowers/plans/2026-04-07-session-history-ui-polish.md` | UI bugfix work should lock the approved interaction details in docs before any component code changes begin. |
| Task 2 | `07b1da49` | `manual review` | `bun test src/components/ui/cyber-select.test.tsx`; `bun run build`; `git diff --check` | Provider-history selectors should stay shared; styling differences belong in a variant, not duplicated Claude/Codex components. |
| Task 3 | `b442d510` | `manual review` | `bun test src/components/MessagePanel/presentational.test.tsx`; `bun run build`; `git diff --check` | Search should not permanently consume header space when the user is not actively filtering chat. |
| Task 4 | `bf6cb39d` | `manual review` | `bun test src/components/MessagePanel/presentational.test.tsx`; `bun run build`; `git diff --check` | Secondary chat affordances should read like lightweight navigation, not filled primary-action pills. |
| Task 5 | `540f33d6` | `self-review` | `bun test src/components/ui/cyber-select.test.tsx` ✅ 7 pass; `bun run build` ✅; `git diff --check` ✅ | Provider history rows must favor readability over compact truncation; export HistoryMenuOption for direct testability instead of needing a defaultOpen prop. |
| Task 6 | `92641c8f` | `self-review` | `bun test src/components/MessagePanel/presentational.test.tsx src/components/MessagePanel/index.test.tsx` ✅ 10 pass; `bun run build` ✅; `git diff --check` ✅ | Product fix needed: remove chatMessages.length > 0 guard — Zustand v5 SSR uses api.getInitialState() which cannot be reliably patched from outside the store closure; removing the guard is the correct minimal fix. |
| Task 7 | `de71e3e7` | `manual review` | `bun test src/components/MessagePanel/presentational.test.tsx -t "BackToBottomButton"` ✅ 2 pass; `bun run build` ✅; `git diff --check` ✅ | Back-to-bottom control should keep its existing chrome; only the fill treatment changes to transparent. |

### Task 1: Record the approved design and execution contract

**Files:**
- Create: `docs/superpowers/specs/2026-04-07-session-history-ui-polish-design.md`
- Create: `docs/superpowers/plans/2026-04-07-session-history-ui-polish.md`

- [x] **Step 1: Write the approved design spec**

Document:

- why the current shared history dropdown breaks down on long session text
- why both provider panels should stay on one shared history-select variant
- the search disclosure interaction
- the transparent back-to-bottom treatment

- [x] **Step 2: Write the implementation plan with CM tracking**

The plan must include:

- exact file paths
- one task per user-visible fix
- explicit verification commands
- `## CM Memory`

- [x] **Step 3: Verify doc formatting**

Run:

```bash
git diff --check -- docs/superpowers/specs/2026-04-07-session-history-ui-polish-design.md docs/superpowers/plans/2026-04-07-session-history-ui-polish.md
```

Expected: no whitespace or patch-format issues.

- [x] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-04-07-session-history-ui-polish-design.md docs/superpowers/plans/2026-04-07-session-history-ui-polish.md
git commit -m "docs: record session history ui polish plan"
```

- [x] **Step 5: Update `## CM Memory`**

Replace Task 1 placeholders with the real commit hash and verification evidence before implementation starts.

### Task 2: Polish the shared provider-history dropdown

**Files:**
- Modify: `src/components/ui/cyber-select.tsx`
- Modify: `src/components/ui/cyber-select.test.tsx`
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/AgentStatus/CodexPanel.tsx`

- [x] **Step 1: Write the failing history-select tests**

Add focused coverage for a `variant="history"` rendering mode:

```tsx
test("history variant keeps collapsed trigger compact", async () => {
  const { CyberSelect } = await import("./cyber-select");
  const html = renderToStaticMarkup(
    createElement(CyberSelect, {
      value: "hist_1",
      variant: "history",
      options: [
        {
          value: "hist_1",
          label: "A very long session title that should stay readable",
          description: "sess_abc123456789",
        },
      ],
      onChange: () => {},
    }),
  );
  expect(html).toContain("A very long session title");
  expect(html).not.toContain("sess_abc123456789");
});
```

Run:

```bash
bun test src/components/ui/cyber-select.test.tsx
```

Expected: FAIL because `CyberSelect` does not yet support a dedicated history variant.

- [x] **Step 2: Implement the shared history-select variant**

Update `src/components/ui/cyber-select.tsx` so it supports:

```tsx
interface CyberSelectProps {
  value: string;
  options: CyberSelectOption[];
  onChange: (value: string) => void;
  disabled?: boolean;
  placeholder?: string;
  variant?: "default" | "history";
}
```

Apply the variant so:

```tsx
const historyVariant = variant === "history";

<button
  className={cn(
    "...existing classes...",
    historyVariant && "min-w-[11rem] max-w-[15rem] justify-between rounded-full px-3 py-1 text-[11px]",
  )}
>
  <span className="min-w-0 truncate text-left">{displayLabel}</span>
</button>

{open && (
  <div
    className={cn(
      "...existing menu classes...",
      historyVariant && "top-8 w-[22rem] max-w-[min(22rem,calc(100vw-2rem))] rounded-2xl p-2",
    )}
  >
```

Render option rows so history items use a roomier two-line layout in the menu while the collapsed trigger stays compact.

Opt both history pickers into it:

```tsx
<CyberSelect
  variant="history"
  value={selectedHistoryId}
  options={historyOptions}
  onChange={setSelectedHistoryId}
  ...
/>
```

- [x] **Step 3: Re-run the focused tests**

Run:

```bash
bun test src/components/ui/cyber-select.test.tsx
```

Expected: PASS.

- [x] **Step 4: Run build and diff verification**

Run:

```bash
bun run build
git diff --check
```

Expected: PASS with no diff-format issues.

- [x] **Step 5: Commit**

```bash
git add src/components/ui/cyber-select.tsx src/components/ui/cyber-select.test.tsx src/components/ClaudePanel/index.tsx src/components/AgentStatus/CodexPanel.tsx
git commit -m "fix: polish shared provider history select"
```

- [x] **Step 6: Update `## CM Memory`**

Replace the Task 2 placeholder row with the real commit hash and verification evidence.

### Task 3: Collapse message search behind the header icon

**Files:**
- Create: `src/components/MessagePanel/presentational.test.tsx`
- Create: `src/components/MessagePanel/search-chrome.tsx`
- Modify: `src/components/MessagePanel/index.tsx`

- [x] **Step 1: Write the failing search-disclosure tests**

Create `src/components/MessagePanel/presentational.test.tsx` with:

```tsx
import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { MessageSearchChrome } from "./index";

describe("MessageSearchChrome", () => {
  test("closed state renders only the header search button", () => {
    const html = renderToStaticMarkup(
      createElement(MessageSearchChrome, {
        searchOpen: false,
        searchQuery: "",
        searchSummary: null,
        inputRef: { current: null },
        onOpen: () => {},
        onQueryChange: () => {},
        onClose: () => {},
      }),
    );
    expect(html).toContain("Search messages");
    expect(html).not.toContain('type="search"');
  });
});
```

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx
```

Expected: FAIL because the dedicated presentational chrome does not exist yet.

- [x] **Step 2: Extract and integrate the search disclosure chrome**

In `src/components/MessagePanel/index.tsx`, add a small exported presentational component:

```tsx
export function MessageSearchChrome({
  searchOpen,
  searchQuery,
  searchSummary,
  inputRef,
  onOpen,
  onQueryChange,
  onClose,
}: {
  searchOpen: boolean;
  searchQuery: string;
  searchSummary: string | null;
  inputRef: React.RefObject<HTMLInputElement | null>;
  onOpen: () => void;
  onQueryChange: (query: string) => void;
  onClose: () => void;
}) {
  return (
    <>
      <div className="flex items-center border-b border-border/35 px-4 py-1.5">
        <button ... onClick={onOpen} aria-label="Search messages">
          <Search className="size-4" />
        </button>
      </div>
      {searchOpen ? (
        <SearchRow ... />
      ) : null}
    </>
  );
}
```

Use it from `MessagePanel` so the icon is always in the header and the input row only appears while `searchOpen` is true.

- [x] **Step 3: Re-run the focused tests**

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx
```

Expected: PASS.

- [x] **Step 4: Run build and diff verification**

Run:

```bash
bun run build
git diff --check
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/components/MessagePanel/index.tsx src/components/MessagePanel/presentational.test.tsx
git commit -m "fix: collapse message search behind header icon"
```

- [x] **Step 6: Update `## CM Memory`**

Replace the Task 3 placeholder row with the real commit hash and verification evidence.

### Task 4: Make the back-to-bottom control transparent

**Files:**
- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/search-chrome.tsx`
- Modify: `src/components/MessagePanel/presentational.test.tsx`

- [x] **Step 1: Extend the failing presentation test**

Add:

```tsx
import { BackToBottomButton } from "./MessageList";

test("back-to-bottom button uses transparent chrome", () => {
  const html = renderToStaticMarkup(
    createElement(BackToBottomButton, {
      onClick: () => {},
    }),
  );
  expect(html).toContain("Back to bottom");
  expect(html).toContain("bg-transparent");
});
```

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx
```

Expected: FAIL because the exported transparent button does not exist yet.

- [x] **Step 2: Extract the transparent button and integrate it**

In `src/components/MessagePanel/MessageList.tsx`, add:

```tsx
export function BackToBottomButton({
  onClick,
}: {
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="z-10 rounded-full bg-transparent px-3 py-1.5 text-[11px] text-muted-foreground transition-colors hover:text-foreground"
    >
      ↓ Back to bottom
    </button>
  );
}
```

Replace the inline button with `<BackToBottomButton onClick={scrollToBottom} />`.

- [x] **Step 3: Re-run the focused tests**

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx
```

Expected: PASS.

- [x] **Step 4: Run build and diff verification**

Run:

```bash
bun run build
git diff --check
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/components/MessagePanel/MessageList.tsx src/components/MessagePanel/presentational.test.tsx
git commit -m "style: make back-to-bottom chrome transparent"
```

- [x] **Step 6: Update `## CM Memory`**

Replace the Task 4 placeholder row with the real commit hash and verification evidence.

### Task 5: Correct the provider-history dropdown clipping regression

**Files:**
- Modify: `src/components/ui/cyber-select.tsx`
- Modify: `src/components/ui/cyber-select.test.tsx`

- [x] **Step 1: Add a failing readability regression test**

Extend the history-variant tests so they fail unless dropdown items keep long history content readable instead of truncating both lines.

Run:

```bash
bun test src/components/ui/cyber-select.test.tsx
```

Expected: FAIL before the regression fix.

- [x] **Step 2: Fix the history dropdown layout**

Adjust the shared `history` variant so the real provider-history menu is not visually clipped:

- remove unnecessary one-line truncation from history menu rows
- allow long session labels/ids to wrap or clamp more gracefully
- widen or rebalance the history menu so it matches the screenshot context instead of forcing a narrow compact menu
- keep the collapsed trigger readable without making the menu ugly

- [x] **Step 3: Re-run verification**

Run:

```bash
bun test src/components/ui/cyber-select.test.tsx
bun run build
git diff --check
```

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add src/components/ui/cyber-select.tsx src/components/ui/cyber-select.test.tsx
git commit -m "fix: unclip provider history dropdown content"
```

- [x] **Step 5: Update `## CM Memory`**

Replace the Task 5 placeholder row with the real commit hash and verification evidence.

### Task 6: Correct the missing header search entrypoint

**Files:**
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/MessagePanel/search-chrome.tsx`
- Modify: `src/components/MessagePanel/presentational.test.tsx`
- Modify: `src/components/MessagePanel/index.test.tsx`

- [x] **Step 1: Add a failing integration test for the visible search entrypoint**

Add coverage that fails unless the intended chat-header search entrypoint is visible in the integrated message panel state that should expose it.

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx src/components/MessagePanel/index.test.tsx
```

Expected: FAIL before the fix.

- [x] **Step 2: Fix the integrated search entrypoint**

Removed the `chatMessages.length > 0 &&` guard around `MessageSearchChrome` in `index.tsx`. Root cause: Zustand v5 SSR always calls `api.getInitialState()` for the server snapshot — this returns the original empty state and cannot be patched from outside the store closure. Removing the guard is the correct minimal fix: the search icon is now always present in chat mode.

- [x] **Step 3: Re-run verification**

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx src/components/MessagePanel/index.test.tsx
bun run build
git diff --check
```

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add src/components/MessagePanel/index.tsx src/components/MessagePanel/index.test.tsx
git commit -m "fix: restore visible header search entrypoint"
```

- [x] **Step 5: Update `## CM Memory`**

Replace the Task 6 placeholder row with the real commit hash and verification evidence.

### Task 7: Restore the back-to-bottom chrome while keeping the background transparent

**Files:**
- Modify: `src/components/MessagePanel/search-chrome.tsx`
- Modify: `src/components/MessagePanel/presentational.test.tsx`

- [x] **Step 1: Write the failing presentation test**

Extend the button presentation test so it fails unless the back-to-bottom control keeps the pre-polish chrome while only changing the background fill:

```tsx
test("back-to-bottom button keeps the original chrome with a transparent background", () => {
  const html = renderToStaticMarkup(
    createElement(BackToBottomButton, { onClick: () => {} }),
  );
  expect(html).toContain("Back to bottom");
  expect(html).toContain("rounded-full");
  expect(html).toContain("text-primary-foreground");
  expect(html).toContain("shadow-lg");
  expect(html).toContain("bg-transparent");
  expect(html).not.toContain("bg-primary/90");
});
```

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx
```

Expected: FAIL because the current transparent button dropped the original chrome classes instead of only changing the background treatment.

- [x] **Step 2: Restore the original chrome classes and only swap the fill**

Update `src/components/MessagePanel/search-chrome.tsx` so `BackToBottomButton` keeps the original sizing, rounding, foreground color, shadow, and transition classes from the pre-polish button, but uses a transparent background in both resting and hover states.

- [x] **Step 3: Re-run focused verification**

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx -t "BackToBottomButton"
bun run build
git diff --check
```

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add src/components/MessagePanel/search-chrome.tsx src/components/MessagePanel/presentational.test.tsx docs/superpowers/plans/2026-04-07-session-history-ui-polish.md
git commit -m "style: restore back-to-bottom chrome"
```

- [x] **Step 5: Update `## CM Memory`**

Append the real Task 7 commit hash, review result, and verification evidence to `## CM Memory`.
