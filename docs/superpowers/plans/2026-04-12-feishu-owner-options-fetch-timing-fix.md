# Feishu Owner Options Fetch-Timing Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Feishu owner options fetch only on app-open hydration and workspace changes, not on sync-mode changes or ordinary filter interactions.

**Architecture:** Keep the current owner-option source and list-filter flow unchanged. Move the refresh decision to the frontend store layer: first-page hydration still fetches filter options once, mode changes stop forcing a refresh, and config saves only re-fetch filter options when the workspace actually changes.

**Tech Stack:** TypeScript, React, Zustand, Bun.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-owner-options-fetch-timing` on branch `fix/feishu-owner-options-fetch-timing`
- Baseline verification passed:
  - `bun test src/stores/feishu-project-store.test.ts src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Current behavior from code:
  - `hydrateIssuesFirstPage()` always calls `fetchFilterOptions()` on panel open
  - `handleModeChange()` in `src/components/BugInboxPanel/index.tsx` also calls `fetchFilterOptions()`
  - ordinary assignee/status filter changes do **not** refresh owner options
- User requirement:
  - owner list refresh on app open is acceptable
  - afterwards refresh owner options only when `workspace` changes

## Project Memory

### Recent related commits

- `ad49610d` — fixed real `list_team_members` payload shape
- `1db6e732` — hydrated `project_name` before owner option fetch
- `4bf33dc7` — switched owner options to team-based source

### Lessons that constrain this plan

- Do not touch backend Feishu query behavior for this task.
- This is a fetch-timing/UI-state fix only.
- Keep the “validate live payloads before merge” lesson in mind, but no new MCP parser changes are planned here.

## File Map

- Modify: `src/stores/feishu-project-store.ts`
- Modify: `src/stores/feishu-project-store.test.ts`
- Modify: `src/components/BugInboxPanel/index.tsx`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: only refresh feishu owner options on workspace change` | `bun test src/stores/feishu-project-store.test.ts src/components/BugInboxPanel/index.test.tsx`; `bun run build` | Owner options should refresh on app-open hydration and workspace changes only; mode changes must not trigger redundant owner-option fetches. |

---

### Task 1: Restrict owner-option refresh timing

**task_id:** `feishu-owner-options-fetch-timing-fix`

**Acceptance criteria:**

- App-open first-page hydration still fetches filter options once.
- Sync mode changes no longer force a filter-option refresh.
- Saving config only re-fetches filter options when `workspace_hint` actually changes.
- Status/assignee filtering behavior remains unchanged.

**allowed_files:**

- `src/stores/feishu-project-store.ts`
- `src/stores/feishu-project-store.test.ts`
- `src/components/BugInboxPanel/index.tsx`

**max_files_changed:** `3`

**max_added_loc:** `90`

**max_deleted_loc:** `40`

**verification_commands:**

- `bun test src/stores/feishu-project-store.test.ts src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing tests first**

Add store/component tests proving:

- initial hydration still calls `fetch_filter_options`
- mode change does not call `fetch_filter_options`
- workspace-changing save does call `fetch_filter_options`

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal timing fix**

Make only these changes:

- in `feishu-project-store.ts`, compare previous `workspaceHint` against the saved config and re-fetch options only when the workspace changes
- in `BugInboxPanel/index.tsx`, remove the unconditional `fetchFilterOptions()` from `handleModeChange()`

Do not:

- change backend files
- change list filtering logic
- change owner option data source

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src/stores/feishu-project-store.ts \
  src/stores/feishu-project-store.test.ts \
  src/components/BugInboxPanel/index.tsx
git commit -m "fix: only refresh feishu owner options on workspace change"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
