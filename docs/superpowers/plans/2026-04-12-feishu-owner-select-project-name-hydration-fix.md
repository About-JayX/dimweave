# Feishu Owner Select Project-Name Hydration Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the Feishu current-owner select by ensuring owner options still load when `project_name` has not yet been hydrated into runtime state.

**Architecture:** Keep the single-team owner-option strategy from `4bf33dc7`, but close its startup timing gap. When filter options are fetched and runtime state does not yet contain `project_name`, the backend should fetch project info inline, use that name for team selection, and persist it into runtime state before emitting options.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP client, Cargo, Bun.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-owner-select-restore` on branch `fix/feishu-owner-select-restore`
- Baseline verification passed:
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Root-cause evidence:
  - Before `4bf33dc7`, owner options used `fetch_assignee_options()` (MQL group-by) and did not depend on `project_name`.
  - `4bf33dc7` switched owner options to `fetch_team_member_names(..., project_name.as_deref())`.
  - `save_and_restart()` clears runtime cache, `get_runtime_state()` rebuilds from config with `project_name = None`, and `hydrateIssuesFirstPage()` calls `fetchFilterOptions()` before sync has repopulated `project_name`.
  - With `project_name = None`, `issue_query_team::select_team()` returns `None`, so owner options become `[]` and the select disappears.

## Project Memory

### Recent related commits

- `4bf33dc7` — switched owner options to single-team member discovery
- `4aee18b1` — fixed `team_name` parsing for `list_project_team`
- `10f98ed8` — fixed early filter-option hydration race
- `3ed95c8f` — store re-reads runtime after fetching filter options

### Lessons that constrain this plan

- Do not revert the single-team owner-option strategy unless unavoidable.
- The minimal fix is to repair the missing `project_name` input, not to redesign the filter path again.
- Frontend contract should remain unchanged: `SyncModeNav` still reads `runtimeState.assigneeOptions`.

### Post-incident lesson (2026-04-12 incident chain)

This fix was necessary but insufficient: `project_name` hydration alone did not restore the owner select because `parse_team_members()` still expected `members[].user_key` objects while the live endpoint returns `members: string[]`. The second fix (`ad49610d`) was required.

**Constraint for future Feishu MCP changes:** When adding a new MCP dependency (like `project_name` for team selection), validate not just the new input's availability but also the downstream parser's compatibility with the real live payload shape.

## File Map

- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`

## CM Memory

| Task | Planned commit message | Actual commit | Verification | Memory |
|------|------------------------|---------------|--------------|--------|
| Task 1 | `fix: hydrate project name before owner option fetch` | `1db6e732` | `cargo test feishu_project` ✅ 116 passed; `bun test BugInboxPanel` ✅ 19 passed; `bun run build` ✅ | Repair the regression introduced by `4bf33dc7` by ensuring owner-option fetch has a usable `project_name` even immediately after restart/hydration. +78/-2, 2 files changed. |

---

### Task 1: Restore owner option loading when runtime project name is absent

**task_id:** `feishu-owner-select-restore-code`

**Acceptance criteria:**

- `fetch_filter_options()` no longer returns empty owner options solely because runtime `project_name` is still absent.
- If runtime `project_name` is missing, backend performs a project-info lookup and uses that result for single-team selection.
- Any fetched `project_name` is written back into runtime state before emitting the updated state.
- Frontend files remain unchanged.

**allowed_files:**

- `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`

**max_files_changed:** `2`

**max_added_loc:** `90`

**max_deleted_loc:** `30`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing regression test first**

Add a focused test in `feishu_project_lifecycle_tests.rs` proving the fallback path preserves/uses a fetched project name when runtime state is absent or missing `project_name`.

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement minimal backend fix**

In `feishu_project_lifecycle.rs` only:

- add a small helper to fetch/parse `search_project_info` project name for the current workspace when runtime `project_name` is missing
- use that effective project name when calling `issue_query_team::fetch_team_member_names(...)`
- persist the resolved project name back into runtime state before emitting the updated state

Do not:

- change frontend code
- change the single-team selection rules
- revert to the old MQL group-by owner-option source

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/daemon/feishu_project_lifecycle.rs \
  src-tauri/src/daemon/feishu_project_lifecycle_tests.rs
git commit -m "fix: hydrate project name before owner option fetch"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
