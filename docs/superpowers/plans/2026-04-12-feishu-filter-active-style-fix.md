# Feishu Filter Active-Style Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the Feishu status/current-owner dropdown option lists stable and render the currently selected option with an active style instead of removing it from the menu.

**Architecture:** Fix the menu contract rather than patching per dropdown. Extend `ActionMenuItem` with an explicit active state, render active styling in `ActionMenu`, and stop filtering the selected option out of `SyncModeNav` menus. The trigger label continues to show the current selection, and the dropdown contents remain stable.

**Tech Stack:** React 19, TypeScript, Bun, Vite.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-filter-active-style-fix` on branch `fix/feishu-filter-active-style-fix`
- Baseline verification before implementation:
  - `bun test src/components/BugInboxPanel/index.test.tsx`
  - `bun run build`
- Baseline result: pass

## Project Memory

### Recent related commits

- `862f8875` — visible filtered view is now separate from raw cache; list replacement/appending semantics are fixed.
- `7cb061bd` — current-owner MQL filtering is wired through the Bug Inbox UI.
- `1572ae51` / `aac88ef4` — issues-area hydration gate controls when the filter bar appears.
- `67a843a8` — BugInboxPanel tests already account for ActionMenu portal behavior and should be extended rather than replaced.

### Verified root-cause evidence

- `src/components/BugInboxPanel/SyncModeNav.tsx` currently does:
  - `statusOptions.filter((s) => s !== statusFilter)`
  - `teamMembers.filter((a) => a !== assigneeFilter)`
- So the selected option is removed from the dropdown list entirely.
- `src/components/AgentStatus/ActionMenu.tsx` has no active/selected item contract and no active styling.
- Result: trigger text changes, but the dropdown itself neither preserves the selected entry nor highlights it.

### Lessons that constrain this plan

- Do not refresh or reorder the option data to fake active state.
- Do not change backend behavior; this is a pure frontend interaction fix.
- Keep the menu option order stable.
- Reuse the existing `ActionMenu` portal component instead of introducing a new dropdown widget.

## File Map

- Modify: `src/components/AgentStatus/ActionMenu.tsx`
- Modify: `src/components/BugInboxPanel/SyncModeNav.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: show active feishu filter option in dropdown` | `bun test src/components/BugInboxPanel/index.test.tsx`; `bun run build` | The selected filter option must remain in the dropdown and have an explicit active style. The option list itself must stay stable. **Accepted: `eb83a0d6`** |

---

### Task 1: Keep selected filter options visible and styled

**task_id:** `feishu-filter-active-style-code`

**Acceptance criteria:**

- Status and current-owner dropdowns keep the selected option in the option list.
- The selected option has an explicit active style in the dropdown.
- Option order stays stable except for the existing “全部状态 / 全部经办人” reset entry behavior.
- No backend files change.

**allowed_files:**

- `src/components/AgentStatus/ActionMenu.tsx`
- `src/components/BugInboxPanel/SyncModeNav.tsx`
- `src/components/BugInboxPanel/index.test.tsx`

**max_files_changed:** `3`

**max_added_loc:** `90`

**max_deleted_loc:** `40`

**verification_commands:**

- `bun test src/components/BugInboxPanel/index.test.tsx`
- `bun run build`

- [ ] **Step 1: Add failing tests first**

Add tests proving:

- selected status/current-owner options are still present in the menu model
- selected options are marked active
- reset entries still appear only when a non-empty filter is selected

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal UI fix**

Make only these changes:

- add `active?: boolean` to `ActionMenuItem`
- render an active visual state in `ActionMenu`
- stop filtering out the selected value in `SyncModeNav`; instead mark it active

Do not:

- change backend or store code
- reorder options beyond existing reset-entry behavior
- introduce a new dropdown component

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src/components/AgentStatus/ActionMenu.tsx \
  src/components/BugInboxPanel/SyncModeNav.tsx \
  src/components/BugInboxPanel/index.test.tsx
git commit -m "fix: show active feishu filter option in dropdown"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
