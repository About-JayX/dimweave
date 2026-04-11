# Feishu First-Page Hydration Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the entire issues area in a skeleton state until the first page is fully hydrated, then reveal filters and cards together.

**Architecture:** Add an explicit frontend hydration gate for issues mode instead of inferring readiness from partially available state. The store will own a small `issuesHydrating` flag and a single reload path for the first page. The panel will render an issues-area skeleton while that gate is active, then switch to the existing filter/list UI after runtime state, filter options, and first-page items are all ready.

**Tech Stack:** React 19, Zustand, TypeScript, Bun test runner, Vite.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-first-page-hydration-fix` on branch `fix/feishu-first-page-hydration-fix`
- Baseline verification before implementation:
  - `bun test src/stores/feishu-project-store.test.ts`
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass

## Project Memory

### Recent related commits

- `626fd6f1` — fixed status enum parsing from real Feishu `GROUP BY` payloads.
- `3ed95c8f` — store now re-reads runtime state after `fetchFilterOptions()`.
- `cbc88fb1` — backend now preserves filter options across runtime refreshes.
- `10f98ed8` — initial frontend fetch ordering now waits for `fetchState()` before `fetchFilterOptions()`.
- `989bf41b` — issues UI now renders status + assignee filters.
- `7a3231b3` / `fffde8a2` — load-more continuity and observer trigger are already fixed and must not regress.

### Verified runtime evidence

- Current UI still reveals the issues area incrementally:
  - filters and list can appear in separate phases
  - this produces the “半成品” experience the user explicitly rejected
- The user’s required behavior is narrower than “load all data forever”:
  - only the **first page** must fully hydrate before the issues area appears
  - later pages should keep the existing incremental load-more behavior
- The same gating must apply when filters change:
  - reset to full issues-area skeleton
  - then reveal the new filtered first page once hydration completes

### Lessons that constrain this plan

- Do not remove pagination or the load-more observer.
- Do not redesign backend query flow; this is a frontend hydration-gating fix.
- Do not infer readiness from `statusOptions.length` / `items.length` alone; use an explicit gate to avoid repeating earlier race bugs.
- Keep the skeleton scoped to the issues area, not the whole Feishu panel.

## File Map

- Modify: `src/stores/feishu-project-store.ts`
- Modify: `src/stores/feishu-project-store.test.ts`
- Modify: `src/components/BugInboxPanel/index.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: gate feishu issues view on first-page hydration` | `bun test src/stores/feishu-project-store.test.ts`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | The user requires the entire issues area to remain skeleton-only until first-page runtime state, filter options, and items are all ready. Load-more after first reveal must remain incremental. **Accepted: `aac88ef4`**, follow-up **`1572ae51`** to keep `SyncModeNav` hidden until the same hydration gate lifts. |

---

### Task 1: Gate the issues area on first-page hydration

**task_id:** `feishu-first-page-hydration-code`

**Acceptance criteria:**

- In issues mode, the panel shows only an issues-area skeleton until the first-page hydration flow completes.
- The first reveal includes both filters and cards together; no partial issues UI is shown earlier.
- Changing issue filters re-enters the same skeleton gate and then reveals the new filtered first page.
- Existing load-more behavior remains unchanged after the first reveal.
- No backend files are modified.

**allowed_files:**

- `src/stores/feishu-project-store.ts`
- `src/stores/feishu-project-store.test.ts`
- `src/components/BugInboxPanel/index.tsx`
- `src/components/BugInboxPanel/index.test.tsx`

**max_files_changed:** `4`

**max_added_loc:** `140`

**max_deleted_loc:** `60`

**verification_commands:**

- `bun test src/stores/feishu-project-store.test.ts`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing tests first**

Add tests proving:

- the store has an explicit issues first-page hydration action/flag
- that action fetches state, filter options, and first-page items before clearing the hydration gate
- the panel renders an issues-area skeleton while the gate is active
- the real issues controls/list do not render until the gate is lifted

- [ ] **Step 2: Run the frontend verification set and confirm the new assertions fail before implementation**

- [ ] **Step 3: Implement the minimal store/panel fix**

Make only these changes:

- add an explicit issues-area hydration flag + first-page reload path in the store
- use that path on initial issues load and on filter changes
- render a dedicated issues-area skeleton in `BugInboxPanel` while the flag is active

Do not:

- modify backend code
- remove or rewrite the existing load-more observer
- expand this into “load the entire issue space before rendering”

- [ ] **Step 4: Re-run the verification set**

- [ ] **Step 5: Commit**

```bash
git add \
  src/stores/feishu-project-store.ts \
  src/stores/feishu-project-store.test.ts \
  src/components/BugInboxPanel/index.tsx \
  src/components/BugInboxPanel/index.test.tsx
git commit -m "fix: gate feishu issues view on first-page hydration"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
