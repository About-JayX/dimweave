# Feishu Owner Select Members-Shape Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore the Feishu owner select by fixing the real `list_team_members` payload parsing used by the single-team owner-option source.

**Architecture:** Keep the single-team owner-option strategy and the recent project-name hydration fix. Repair only the member parsing layer so `list_team_members` can consume the real payload shape returned by Feishu MCP. This is a parser fix, not another query-flow redesign.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP client, Cargo, Bun.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-owner-select-members-shape-fix` on branch `fix/feishu-owner-select-members-shape-fix`
- Recent accepted related commits on `main`:
  - `4bf33dc7` — switched owner options to single-team member discovery
  - `4aee18b1` — fixed `team_name` parsing
  - `1db6e732` / `53751e15` — fixed missing `project_name` hydration before owner option fetch
- Live MCP evidence gathered after those fixes:
  - `search_project_info(project_key=manciyuan)` returns `name = 极光矩阵--娱乐站`
  - `list_project_team(project_key=manciyuan)` returns teams with `team_name`
  - `list_team_members(project_key=manciyuan, team_id=7612468934737956047, page_size=200)` returns:
    - top-level `members`
    - `members` is an array of **user_key strings**, not objects
  - current parser in `issue_query_team.rs` expects `members[].user_key`, so it returns `[]`
  - `search_user_info(user_keys)` returns a top-level JSON array and is already parseable
- This explains the current regression:
  - owner select disappears because `assigneeOptions` stays empty even after project-name hydration

## Project Memory

### Recent related commits

- `4bf33dc7`
- `4aee18b1`
- `1db6e732`
- `53751e15`

### Lessons that constrain this plan

- Do not revert the single-team strategy.
- Do not touch frontend code again unless forced by evidence.
- Fix the real payload mismatch at the parser boundary only.

### Post-incident lesson (2026-04-12 incident chain: `4bf33dc7` → `1db6e732` → `ad49610d`)

This was the final fix in a three-commit chain. The root cause was that `4bf33dc7` wrote parsers against a guessed `members[].user_key` shape without verifying the live MCP response, which actually returns `members: string[]`.

**Constraint for future Feishu MCP changes:** Before merging any Feishu parser or query change, call the real MCP endpoint with the current token/workspace and confirm the actual response structure. Record the confirmed payload shape (with a date stamp) in `docs/feishu.md` and in the plan's baseline evidence. Do not rely on field names from documentation alone — the MCP tool layer may transform them.

## File Map

- Modify: `src-tauri/src/feishu_project/issue_query_team.rs`
- Modify: `docs/feishu.md`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: parse string member keys for feishu owner options` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`; `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | Repair the real `list_team_members` response mismatch: `members` is an array of string user keys, not objects. |

---

### Task 1: Fix single-team member parsing for owner options

**task_id:** `feishu-owner-select-members-shape-fix`

**Acceptance criteria:**

- `parse_team_members()` accepts the real Feishu payload where `members` is an array of string user keys.
- Existing fallback/object parsing is preserved if cheap to keep.
- `fetch_team_member_names()` can produce non-empty user keys from the live single-team call.
- Frontend contracts remain unchanged.

**allowed_files:**

- `src-tauri/src/feishu_project/issue_query_team.rs`
- `docs/feishu.md`

**max_files_changed:** `2`

**max_added_loc:** `70`

**max_deleted_loc:** `20`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing regression test first**

Add a test proving `parse_team_members()` parses a real payload like:

```json
{"members":["7620253762535378105","7611423493078535394"]}
```

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal parser fix**

In `issue_query_team.rs`:

- update `parse_team_members()` to support `members` as an array of strings
- preserve support for object entries if possible without extra complexity
- do not change the single-team selection logic

In `docs/feishu.md`:

- add the confirmed live payload note that `list_team_members` returns `members: string[]` for this workspace

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/feishu_project/issue_query_team.rs \
  docs/feishu.md
git commit -m "fix: parse string member keys for feishu owner options"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
