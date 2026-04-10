# Feishu Operator Filter Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Feishu defect-management list and assignee filter use the actual operator/member assignments from work-item detail instead of the misleading `current_status_operator` MQL field.

**Architecture:** Keep MQL as the fast list/index source for issue IDs, titles, and status, but enrich each issue row with detail data from `get_workitem_brief` so the UI consumes `role_members.operator` for assignee display and filtering. Derive the `team_members` filter list from the enriched issue items during issue-mode sync instead of from a separate MQL `GROUP BY current_status_operator` query.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP HTTP client, serde_json, Cargo test/check.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-operator-filter-fix` on branch `fix/feishu-operator-filter-fix`
- Required setup before baseline verification: `cargo build --manifest-path bridge/Cargo.toml`
- Baseline verification before any implementation changes:
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
- Baseline result: pass (`14 passed`, `80 passed`, `cargo check` exit `0`)

## Project Memory

### Recent related commits

- `cd931f03` — restored `current_status_operator` as the sole MQL field after proving `operator` is not a valid issue `field_key`.
- `fdf8aec8` — recorded the corrected CM memory for the previous field-key fix.
- `7888ae33` — introduced the dead `operator` query path that was later removed.
- `ea461155` — finalized the Feishu MCP inbox backend and original issue-list/query wiring.

### Related plans / addendum

- `docs/superpowers/plans/2026-04-10-feishu-issues-assignee-fix.md`
- `docs/superpowers/plans/2026-04-09-feishu-mcp-inbox.md`
- `docs/superpowers/plans/2026-04-09-feishu-project-mcp-pivot.md`

### Verified runtime evidence constraining this plan

- `list_workitem_field_config(project_key=manciyuan, work_item_type=issue)` confirms `current_status_operator -> 当前负责人 -> multi-user`; `operator` is not a valid `field_key`.
- `list_workitem_role_config(project_key=manciyuan, work_item_type=issue)` confirms `operator = 经办人` and `reporter = 报告人` as role IDs.
- Live issue samples show `search_by_mql(... current_status_operator ...)` returning the same person as `get_workitem_brief(...).work_item_attribute.role_members.reporter`, while the real assignee lives in `role_members.operator`.

### Lessons that constrain this plan

- Do not put `operator` or `reporter` directly into MQL; they are role IDs, not issue `field_key`s.
- Keep the fix focused on issue-mode assignee semantics; do not change unrelated Bug Inbox behavior.
- Preserve the existing `FeishuProjectInboxItem.assignee_label` frontend contract; only fix its source.
- Because `mcp_sync.rs` and `runtime.rs` already exceed the 200-line source-file limit, split new logic into a focused helper module instead of further bloating those files.

## File Map

- Create: `src-tauri/src/feishu_project/issue_operator.rs`
- Modify: `src-tauri/src/feishu_project/mod.rs`
- Modify: `src-tauri/src/feishu_project/mcp_sync.rs`
- Modify: `src-tauri/src/feishu_project/mcp_sync_tests.rs`
- Modify: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `docs/feishu.md`
- Modify: `docs/superpowers/plans/2026-04-10-feishu-issues-assignee-fix.md`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: source issue assignees from role_members operator` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `cargo check --manifest-path src-tauri/Cargo.toml` | `f86a0bc5` — final accepted Task 1 commit. Builds on initial implementation `f2b27650`, then fixes `issue_operator::parse_operator_names()` to match the real Feishu `get_workitem_brief` response shape (`role_members` array of `{key, members}` objects). Lead re-ran the full verification set successfully on 2026-04-10. |
| Task 2 | `docs: record feishu operator filter evidence` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `cargo check --manifest-path src-tauri/Cargo.toml` | `506aa4a9` — records the verified distinction between MQL `field_key`s and role IDs, and documents that actual 经办人 comes from detail `role_members.operator`. Lead re-ran the full verification set successfully on 2026-04-10. |

---

### Task 1: Source issue assignees from detail `role_members.operator`

