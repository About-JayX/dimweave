# Feishu Filter Query Flow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Feishu issue filtering follow the correct business flow: status filtering must be server-side, assignee options must not depend on the first loaded page, and assignee-filtered pagination must scan progressively instead of locally filtering already-loaded pages.

**Architecture:** Add a backend issue-query flow that accepts filter parameters, uses MQL `work_item_status` for server-side status filtering, uses project team membership APIs to build assignee options globally, and uses progressive raw-page scanning + detail enrichment to satisfy assignee-filtered pagination without loading the whole dataset up front. Update the frontend to send filter parameters on sync/load-more and render a new status dropdown alongside the assignee dropdown.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP HTTP client, React 19, TypeScript, Bun test runner, Cargo.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-filter-query-fix` on branch `fix/feishu-filter-query-fix`
- Required setup before baseline verification: `cargo build --manifest-path bridge/Cargo.toml`
- Baseline verification before any implementation changes:
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass

## Project Memory

### Recent related commits

- `fffde8a2` — frontend observer now attaches after sentinel mount, restoring bottom-scroll triggering.
- `7a3231b3` — load_more now enriches appended issue items and refreshes `team_members`.
- `f0d0939d` — detail role-member parsing now uses `members[].name`.
- `b60350e9` — detail enrichment uses correct `get_workitem_brief` argument shape and bounded concurrency.

### Verified runtime evidence

- `work_item_status` is a valid issue `field_key`; `WHERE work_item_status = "已关闭"` works, while filtering by key (`"CLOSED"`) does not.
- `list_project_team` and paginated `list_team_members(page_token)` work and expose full team membership.
- `search_user_info(user_keys)` resolves team-member names from user_keys.
- `operator` remains a role ID, not an MQL `field_key`.
- Current frontend still filters locally by `assigneeLabel` and only knows assignee options from `runtimeState.team_members`, so filtered pagination semantics are incomplete.

### Lessons that constrain this plan

- Do not reintroduce any direct MQL filtering on `operator`.
- Status filters must use MQL labels (`已关闭`, `处理中`, etc.), not status keys.
- Assignee-filtered pagination must be progressive and cursor-based; do not load the full issue dataset before filtering.
- Keep the existing list card contract (`statusLabel`, `assigneeLabel`) intact.

### Plan revision history

- `2026-04-10` — User approved a focused revision after lead review found that backend Task 1 exceeded the original `max_added_loc` because private MQL parsing helpers in `mcp_sync.rs` had to be duplicated into split query modules to satisfy the 200-line source-file rule without expanding `allowed_files`.

## File Map

### Backend query flow

- Create: `src-tauri/src/feishu_project/issue_query.rs`
- Create: `src-tauri/src/feishu_project/issue_query_parse.rs`
- Create: `src-tauri/src/feishu_project/issue_query_team.rs`
- Modify: `src-tauri/src/feishu_project/mod.rs`
- Modify: `src-tauri/src/feishu_project/types.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/commands_feishu_project.rs`
- Modify: `src-tauri/src/main.rs`

### Frontend filter wiring

- Create: `src/stores/feishu-project-api.ts`
- Modify: `src/stores/feishu-project-store.ts`
- Modify: `src/components/BugInboxPanel/SyncModeNav.tsx`
- Modify: `src/components/BugInboxPanel/index.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx`

### Docs

- Modify: `docs/feishu.md`
- Modify: `docs/superpowers/plans/2026-04-10-feishu-load-more-filter-fix.md`
- Modify: `docs/superpowers/plans/2026-04-10-feishu-filter-query-fix.md`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `feat: add feishu issue query filters and cursor state` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `cargo check --manifest-path src-tauri/Cargo.toml` | Must preserve existing operator-detail enrichment while adding server-side status filters, global assignee options, and progressive assignee-filtered pagination. Query helpers must be split to stay within the 200-line source-file rule, and new Tauri commands must be registered in `main.rs`. **Accepted: `8e1650fa`** |
| Task 2 | `feat: wire feishu status and assignee filters through bug inbox UI` | `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build`; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture` | UI must expose status filters and pass filter state to backend sync/load-more calls without regressing existing load-more trigger behavior. **Accepted: `989bf41b`** |
| Task 3 | `docs: record feishu filter query evidence` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | Document why `work_item_status` uses labels in MQL and why assignee options come from team membership while assignee matches come from detail `role_members.operator`. |

---

### Task 1: Add backend issue-query filters and cursor state

**task_id:** `feishu-filter-query-backend`

**Acceptance criteria:**

- Backend accepts optional issue filters: `assignee` and `status`.
- Status filter is applied in MQL via `work_item_status = "<label>"`.
- Assignee-filtered pagination scans raw issue pages progressively, enriches with detail operators, and returns up to one page of matches without requiring a full dataset load.
- Assignee option list is populated from project-team membership, not from currently loaded issue items.
- Filtered `load_more` continues from the previous raw offset/cursor for the same filter tuple.

