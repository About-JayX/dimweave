# Feishu Issues Assignee Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Feishu defect-management list and assignee filter use the actual assignee/operator field instead of surfacing reporter data.

**Architecture:** Keep the Bug Inbox UI contract unchanged and repair the backend MCP query/parsing path. Centralize assignee-field selection in the Feishu MCP sync layer so both the issue list and the team-member filter prefer `operator`, while falling back to `current_status_operator` only when the MCP query shape rejects the newer field.

**Tech Stack:** Rust, Tauri daemon, serde_json, Cargo test/check.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-issues-assignee-fix` on branch `fix/feishu-issues-assignee-fix`
- Required setup before baseline verification: `cargo build --manifest-path bridge/Cargo.toml`
- Baseline verification before any implementation changes:
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Baseline result: pass (`10 passed`, `76 passed`, `cargo check` exit `0`)

## Project Memory

### Recent related commits

- `ea461155` — finalized the Feishu MCP inbox backend and introduced the current `current_status_operator` issue query.
- `3277d2c8` — added the current team-member refresh path and `GROUP BY current_status_operator` runtime cache.
- `33125abc` — refreshed Feishu runtime state after manual sync; do not regress runtime-state emission.
- `9ef63dfb` — finalized the Bug Inbox frontend MCP workflow; preserve the existing frontend store/UI contract.

### Related plans / addendum

- `docs/superpowers/plans/2026-04-09-feishu-mcp-inbox.md`
- `docs/superpowers/plans/2026-04-09-feishu-project-mcp-pivot.md`
- `docs/superpowers/plans/2026-04-10-tools-drawer.md`

### Plan revision history

- `2026-04-10` — user approved a focused revision after lead review found that candidate commit `cfe2d862` exceeded the original add-line budget and missed the approved operator -> `current_status_operator` query-failure fallback.
- Revision scope remains unchanged: same 3 Rust files, no frontend/config/interface changes.
- `2026-04-10` — `list_workitem_field_config` MCP 实测证实：在 workspace `manciyuan` 的 `issue` 类型下，`operator` **不是** 合法 field_key；唯一的经办人字段是 `current_status_operator`（当前负责人, multi-user）。之前 `7888ae33` 引入的 `operator` 优先路径实际上始终走 fallback，属于 dead code。后续修复将删除 `operator` 主路径，恢复 `current_status_operator` 为唯一查询字段。

### Lessons that constrain this plan

- Keep the repair inside the existing Feishu MCP Rust path; no frontend contract change is required.
- Preserve manual-sync/runtime-state behavior from `33125abc`; only the assignee source should change.
- Stay within the minimal repair scope: no new files, no config/interface changes, no UI restructuring.
- Maintain compatibility for spaces that still require `current_status_operator` by falling back only when the preferred query fails.

## File Map

- Modify: `src-tauri/src/feishu_project/mcp_sync.rs`
- Modify: `src-tauri/src/feishu_project/mcp_sync_tests.rs`
- Modify: `src-tauri/src/feishu_project/runtime.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: use assignee for feishu issue inbox` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `cargo check --manifest-path src-tauri/Cargo.toml` | `cd931f03` — final accepted correction after MCP field-config probe proved `current_status_operator -> 当前负责人 -> multi-user` and disproved `operator` as an `issue` field_key in workspace `manciyuan`. This commit removes the dead `operator` query path introduced by `7888ae33`, keeps the runtime-state behavior from `33125abc`, and was re-verified by lead on 2026-04-10 with the full required command set. |

---

### Task 1: Prefer assignee/operator for issue rows and filter options

**task_id:** `feishu-issues-assignee-fix`

**Acceptance criteria:**

- Feishu issue-list sync prefers the assignee/operator field when building inbox rows.
- Parsed inbox rows still fall back to `current_status_operator` only when the preferred field is unavailable in the MCP result shape.
- Runtime team-member discovery for the assignee dropdown prefers operator-based grouping and only falls back on query failure.
- Existing frontend `assigneeLabel` / `teamMembers` consumers keep working unchanged.
- Focused Rust tests and `cargo check` pass.
- If the preferred `operator` MQL query fails, issue sync and team-member discovery retry once with `current_status_operator`.

**allowed_files:**

- `src-tauri/src/feishu_project/mcp_sync.rs`
- `src-tauri/src/feishu_project/mcp_sync_tests.rs`
- `src-tauri/src/feishu_project/runtime.rs`

**max_files_changed:** `3`

**max_added_loc:** `140`

**max_deleted_loc:** `30`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: Add failing regression tests for operator-first parsing and query generation**

Update `src-tauri/src/feishu_project/mcp_sync_tests.rs` to add assertions that:

- a parsed MQL row prefers `operator` over `current_status_operator` when both exist
- a parsed MQL row still falls back to `current_status_operator` when `operator` is absent
- the shared MQL builders generate `SELECT ... operator ...` and `GROUP BY operator` for the preferred path

- [ ] **Step 2: Run the focused sync tests and confirm the new assertions fail before implementation**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture
```

Expected: fail because the current parser/query builder only knows `current_status_operator`.

- [ ] **Step 3: Implement operator-first selection with compatibility fallback**

Make only these code changes:

- in `mcp_sync.rs`, centralize assignee field candidates, issue-list/group-by MQL builders, and people-field parsing helpers
- update issue sync to try `operator` first and fall back to `current_status_operator` only when the MCP call or parse fails
- update parsed inbox rows to read `operator` first, then `current_status_operator`
- in `runtime.rs`, reuse the shared assignee-field helpers so team-member discovery follows the same operator-first / query-failure fallback policy

Do not:

- change any frontend file
- add new files or new dependencies
- change runtime-state payload shape
- broaden the fix beyond assignee source selection

- [ ] **Step 4: Re-run the full verification set**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture
cargo check --manifest-path src-tauri/Cargo.toml
```

Expected: all commands pass; warnings are acceptable if unchanged from baseline.

- [ ] **Step 5: Commit the task**

Run:

```bash
git add \
  src-tauri/src/feishu_project/mcp_sync.rs \
  src-tauri/src/feishu_project/mcp_sync_tests.rs \
  src-tauri/src/feishu_project/runtime.rs
git commit -m "fix: use assignee for feishu issue inbox"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**

Record the actual commit hash and verification result in the table above after review passes.
