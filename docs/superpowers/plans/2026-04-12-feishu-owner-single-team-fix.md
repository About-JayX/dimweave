# Feishu Owner Single-Team Options Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Feishu current-owner dropdown complete for the current workspace while reducing option-fetch calls by sourcing names from the single relevant project team instead of truncated MQL group-by results.

**Architecture:** Keep issue list filtering as pure MQL on `work_item_status + current_status_operator`. Replace the owner dropdown source only: resolve the relevant project team from `search_project_info` + `list_project_team`, fetch members for that one team with a large `page_size`, resolve display names via `search_user_info`, and cache the resulting names into `assigneeOptions`.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP HTTP client, Bun, Cargo.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-owner-single-team-fix` on branch `fix/feishu-owner-single-team-fix`
- Verified live evidence before implementation:
  - `search_project_info(project_key=manciyuan)` returns `name = 极光矩阵--娱乐站`
  - `list_project_team(project_key=manciyuan)` returns 3 teams: `娱乐站--基座`, `UXD`, `基座-前端`
  - Known issue owners (`橙子`, `铃铛`, `大永`, `牛丸`, `Grape`) all belong to `娱乐站--基座`
  - `jay` also belongs to `娱乐站--基座`
  - `list_team_members(team_id=娱乐站--基座, page_size=200)` returns all members in one page
  - `search_user_info(user_keys)` resolves all display names in one call
- Current owner-option source is incomplete:
  - `SELECT current_status_operator ... GROUP BY current_status_operator` returns only a top-N subset

## Project Memory

### Recent related commits

- `218ab5ab` — dropdown keeps selected option visible and styled
- `862f8875` — visible filtered view is now separate from raw cache
- `5e6329ff` / `b9eba99e` / `7cb061bd` — current-owner filter flow is already pure MQL and should remain unchanged

### Lessons that constrain this plan

- Do not touch the issue list MQL filter path again.
- Only replace the owner dropdown source.
- The optimization is workspace-specific but should still degrade safely: if no unique matching team is found, return an empty list or documented fallback rather than guessing silently.

## File Map

- Modify: `src-tauri/src/feishu_project/issue_query_team.rs`
- Modify: `src-tauri/src/feishu_project/issue_query.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- Modify: `docs/feishu.md`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: source feishu owner options from project team` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | Replace truncated `GROUP BY current_status_operator` owner options with single-team member discovery based on current workspace project/team naming. Keep list filtering as pure MQL. **Accepted: `4bf33dc7`**, follow-up **`4aee18b1`** fixes real `list_project_team` parsing to use `team_name`. |

---

### Task 1: Source owner options from the relevant project team

**task_id:** `feishu-owner-single-team-code`

**Acceptance criteria:**

- `assigneeOptions` no longer depend on `GROUP BY current_status_operator`.
- For the current workspace, owner options include names such as `jay` that were missing from the truncated group-by result.
- Owner option fetching uses one project-team lookup, one team-member page fetch, and one user-info lookup after initialize for the current workspace.
- Issue list filtering remains pure MQL and unchanged.
- Frontend contracts remain unchanged.

**allowed_files:**

- `src-tauri/src/feishu_project/issue_query_team.rs`
- `src-tauri/src/feishu_project/issue_query.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- `docs/feishu.md`

**max_files_changed:** `5`

**max_added_loc:** `170`

**max_deleted_loc:** `80`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing tests first**

Add tests proving:

- project/team selection prefers the unique team matching the project name suffix
- real `list_team_members` response shape (`members`, not `data`) is parsed correctly
- real `search_user_info` response shape (top-level array) is parsed correctly
- `fetch_filter_options()` uses team-member owner options rather than current-owner group-by options

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal owner-option source swap**

Make only these changes:

- add project-team selection logic to `issue_query_team.rs`
- parse `list_team_members` and `search_user_info` using their real payload shapes
- rewire `fetch_filter_options()` / `fetch_assignee_options()` to call the single-team member source
- document the real payloads and current-workspace single-team optimization in `docs/feishu.md`

Do not:

- change issue list filtering logic
- reintroduce MQL group-by as the owner source
- add frontend changes

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/feishu_project/issue_query_team.rs \
  src-tauri/src/feishu_project/issue_query.rs \
  src-tauri/src/daemon/feishu_project_lifecycle.rs \
  src-tauri/src/daemon/feishu_project_lifecycle_tests.rs \
  docs/feishu.md
git commit -m "fix: source feishu owner options from project team"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
