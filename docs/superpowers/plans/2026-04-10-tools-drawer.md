# Tools Drawer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current bug-specific shell drawer entry with a generic tools drawer that uses a wrench icon and contains collapsible Telegram and Feishu Project sections.

**Architecture:** Keep the internal shell pane key as `bugs` to avoid a broader state migration, but change the visible rail/header copy to `Tools`. Insert a new `ToolsPanel` composition layer between `TaskContextPopover` and the existing `TelegramPanel` / `BugInboxPanel`, then remove Telegram from the Agents drawer so the tool surfaces exist in one place.

**Tech Stack:** React 19, TypeScript, Tailwind CSS, Bun test runner, Vite build.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/2026-04-10-tools-drawer` on branch `feat/tools-drawer`
- Baseline verification before any changes:
  - `bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass (17 tests), build success

## Project Memory

### Recent related commits

- `de6fcb99` — Bug Inbox shell panel introduced the current drawer slot and badge path.
- `6e8c79ed` — Feishu inbox UI now assumes MCP-driven config/runtime copy.
- `c3b18acc` — Telegram panel UX and pairing behavior were stabilized; do not regress them.
- `9ef63dfb` — current Feishu frontend workflow and tests were finalized.

### Related plans / addendum

- `docs/superpowers/plans/2026-04-09-feishu-project-bug-inbox.md`
- `docs/superpowers/plans/2026-04-09-feishu-project-mcp-pivot.md`
- `docs/superpowers/plans/2026-04-09-fix-tg-and-task-panel.md`
- `docs/superpowers/plans/2026-04-09-fix-tg-and-task-panel-review-addendum.md`

### Lessons that constrain this plan

- Preserve the existing shell drawer extension points; do not introduce a new navigation surface.
- Preserve the Feishu badge signal on the rail.
- Keep Telegram runtime logic untouched; only relocate its panel in the UI.
- Keep scope tight by avoiding an internal `bugs` → `tools` type rename.

## File Map

### UI shell and tools grouping

- Modify: `src/components/ShellContextBar.tsx`
- Modify: `src/components/TaskContextPopover.tsx`
- Modify: `src/components/AgentStatus/index.tsx`
- Create: `src/components/ToolsPanel/index.tsx`

### Embedded panel polish

- Modify: `src/components/BugInboxPanel/index.tsx` (only if needed to fit the new section container cleanly)

### Tests

- Modify: `src/components/ShellContextBar.test.tsx`
- Modify: `src/components/TaskContextPopover.test.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx` (only if embedding markup changes)
- Create: `src/components/ToolsPanel/index.test.tsx`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `feat: regroup telegram and feishu under tools drawer` | `bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/ToolsPanel/index.test.tsx src/components/BugInboxPanel/index.test.tsx`; `bun run build`; `git diff --check` | Keep the existing `bugs` pane identity and Feishu badge path from `de6fcb99`; keep Telegram behavior intact per `c3b18acc`; keep Feishu MCP surface wording aligned with `6e8c79ed` and `9ef63dfb`. |

---

### Task 1: Regroup Telegram and Feishu under the tools drawer

**task_id:** `tools-drawer-ui`

**Acceptance criteria:**

- The shell rail entry previously labeled `Bug Inbox` now renders as `Tools`.
- The rail entry uses a wrench-style icon while preserving the existing Feishu badge count.
- The drawer header copy is tools-oriented rather than bug-specific.
- The drawer body renders a new `ToolsPanel` with two disclosure sections: `Telegram` and `Feishu Project`.
- `Feishu Project` is expanded by default and `Telegram` is collapsed by default.
- Each section exposes an expand/collapse affordance.
- `TelegramPanel` is removed from `AgentStatusPanel`.
- Existing Telegram and Feishu inner functionality remains mounted from their original components; no runtime/store behavior changes are introduced.
- Focused frontend tests and `bun run build` pass.

**allowed_files:**

- `src/components/ShellContextBar.tsx`
- `src/components/TaskContextPopover.tsx`
- `src/components/AgentStatus/index.tsx`
- `src/components/BugInboxPanel/index.tsx`
- `src/components/ShellContextBar.test.tsx`
- `src/components/TaskContextPopover.test.tsx`
- `src/components/BugInboxPanel/index.test.tsx`
- `src/components/ToolsPanel/index.tsx`
- `src/components/ToolsPanel/index.test.tsx`

**max_files_changed:** `9`

**max_added_loc:** `260`

**max_deleted_loc:** `140`

**verification_commands:**

- `bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/ToolsPanel/index.test.tsx src/components/BugInboxPanel/index.test.tsx`
- `bun run build`
- `git diff --check`

- [ ] **Step 1: Add failing UI tests for the renamed rail item and new tools drawer sections**

Update and add tests so they assert:

- `ShellContextBar` renders `Tools` instead of `Bug Inbox`
- the drawer header uses tools wording
- `ToolsPanel` renders `Telegram` and `Feishu Project`
- default disclosure state is `Telegram` collapsed / `Feishu Project` expanded
- the Agents drawer no longer includes Telegram

- [ ] **Step 2: Run the focused frontend tests and confirm the new assertions fail before implementation**

Run:

```bash
bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/ToolsPanel/index.test.tsx src/components/BugInboxPanel/index.test.tsx
```

Expected: fail because the shell still says `Bug Inbox`, no `ToolsPanel` exists, and Telegram still lives in the Agents drawer.

- [ ] **Step 3: Implement the tools drawer composition without changing runtime wiring**

Make only the planned UI changes:

- swap the bug icon/label for a wrench-style `Tools` affordance in `ShellContextBar`
- update `TaskContextPopover` metadata for the `bugs` pane to tools-oriented copy
- create `ToolsPanel` as a disclosure container that composes `TelegramPanel` and `BugInboxPanel`
- remove `TelegramPanel` from `AgentStatusPanel`
- if Feishu layout bleeds because of its existing full-pane margins, make the smallest embedding-only adjustment in `BugInboxPanel/index.tsx`

Do not:

- rename `ShellSidebarPane`
- change store contracts
- change Telegram or Feishu runtime logic
- add new dependencies

- [ ] **Step 4: Re-run the full verification set**

Run:

```bash
bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/ToolsPanel/index.test.tsx src/components/BugInboxPanel/index.test.tsx
bun run build
git diff --check
```

Expected: all tests pass, build succeeds, and no whitespace errors remain.

- [ ] **Step 5: Commit the task**

Run:

```bash
git add \
  src/components/ShellContextBar.tsx \
  src/components/TaskContextPopover.tsx \
  src/components/AgentStatus/index.tsx \
  src/components/BugInboxPanel/index.tsx \
  src/components/ShellContextBar.test.tsx \
  src/components/TaskContextPopover.test.tsx \
  src/components/BugInboxPanel/index.test.tsx \
  src/components/ToolsPanel/index.tsx \
  src/components/ToolsPanel/index.test.tsx
git commit -m "feat: regroup telegram and feishu under tools drawer"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after review**

Record the actual commit hash and verification result in the table above after lead review passes.
