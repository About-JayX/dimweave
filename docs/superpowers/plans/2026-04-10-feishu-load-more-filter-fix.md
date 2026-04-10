# Feishu Load-More Filter Continuity Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve correct assignee display and assignee filtering after issue pagination so loading more issues does not drop assignee labels or keep the filter list stale.

**Architecture:** Reuse the new issue-detail enrichment path for `load_more`, then refresh runtime state from the enriched store so `team_members` always reflects every currently loaded issue item. Keep the flow aligned with the existing product behavior: current page loads first, additional pages append on demand, and filtering applies to the loaded dataset.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP HTTP client, serde_json, Cargo test/check.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-filter-business-fix` on branch `fix/feishu-filter-business-fix`
- Required setup before baseline verification: `cargo build --manifest-path bridge/Cargo.toml`
- Baseline verification before any implementation changes:
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Baseline result: pass (`19 passed`, `85 passed`, `cargo check` exit `0`)

## Project Memory

### Recent related commits

- `f0d0939d` — fixed detail member parsing to use `members[].name`.
- `b60350e9` — fixed detail call argument shape and added bounded concurrency for initial issue enrichment.
- `f2b27650` — introduced issue-detail enrichment and team-member derivation from enriched items.
- `cd931f03` — restored `current_status_operator` as the sole valid MQL `field_key`.

### Verified runtime evidence constraining this plan

- Current main now returns correct assignee names on initial issue sync when detail enrichment runs.
- `daemon/feishu_project_lifecycle.rs::load_more()` currently appends raw MQL items only; it does **not** call `enrich_issues_with_operators()` and does **not** refresh runtime state after append.
- Therefore cards appended via load-more have no assignee label, and the filter pool stays stale after pagination.

### Lessons that constrain this plan

- Do not reintroduce MQL-based assignee filtering; operator names still must come from detail `role_members.operator`.
- Keep the business flow tied to the currently loaded dataset; do not expand this task into a full-space background indexer.
- Stay within the existing contracts for `FeishuProjectInboxItem.assignee_label` and `FeishuProjectRuntimeState.team_members`.

## File Map

- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: enrich feishu load-more items before filtering` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `cargo check --manifest-path src-tauri/Cargo.toml` | `7a3231b3` — final accepted continuity fix. Extends the operator-detail enrichment path to `load_more()`, refreshes runtime `team_members` after append, and preserves the initial-page semantics from `b60350e9` / `f0d0939d`. Lead re-ran the full verification set successfully on 2026-04-10. |

---

### Task 1: Keep assignee cards and filters correct across load-more

**task_id:** `feishu-load-more-filter-code`

**Acceptance criteria:**

- `load_more()` enriches newly fetched issue items with detail `role_members.operator` before upserting them.
- `load_more()` refreshes runtime state after append so `team_members` reflects the full currently loaded dataset.
- Existing initial sync behavior remains unchanged.
- No frontend contract changes are introduced.

**allowed_files:**

- `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`

**max_files_changed:** `2`

**max_added_loc:** `100`

**max_deleted_loc:** `40`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: Add failing tests for paginated enrichment/runtime-state refresh**

Add tests proving:

- `load_more()` (or a factored helper behind it) enriches appended issue items before persisting them
- runtime `team_members` is recomputed after load-more append

- [ ] **Step 2: Run the focused verification and confirm the new assertions fail before implementation**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture
```

Expected: fail because current `load_more()` appends raw items only and never refreshes runtime state.

- [ ] **Step 3: Implement the minimal continuity fix**

Make only these code changes:

- after `sync_issues_page()` in `load_more()`, call `enrich_issues_with_operators()` on the fetched page
- after upsert + persist, call `update_mcp_state()` (or equivalent minimal refresh path) so `team_members` re-derives from the full store

Do not:

- change frontend files
- change initial sync code paths beyond what the load-more path strictly needs
- add new dependencies
- expand into background prefetch of all issue operators

- [ ] **Step 4: Re-run the full verification set**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: all commands pass; warnings are acceptable only if unchanged from baseline.

- [ ] **Step 5: Commit the task**

Run:

```bash
git add \
  src-tauri/src/daemon/feishu_project_lifecycle.rs \
  src-tauri/src/daemon/feishu_project_lifecycle_tests.rs
git commit -m "fix: enrich feishu load-more items before filtering"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**

Record the actual commit hash and verification result in the table above after lead review passes.
