# Feishu Status Options Hydration Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ensure the status filter dropdown reliably appears on first load by fixing the runtime-state hydration race between `fetchState()` and `fetchFilterOptions()`.

**Architecture:** Keep the new filter-query backend and UI intact, but make the filter-options path robust when `feishu_project_runtime` has not been initialized yet. The minimal fix is to ensure filter-options hydration can materialize/update runtime state before the frontend consumes it, and to sequence the initial frontend fetches so the state flow is deterministic.

**Tech Stack:** Rust, Tauri daemon, React 19, TypeScript, Bun test runner, Cargo.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-status-options-fix` on branch `fix/feishu-status-options-fix`
- Required setup before baseline verification: `cargo build --manifest-path bridge/Cargo.toml`
- Baseline verification before any implementation changes:
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass

## Project Memory

### Recent related commits

- `2d4c745d` — docs recorded filter-query evidence.
- `989bf41b` — wired status/assignee filters through the Bug Inbox UI.
- `8e1650fa` — added backend filtered query flow and filter option endpoints.
- `e3e57223` — fixed the load-more observer trigger after sentinel mount.

### Verified root-cause evidence

- `BugInboxPanel/index.tsx` currently fires `fetchState()`, `fetchItems()`, and `fetchFilterOptions()` concurrently in one `useEffect`.
- `daemon/feishu_project_lifecycle.rs::fetch_filter_options()` only updates and emits options when `feishu_project_runtime` already exists.
- If `fetch_filter_options()` wins the race before runtime state exists, status/assignee options are dropped, and `SyncModeNav` never renders the status dropdown because `statusOptions.length === 0`.

### Lessons that constrain this plan

- Do not change the business flow for status/assignee filtering; this fix is only about first-load hydration.
- Preserve the existing filtered load-more and operator enrichment behavior.
- Keep the fix small and targeted; do not refactor unrelated frontend/backend code.

## File Map

- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- Modify: `src/components/BugInboxPanel/index.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: hydrate feishu filter options before first render` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | `10f98ed8` — final accepted fix. Materializes `feishu_project_runtime` when filter options arrive before runtime initialization and sequences `fetchState()` before `fetchFilterOptions()` on first load, eliminating silent drops of `statusOptions`/`assigneeOptions`. Lead re-ran the verification set successfully on 2026-04-11. |

---

### Task 1: Fix first-load filter option hydration

**task_id:** `feishu-status-options-hydration`

**Acceptance criteria:**

- First load in issues mode produces non-empty `statusOptions`/`assigneeOptions` in runtime state when the backend returns them.
- Status dropdown appears without requiring a second manual sync or mode toggle.
- Existing filtered load-more behavior remains intact.
- No new dependencies are added.

**allowed_files:**

- `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- `src/components/BugInboxPanel/index.tsx`
- `src/components/BugInboxPanel/index.test.tsx`

**max_files_changed:** `4`

**max_added_loc:** `120`

**max_deleted_loc:** `60`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing tests for the missing-runtime filter-options case**

Add tests that prove:

- backend filter-option hydration initializes or updates runtime state even when it starts as `None`
- frontend initial load path sequences state/options fetches sufficiently for the dropdown to appear

- [ ] **Step 2: Run the verification set and confirm the new assertions fail before implementation**

- [ ] **Step 3: Implement the minimal hydration fix**

Make only these code changes:

- backend: ensure `fetch_filter_options()` can materialize/update runtime state when `feishu_project_runtime` is `None`
- frontend: make the initial data-fetch sequence deterministic so filter options are requested after state is available

Do not:

- alter backend filter semantics
- change `SyncModeNav` props/markup beyond what the deterministic fetch flow strictly requires
- add dependencies

- [ ] **Step 4: Re-run the verification set**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/daemon/feishu_project_lifecycle.rs \
  src-tauri/src/daemon/feishu_project_lifecycle_tests.rs \
  src/components/BugInboxPanel/index.tsx \
  src/components/BugInboxPanel/index.test.tsx
git commit -m "fix: hydrate feishu filter options before first render"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**

Record the actual commit hash and verification result in the table above after lead review passes.
