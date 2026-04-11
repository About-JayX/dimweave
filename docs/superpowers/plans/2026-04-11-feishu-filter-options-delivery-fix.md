# Feishu Filter Options Delivery Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the status filter reliably render by ensuring the frontend store updates `runtimeState` after requesting filter options instead of relying only on the async event path.

**Architecture:** Keep the backend as-is. Fix the last delivery gap in the frontend store: after `feishu_project_fetch_filter_options` completes, immediately re-read `feishu_project_get_state` and store the returned runtime state. This closes the remaining race where the event can be missed even though the backend has already populated `statusOptions`.

**Tech Stack:** TypeScript, Zustand, Bun test runner, Vite build.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-status-delivery-fix` on branch `fix/feishu-status-delivery-fix`
- Baseline verification before any implementation changes:
  - `bun test src/stores/feishu-project-store.test.ts`
  - `bun run build`
- Baseline result: pass

## Project Memory

### Recent related commits

- `cbc88fb1` — preserves filter options across backend runtime-state refreshes.
- `10f98ed8` — ensures filter-option hydration can materialize runtime state and sequences initial fetches.
- `989bf41b` — wires status/assignee filter UI against `runtimeState.statusOptions`.

### Verified root-cause evidence

- Backend enum query works and `fetch_filter_options()` now successfully writes `statusOptions`.
- `feishu-project-store.ts::fetchFilterOptions()` only awaits the command and does not refresh `runtimeState`.
- Because the store depends on the `feishu_project_state` event to observe the change, a missed/late event leaves `runtimeState.statusOptions` stale and the UI continues to hide the status dropdown.

## File Map

- Modify: `src/stores/feishu-project-store.ts`
- Modify: `src/stores/feishu-project-store.test.ts`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: rehydrate feishu runtime after filter option fetch` | `bun test src/stores/feishu-project-store.test.ts`; `bun run build` | Must preserve the backend fixes from `10f98ed8` / `cbc88fb1` and close the final frontend delivery gap by directly re-reading runtime state after the filter-options command returns. |

---

### Task 1: Rehydrate runtime state after filter-option fetch

**task_id:** `feishu-filter-options-delivery`

**Acceptance criteria:**

- After `fetchFilterOptions()` resolves, the store updates `runtimeState` using `feishu_project_get_state`.
- The store no longer depends solely on the async event for filter option visibility.
- Existing store actions and UI wiring remain unchanged.

**allowed_files:**

- `src/stores/feishu-project-store.ts`
- `src/stores/feishu-project-store.test.ts`

**max_files_changed:** `2`

**max_added_loc:** `60`

**max_deleted_loc:** `20`

**verification_commands:**

- `bun test src/stores/feishu-project-store.test.ts`
- `bun run build`

- [ ] **Step 1: Add a failing store test for filter-option rehydration**

Add a test that proves:

- `fetchFilterOptions()` triggers the filter-options command
- then re-reads `feishu_project_get_state`
- and stores the returned `statusOptions` / `assigneeOptions`

- [ ] **Step 2: Run the store verification and confirm the new assertion fails before implementation**

- [ ] **Step 3: Implement the minimal store fix**

Make only these code changes:

- in `fetchFilterOptions()`, after the invoke returns, call `feishu_project_get_state` and update `runtimeState`

Do not:

- change backend files
- change component files
- add dependencies

- [ ] **Step 4: Re-run the verification set**

- [ ] **Step 5: Commit**

```bash
git add \
  src/stores/feishu-project-store.ts \
  src/stores/feishu-project-store.test.ts
git commit -m "fix: rehydrate feishu runtime after filter option fetch"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
