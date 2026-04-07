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
| Task 4 | `style: make back-to-bottom chrome transparent` | `manual review` | `bun test src/components/MessagePanel/presentational.test.tsx`; `bun run build`; `git diff --check` | Secondary chat affordances should read like lightweight navigation, not filled primary-action pills. |

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
- Modify: `src/components/MessagePanel/presentational.test.tsx`

- [ ] **Step 1: Extend the failing presentation test**

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

- [ ] **Step 2: Extract the transparent button and integrate it**

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

- [ ] **Step 3: Re-run the focused tests**

Run:

```bash
bun test src/components/MessagePanel/presentational.test.tsx
```

Expected: PASS.

- [ ] **Step 4: Run build and diff verification**

Run:

```bash
bun run build
git diff --check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/components/MessagePanel/MessageList.tsx src/components/MessagePanel/presentational.test.tsx
git commit -m "style: make back-to-bottom chrome transparent"
```

- [ ] **Step 6: Update `## CM Memory`**

Replace the Task 4 placeholder row with the real commit hash and verification evidence.
