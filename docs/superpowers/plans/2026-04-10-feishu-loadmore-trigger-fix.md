# Feishu Load-More Trigger Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the missing bottom-scroll load-more trigger in the Feishu issue list so pagination actually fires when the sentinel mounts after items load.

**Architecture:** Keep the existing infinite-scroll behavior, but fix the observer attachment timing in `IssueList`. The current effect runs before the sentinel exists and never reruns because its dependencies omit sentinel/item mount state. Use a ref/state pattern (or equivalent minimal rerender trigger) so the observer attaches when the sentinel node appears, while keeping the backend `load_more` behavior intact.

**Tech Stack:** React 19, TypeScript, Bun test runner, Vite build.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-loadmore-trigger-fix` on branch `fix/feishu-loadmore-trigger-fix`
- Required setup before baseline verification: `cargo build --manifest-path bridge/Cargo.toml`
- Baseline verification before any implementation changes:
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass (`6 passed`, build success)

## Project Memory

### Recent related commits

- `7a3231b3` — backend load-more now enriches appended items and refreshes runtime filter state.
- `b60350e9` — detail enrichment moved to bounded concurrency for initial issue sync.
- `9ef63dfb` — finalized the Feishu BugInbox frontend MCP workflow.
- `de6fcb99` — introduced the original Bug Inbox panel and list rendering surface.

### Related plans / addendum

- `docs/superpowers/plans/2026-04-09-feishu-mcp-inbox.md`
- `docs/superpowers/plans/2026-04-10-feishu-load-more-filter-fix.md`
- `docs/superpowers/plans/2026-04-10-feishu-operator-filter-fix.md`

### Verified root-cause evidence

- `IssueList.tsx` currently creates the `IntersectionObserver` in an effect with deps `[hasMore, onLoadMore, loadingMore]`.
- On initial render, `items.length === 0`, so the sentinel is not in the DOM and `sentinelRef.current` is `null`.
- When items later load, the sentinel mounts, but the effect does not rerun because its dependency list still has the same values. Therefore the observer never attaches, and `feishu_project_load_more` is never invoked.

### Lessons that constrain this plan

- Do not change the backend pagination path for this fix; backend continuity was already repaired in `7a3231b3`.
- Keep the behavior as infinite scroll, not a new button-based interaction.
- Stay focused on the missing observer attachment; avoid unrelated UI cleanup.

## File Map

- Modify: `src/components/BugInboxPanel/IssueList.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: attach feishu load-more observer after list mount` | `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | Must preserve the existing infinite-scroll UX from `2026-04-09-feishu-mcp-inbox.md` while fixing the effect timing bug that prevents observer attachment after items first appear. |

---

### Task 1: Attach the load-more observer after the sentinel actually mounts

**task_id:** `feishu-loadmore-trigger-ui`

**Acceptance criteria:**

- When the list first renders with no items and later rerenders with items plus `hasMore=true`, the observer attaches to the sentinel.
- Existing spinner/sentinel behavior stays intact.
- No backend contract changes are introduced.
- Focused frontend test and production build pass.

**allowed_files:**

- `src/components/BugInboxPanel/IssueList.tsx`
- `src/components/BugInboxPanel/index.test.tsx`

**max_files_changed:** `2`

**max_added_loc:** `90`

**max_deleted_loc:** `40`

**verification_commands:**

- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add a failing regression test for the delayed sentinel mount case**

Add a DOM-capable test that proves:

- first render with `items=[]`, `hasMore=true` does not attach because there is no sentinel node yet
- rerender with at least one item and `hasMore=true` attaches the observer and observes the sentinel

- [ ] **Step 2: Run the focused frontend test and confirm the new assertion fails before implementation**

Run:

```bash
bun test src/components/BugInboxPanel/index.test.tsx
```

Expected: fail because `IssueList` never reattaches the observer after the sentinel first mounts.

- [ ] **Step 3: Implement the minimal observer-attachment fix**

Make only these code changes:

- update `IssueList` so observer setup reruns when the sentinel node becomes available after list mount
- keep the existing `hasMore / loadingMore / onLoadMore` semantics

Do not:

- change backend files
- add dependencies
- replace infinite scroll with a button

- [ ] **Step 4: Re-run the verification set**

Run:

```bash
bun test src/components/BugInboxPanel/index.test.tsx
bun run build
```

Expected: tests pass and build succeeds.

- [ ] **Step 5: Commit the task**

Run:

```bash
git add \
  src/components/BugInboxPanel/IssueList.tsx \
  src/components/BugInboxPanel/index.test.tsx
git commit -m "fix: attach feishu load-more observer after list mount"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**

Record the actual commit hash and verification result in the table above after lead review passes.
