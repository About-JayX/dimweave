# Feishu Status Options Persistence Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the Feishu status filter visible after sync/runtime refreshes by preserving `statusOptions` and `assigneeOptions` in runtime state instead of wiping them on every `update_mcp_state()` call.

**Architecture:** Treat filter options as sticky runtime metadata once fetched. `fetch_filter_options()` still initializes them, but later `update_mcp_state()` / `update_mcp_state_error()` must carry forward any existing options when rebuilding `FeishuProjectRuntimeState` from config. This preserves the first-load hydration fix while avoiding option loss after syncs.

**Tech Stack:** Rust, Tauri daemon, React 19, Bun test runner, Cargo.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-status-options-persist-rootfix` on branch `fix/feishu-status-options-persist-rootfix`
- Required setup before baseline verification: `cargo build --manifest-path bridge/Cargo.toml`
- Baseline verification before any implementation changes:
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass

## Project Memory

### Recent related commits

- `10f98ed8` — fixed first-load status-option hydration race by materializing runtime state and sequencing initial fetches.
- `55ee5d8c` — recorded the hydration-race fix plan.
- `2d4c745d` — documented filter-query evidence and status label filtering.
- `989bf41b` — wired status/assignee filters through the Bug Inbox UI.

### Verified root-cause evidence

- `src-tauri/src/feishu_project/runtime.rs::update_mcp_state()` rebuilds `FeishuProjectRuntimeState` via `from_config(cfg)`, which initializes `status_options` / `assignee_options` to empty vectors.
- Any successful sync or runtime refresh after `fetch_filter_options()` therefore clears the options again before the UI renders them.
- `SyncModeNav` only renders the status dropdown when `statusOptions.length > 0`, so clearing them makes the status filter disappear.

### Lessons that constrain this plan

- Do not alter filter semantics or API shapes; this is a persistence fix only.
- Preserve the existing first-load sequencing fix from `10f98ed8`.
- Keep the patch minimal and targeted to runtime-state rebuilding.

## File Map

- Modify: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: preserve feishu filter options across runtime updates` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | Must preserve `statusOptions`/`assigneeOptions` across `update_mcp_state()` and error refreshes while keeping the prior hydration-race fix from `10f98ed8`. |

---

### Task 1: Preserve filter options across runtime-state rebuilds

**task_id:** `feishu-status-options-persist`

**Acceptance criteria:**

- After `fetch_filter_options()` populates runtime options, later `update_mcp_state()` keeps those options instead of clearing them.
- Error-path runtime refresh also preserves existing options when present.
- Existing tests and UI build still pass.

**allowed_files:**

- `src-tauri/src/feishu_project/runtime.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`

**max_files_changed:** `2`

**max_added_loc:** `80`

**max_deleted_loc:** `30`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add a failing regression test for option persistence across runtime refresh**

Add tests proving:

- runtime options survive a successful `update_mcp_state()` rebuild
- runtime options survive the error-path rebuild too

- [ ] **Step 2: Run the verification set and confirm the new assertions fail before implementation**

- [ ] **Step 3: Implement the minimal persistence fix**

Make only these code changes:

- in `runtime.rs`, carry forward existing `status_options` / `assignee_options` from `state.feishu_project_runtime` before replacing it
- keep all other runtime-state fields unchanged

Do not:

- change frontend files
- change filter-query APIs
- add dependencies

- [ ] **Step 4: Re-run the verification set**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/feishu_project/runtime.rs \
  src-tauri/src/daemon/feishu_project_lifecycle_tests.rs
git commit -m "fix: preserve feishu filter options across runtime updates"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