**task_id:** `feishu-operator-filter-code`

**Acceptance criteria:**

- Issue sync still uses MQL for list discovery, but each synced issue row is enriched with detail-derived operator names.
- `FeishuProjectInboxItem.assignee_label` for issue mode comes from `role_members.operator`, not `current_status_operator`.
- Unknown or missing `operator` detail data does not get replaced with `reporter`/`current_status_operator`; it stays empty rather than wrong.
- `team_members` for issue mode is derived from the enriched issue rows so the dropdown matches the displayed assignees.
- No frontend contract changes are introduced.

**allowed_files:**

- `src-tauri/src/feishu_project/issue_operator.rs`
- `src-tauri/src/feishu_project/mod.rs`
- `src-tauri/src/feishu_project/mcp_sync.rs`
- `src-tauri/src/feishu_project/mcp_sync_tests.rs`
- `src-tauri/src/feishu_project/runtime.rs`

**max_files_changed:** `5`

**max_added_loc:** `180`

**max_deleted_loc:** `120`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: Add failing regression tests for detail-role assignee enrichment**

Add tests that prove:

- parsing detail response extracts `role_members.operator`
- issue sync enrichment prefers detail `operator` over MQL `current_status_operator`
- issue-mode `team_members` is derived from enriched assignee labels rather than `current_status_operator`

- [ ] **Step 2: Run the focused tests and confirm they fail before implementation**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture
```

Expected: fail because issue sync currently copies `current_status_operator` into `assignee_label` and runtime `team_members` still comes from MQL `GROUP BY current_status_operator`.

- [ ] **Step 3: Implement the minimal detail-enrichment path**

Make only these code changes:

- add a small helper module that parses `get_workitem_brief` role members and extracts operator names
- keep issue MQL limited to valid issue field keys
- enrich synced issue items with detail `role_members.operator`
- derive issue-mode `team_members` from the enriched store items

Do not:

- change any frontend file
- add new dependencies
- change task-link routing behavior
- refactor unrelated Feishu MCP code

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
  src-tauri/src/feishu_project/issue_operator.rs \
  src-tauri/src/feishu_project/mod.rs \
  src-tauri/src/feishu_project/mcp_sync.rs \
  src-tauri/src/feishu_project/mcp_sync_tests.rs \
  src-tauri/src/feishu_project/runtime.rs
git commit -m "fix: source issue assignees from role_members operator"
```

### Task 2: Record the role-vs-field evidence in docs

**task_id:** `feishu-operator-filter-docs`

**Acceptance criteria:**

- `docs/feishu.md` clearly distinguishes issue MQL `field_key`s from role IDs.
- `docs/feishu.md` records the verified evidence that `current_status_operator` is a field key named “当前负责人”, while actual 经办人 is returned via detail `role_members.operator`.
- `docs/superpowers/plans/2026-04-10-feishu-issues-assignee-fix.md` records that the previous merged fix was insufficient because `current_status_operator` aligned with reporter semantics in live issue samples.

**allowed_files:**

- `docs/feishu.md`
- `docs/superpowers/plans/2026-04-10-feishu-issues-assignee-fix.md`

**max_files_changed:** `2`

**max_added_loc:** `40`

**max_deleted_loc:** `20`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: Update the docs with the verified operator/reporter evidence**

Document:

- `current_status_operator` as the valid issue `field_key`
- `operator` / `reporter` as role IDs from role config and detail `role_members`
- the live mismatch evidence that forced this repair

- [ ] **Step 2: Re-run the verification set to ensure docs-only changes did not disturb the code task**

Run the same verification commands listed above.

- [ ] **Step 3: Commit the docs task**

Run:

```bash
git add \
  docs/feishu.md \
  docs/superpowers/plans/2026-04-10-feishu-issues-assignee-fix.md
git commit -m "docs: record feishu operator filter evidence"
```

- [ ] **Step 4: Update `## CM Memory` with the real commit SHAs after lead review**

Record the actual commit hashes and verification results in the table above after each lead review passes.
