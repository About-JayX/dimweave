# Feishu Current-Owner Filter Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current detail-scan assignee filter with a pure MQL current-owner + status filter flow so issue filtering is fast and uses the correct business definition: current owner.

**Architecture:** Keep issue listing and status filtering on `search_by_mql`. Change the assignee/current-owner path to use `current_status_operator` directly in MQL and source dropdown options from `GROUP BY current_status_operator`. Remove the progressive `get_workitem_brief` scan from filter queries; only keep detail calls where they are still needed outside this filter path.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP HTTP client, React 19, TypeScript, Bun, Cargo.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-current-owner-filter-fix` on branch `fix/feishu-current-owner-filter-fix`
- Verified live MCP syntax and semantics before planning:
  - `SELECT work_item_status FROM manciyuan.issue GROUP BY work_item_status` ✅
  - `SELECT current_status_operator FROM manciyuan.issue GROUP BY current_status_operator` ✅
  - `SELECT work_item_id, name, current_status_operator, work_item_status FROM manciyuan.issue WHERE work_item_status = "已关闭" LIMIT 0,5` ✅
  - `SELECT work_item_id, name FROM manciyuan.issue WHERE current_status_operator IN ("铃铛") LIMIT 0,1` ✅
  - `SELECT work_item_id, name FROM manciyuan.issue WHERE work_item_status = "已关闭" AND current_status_operator IN ("牛丸") LIMIT 0,5` ✅（语法正确，当前数据返回 0 条）
- Important interpretation:
  - The query shape is valid.
  - `已关闭 + 牛丸` currently has no matching rows in live data, so empty result is a data fact, not a syntax failure.

## Project Memory

### Recent related commits

- `c0a3b5d0` — first-page hydration CM update
- `1572ae51` / `aac88ef4` — issues-area hydration gate
- `626fd6f1` — status GROUP BY parsing from real Feishu `list[].group_infos[].group_name`
- `3ed95c8f` — re-read runtime state after filter-options fetch
- `8e1650fa` — introduced current filtered-query architecture with status MQL + assignee scan
- `989bf41b` — wired status and assignee filters through Bug Inbox UI
- `f2b27650` / `f86a0bc5` / `b60350e9` — operator-detail enrichment path that is now the wrong basis for filtering

### Verified runtime evidence

- `current_status_operator` is a valid MQL `field_key` for issue data and grouping.
- `current_status_operator_role` is also queryable, but it expresses current role state, not needed for the requested UI filter.
- `operator` / `reporter` are **not** valid MQL `field_key`s; they cause metadata errors when used in `WHERE`.
- Current implementation is wrong for this business rule because it filters by enriched `role_members.operator` via `scan_assignee_page()` and can stall on large scans.
- User-defined business rule is now explicit:
  - “经办人” = **当前负责人**
  - filter tuple = `work_item_status` + `current_status_operator`

### Lessons that constrain this plan

- Do not use `get_workitem_brief(...).role_members.operator` as the filtering source.
- Do not keep the scan-based assignee filter path in place “just in case”; it is both slow and semantically wrong for the current requirement.
- Keep list queries as `page + limit + filter tuple`; do not preload the whole dataset.
- Dropdown options may still be loaded separately via `GROUP BY`, but list queries themselves must be pure MQL.

## File Map

- Modify: `src-tauri/src/feishu_project/mcp_sync.rs`
- Modify: `src-tauri/src/feishu_project/mcp_sync_tests.rs`
- Modify: `src-tauri/src/feishu_project/issue_query.rs`
- Modify: `src-tauri/src/feishu_project/issue_query_parse.rs`
- Modify: `src-tauri/src/feishu_project/mod.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- Modify: `src/components/BugInboxPanel/SyncModeNav.tsx`
- Modify: `src/components/BugInboxPanel/index.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: filter feishu issues by current owner via mql` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | Replace scan-based assignee filtering with pure MQL `current_status_operator` + `work_item_status`. The live query syntax has been verified before implementation. |

---

### Task 1: Switch issue filtering to pure MQL current owner + status

**task_id:** `feishu-current-owner-filter-code`

**Acceptance criteria:**

- Issue list query uses only MQL filters: `work_item_status` and `current_status_operator`.
- Assignee/current-owner dropdown options come from `GROUP BY current_status_operator`, not team membership APIs and not enriched loaded items.
- Filter changes no longer trigger progressive detail scans.
- The displayed assignee on cards matches `current_status_operator`.
- Existing first-page hydration gate and load-more behavior remain intact.

**allowed_files:**

- `src-tauri/src/feishu_project/mcp_sync.rs`
- `src-tauri/src/feishu_project/mcp_sync_tests.rs`
- `src-tauri/src/feishu_project/issue_query.rs`
- `src-tauri/src/feishu_project/issue_query_parse.rs`
- `src-tauri/src/feishu_project/mod.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- `src/components/BugInboxPanel/SyncModeNav.tsx`
- `src/components/BugInboxPanel/index.tsx`
- `src/components/BugInboxPanel/index.test.tsx`

**max_files_changed:** `10`

**max_added_loc:** `220`

**max_deleted_loc:** `180`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing tests first**

Add tests proving:

- issue MQL includes `current_status_operator`
- filtered query builder produces `WHERE work_item_status = "..." AND current_status_operator IN ("...")`
- current-owner options parse correctly from `GROUP BY current_status_operator`
- frontend uses the current-owner option list source instead of loaded-item-derived names

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal filter-flow correction**

Make only these code changes:

- add `current_status_operator` back to issue list MQL and parse it into `assignee_label`
- change filtered query builder to append `current_status_operator IN (...)`
- replace assignee option fetch with `GROUP BY current_status_operator`
- stop using detail-scan filtering in `load_more_filtered()`
- wire the frontend dropdown to the current-owner option list

Do not:

- reintroduce `role_members.operator` as a filter source
- add new dependencies
- redesign the first-page hydration gate

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/feishu_project/mcp_sync.rs \
  src-tauri/src/feishu_project/mcp_sync_tests.rs \
  src-tauri/src/feishu_project/issue_query.rs \
  src-tauri/src/feishu_project/issue_query_parse.rs \
  src-tauri/src/feishu_project/mod.rs \
  src-tauri/src/daemon/feishu_project_lifecycle.rs \
  src-tauri/src/daemon/feishu_project_lifecycle_tests.rs \
  src/components/BugInboxPanel/SyncModeNav.tsx \
  src/components/BugInboxPanel/index.tsx \
  src/components/BugInboxPanel/index.test.tsx
git commit -m "fix: filter feishu issues by current owner via mql"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