**allowed_files:**

- `src-tauri/src/feishu_project/issue_query.rs`
- `src-tauri/src/feishu_project/issue_query_parse.rs`
- `src-tauri/src/feishu_project/issue_query_team.rs`
- `src-tauri/src/feishu_project/mod.rs`
- `src-tauri/src/feishu_project/types.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle_tests.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/commands_feishu_project.rs`
- `src-tauri/src/main.rs`

**max_files_changed:** `12`

**max_added_loc:** `720`

**max_deleted_loc:** `140`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `cargo check --manifest-path src-tauri/Cargo.toml`

- [ ] **Step 1: Add failing tests for filtered pagination and runtime filter metadata**

Add tests that prove:

- status-filtered queries build valid MQL using labels
- cursor state advances across filtered `load_more`
- assignee options are not derived solely from currently loaded issue items

- [ ] **Step 2: Run the Rust verification set and confirm the new assertions fail before implementation**

- [ ] **Step 3: Implement the minimal backend filter flow**

Create a focused helper module for:

- filter DTOs and cursor state
- status-options query (`GROUP BY work_item_status`)
- team-member option discovery (`list_project_team` + paginated `list_team_members` + `search_user_info`)
- progressive assignee-filtered page scan

Do not:

- change frontend files in this task
- add dependencies
- alter task-link behavior

- [ ] **Step 4: Re-run the Rust verification set**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/feishu_project/issue_query.rs \
  src-tauri/src/feishu_project/issue_query_parse.rs \
  src-tauri/src/feishu_project/issue_query_team.rs \
  src-tauri/src/feishu_project/mod.rs \
  src-tauri/src/feishu_project/types.rs \
  src-tauri/src/daemon/state.rs \
  src-tauri/src/daemon/feishu_project_lifecycle.rs \
  src-tauri/src/daemon/feishu_project_lifecycle_tests.rs \
  src-tauri/src/daemon/cmd.rs \
  src-tauri/src/daemon/mod.rs \
  src-tauri/src/commands_feishu_project.rs \
  src-tauri/src/main.rs
git commit -m "feat: add feishu issue query filters and cursor state"
```

### Task 2: Wire status and assignee filters through the Bug Inbox UI

**task_id:** `feishu-filter-query-frontend`

**Acceptance criteria:**

- Issues mode shows both assignee and status dropdowns.
- Changing filters resets the current issue query and resyncs from page 1.
- `load_more` uses the active filter tuple instead of blindly appending the unfiltered next page.
- Existing `IssueList` observer behavior remains intact.

**allowed_files:**

- `src/stores/feishu-project-api.ts`
- `src/stores/feishu-project-store.ts`
- `src/components/BugInboxPanel/SyncModeNav.tsx`
- `src/components/BugInboxPanel/index.tsx`
- `src/components/BugInboxPanel/index.test.tsx`

**max_files_changed:** `5`

**max_added_loc:** `180`

**max_deleted_loc:** `80`

**verification_commands:**

- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`
- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`

- [ ] **Step 1: Add failing UI/store tests for filter controls and filtered load-more wiring**

- [ ] **Step 2: Run the frontend verification set and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal UI/store wiring**

Do not:

- change unrelated components
- add new UI dependencies

- [ ] **Step 4: Re-run the verification set**

- [ ] **Step 5: Commit**

```bash
git add \
  src/stores/feishu-project-api.ts \
  src/stores/feishu-project-store.ts \
  src/components/BugInboxPanel/SyncModeNav.tsx \
  src/components/BugInboxPanel/index.tsx \
  src/components/BugInboxPanel/index.test.tsx
git commit -m "feat: wire feishu status and assignee filters through bug inbox UI"
```

### Task 3: Record the filter-query evidence in docs

**task_id:** `feishu-filter-query-docs`

**Acceptance criteria:**

- `docs/feishu.md` records the verified `work_item_status` label-based MQL filter behavior.
- `docs/feishu.md` records that global assignee options are sourced from project team membership while issue assignee matches still come from detail `role_members.operator`.

**allowed_files:**

- `docs/feishu.md`
- `docs/superpowers/plans/2026-04-10-feishu-load-more-filter-fix.md`
- `docs/superpowers/plans/2026-04-10-feishu-filter-query-fix.md`

**max_files_changed:** `3`

**max_added_loc:** `50`

**max_deleted_loc:** `20`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Update docs with the verified status-filter and assignee-option evidence**

- [ ] **Step 2: Re-run the verification set**

- [ ] **Step 3: Commit**

```bash
git add \
  docs/feishu.md \
  docs/superpowers/plans/2026-04-10-feishu-load-more-filter-fix.md \
  docs/superpowers/plans/2026-04-10-feishu-filter-query-fix.md
git commit -m "docs: record feishu filter query evidence"
```

- [ ] **Step 4: Update `## CM Memory` with real commit SHAs after lead review**
