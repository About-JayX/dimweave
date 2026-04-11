# Feishu Filter View/Cache Split Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make status/current-owner filter changes actually replace the visible issue list instead of merging into the sync cache, while preserving append semantics for load-more under the same filter tuple.

**Architecture:** Split the Feishu issue data model into two layers inside the daemon: a raw sync cache store and a visible issue-view store. Background sync updates the raw cache. Filtered queries update the visible view. When the filter tuple changes, the visible view is replaced. When the same filter tuple paginates, the visible view appends. UI commands continue to call `list_items`, but that call now returns the visible view store only.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP HTTP client, Bun, Cargo.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-filter-view-split-fix` on branch `fix/feishu-filter-view-split-fix`
- Baseline verification before implementation:
  - `cargo build --manifest-path bridge/Cargo.toml`
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass

## Project Memory

### Recent related commits

- `5e6329ff` — current-owner filter CM update
- `b9eba99e` / `7cb061bd` — current owner + status MQL filtering
- `c0a3b5d0` / `1572ae51` / `aac88ef4` — first-page hydration gate
- `3ed95c8f` / `cbc88fb1` / `10f98ed8` — filter option delivery and persistence fixes

### Verified root-cause evidence

- `feishu_project_store` is currently used as both:
  1. the backend sync cache
  2. the visible filtered result set
- `load_more_filtered()` currently does `upsert()` into that single store, so filter changes merge new results into old ones instead of replacing the visible list.
- `list_items()` returns that entire single store, so the frontend never gets a clean filtered dataset.
- `run_mcp_sync_cycle()` also writes into the same store via `sync_replace()`, so periodic/manual sync can clobber the visible filtered list.

### Lessons that constrain this plan

- Do not “patch” this by adding more front-end clearing only; the bug is in daemon-side shared state.
- Keep list queries as `page + limit + filter tuple`.
- Preserve same-filter load-more append semantics.
- Preserve local workflow state (`ignored`, `linked_task_id`) across both raw cache and visible filtered view.

## File Map

- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- Modify: `src-tauri/src/daemon/feishu_project_task_link.rs`
- Modify: `src-tauri/src/daemon/feishu_project_task_link_tests.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: separate feishu filtered view from sync cache` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | The root cause is a shared daemon store used for both sync cache and filtered view. Filter changes must replace the visible dataset; same-filter load-more may append. Background sync must update cache without corrupting the active filtered view. |

---

### Task 1: Split daemon sync cache from visible filtered view

**task_id:** `feishu-filter-view-split-code`

**Acceptance criteria:**

- The daemon keeps a raw sync cache separate from the currently visible issue list.
- Filter tuple changes replace the visible issue list instead of merging via `upsert()`.
- Same filter tuple load-more appends to the visible list.
- Background/manual sync updates the raw cache without overwriting the active filtered visible list.
- `ignored` and `linked_task_id` updates are preserved in both raw cache and visible view when applicable.
- Frontend contracts remain unchanged.

**allowed_files:**

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/feishu_project/runtime.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`
- `src-tauri/src/daemon/feishu_project_task_link_tests.rs`

**max_files_changed:** `6`

**max_added_loc:** `220`

**max_deleted_loc:** `120`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing tests first**

Add tests proving:

- changing the filter tuple replaces the visible issue list instead of merging
- repeating load-more with the same filter tuple appends
- background sync can update the raw cache without replacing the active filtered visible list
- task-link / ignore updates propagate to both raw cache and visible view when the item exists in both

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal daemon store split**

Make only these changes:

- add raw-cache-vs-visible-view state separation in `DaemonState`
- persist the raw cache store, emit the visible issue view
- update filtered load-more to replace on filter change and append on same-filter continuation
- keep manual/background sync writing the raw cache, only mirroring to the visible view when no active filter is applied
- update ignore/link flows so visible items and raw cache stay consistent

Do not:

- change the front-end contract
- change MQL filter semantics again
- reintroduce detail-scan filtering

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/daemon/state.rs \
  src-tauri/src/feishu_project/runtime.rs \
  src-tauri/src/daemon/feishu_project_lifecycle.rs \
  src-tauri/src/daemon/feishu_project_lifecycle_tests.rs \
  src-tauri/src/daemon/feishu_project_task_link.rs \
  src-tauri/src/daemon/feishu_project_task_link_tests.rs
git commit -m "fix: separate feishu filtered view from sync cache"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
