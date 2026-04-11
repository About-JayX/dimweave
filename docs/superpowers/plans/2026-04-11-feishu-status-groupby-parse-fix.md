# Feishu Status GROUP BY Parse Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the issue status filter actually render by fixing the backend parser to handle the real `search_by_mql ... GROUP BY work_item_status` payload shape returned by Feishu MCP.

**Architecture:** Keep the filter-query flow and frontend wiring intact. Fix only the status enum extraction layer: `parse_status_group_by()` must parse the real Feishu `list[].group_infos[].group_name` structure instead of assuming the normal MQL `data.*.moql_field_list` shape.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP HTTP client, Bun build/test.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-status-groupby-fix` on branch `fix/feishu-status-groupby-fix`
- Baseline verification before implementation:
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass

## Project Memory

### Recent related commits

- `8e1650fa` — added backend filtered-query flow and `fetch_status_options()`.
- `989bf41b` — wired status dropdown rendering from `runtimeState.statusOptions`.
- `10f98ed8` — sequenced first-render hydration so filter options are requested after runtime state exists.
- `cbc88fb1` — preserved `statusOptions` / `assigneeOptions` across runtime rebuilds.
- `3ed95c8f` — store now re-reads runtime state after requesting filter options.

### Verified runtime evidence

- Real MCP call:
  - `SELECT work_item_status FROM manciyuan.issue GROUP BY work_item_status`
- Real JSON text payload shape:
  - top-level `list`
  - each row has `group_infos`
  - status label lives at `group_infos[].group_name`
- Current parser in `issue_query_parse.rs` incorrectly expects:
  - top-level `data`
  - nested `moql_field_list`
- Therefore current code returns `Ok(Vec::new())` for status options even though Feishu returns real groups.

### Lessons that constrain this plan

- Do not change frontend contracts again for this fix.
- Do not rework the filter query model; the remaining defect is in payload parsing.
- Use a real-payload regression test so this exact bug cannot recur.

## File Map

- Modify: `src-tauri/src/feishu_project/issue_query_parse.rs`
- Modify: `src-tauri/src/feishu_project/issue_query.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: parse feishu status group results from list payload` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | Real Feishu status GROUP BY responses use `list[].group_infos[].group_name`; do not assume the standard `data.moql_field_list` shape for grouped responses. **Accepted: `626fd6f1`** |

---

### Task 1: Fix status GROUP BY parsing

**task_id:** `feishu-status-groupby-parse`

**Acceptance criteria:**

- `parse_status_group_by()` returns non-empty labels for the real Feishu `list/group_infos/group_name` payload shape.
- Existing status-filter query flow remains unchanged.
- No frontend files are modified.

**allowed_files:**

- `src-tauri/src/feishu_project/issue_query_parse.rs`
- `src-tauri/src/feishu_project/issue_query.rs`

**max_files_changed:** `2`

**max_added_loc:** `80`

**max_deleted_loc:** `30`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add a failing regression test using the real GROUP BY payload shape**

- [ ] **Step 2: Run the verification set and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal parser fix**

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/feishu_project/issue_query_parse.rs \
  src-tauri/src/feishu_project/issue_query.rs
git commit -m "fix: parse feishu status group results from list payload"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
