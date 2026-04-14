# Task Root Split And Agent DnD Design

> **Scope:** Task/workspace semantics and agent drag-and-drop reliability only. Telegram is explicitly out of scope for this fix.

## Goal

Fix two regressions in the current `main` behavior:

1. Creating a new task must not make older tasks disappear from the task list.
2. Agent reordering inside a task must work in the real app, not just in synthetic tests.

## Root Cause

### 1. Multi-task list disappears after creating a new task

The current model overloads one field with two different meanings:

- the selected project root chosen by the user
- the per-task git worktree path created for execution

Today the task graph stores only `workspace_root`, and task creation rewrites it to the task-specific worktree path. The frontend then hydrates `selectedWorkspace` from the active task snapshot and filters the visible task list by exact `workspaceRoot === selectedWorkspace`.

That means:

- create task A → selected workspace becomes task A worktree
- create task B → selected workspace becomes task B worktree
- task A no longer matches the selected workspace filter

So the UI looks like the new task overwrote the old one, even though both tasks still exist.

### 2. Agent drag-and-drop has no effect in the real app

The current agent reorder UI uses HTML5 drag events, but the implementation only tracks in-memory source/target indexes. The current tests manually synthesize events and do not prove the browser/webview drag contract.

The result is a fragile drag path that can pass tests while failing in the actual app runtime.

## Design

### Task root split

Replace the single overloaded task path with two explicit fields:

- `project_root`: the stable user-selected project root
- `task_worktree_root`: the actual worktree path for that task

Rules:

- task list grouping/filtering uses `project_root`
- provider launch, runtime cwd, and worktree operations use `task_worktree_root`
- frontend `selectedWorkspace` must stay aligned with `project_root`, never the task worktree path

This removes the semantic collision instead of patching around it in selectors.

### Task list behavior

The workspace task list continues to be derived from the selected project root, ordered newest-first by creation time.

Creating a new task must:

- preserve older tasks in the same `project_root`
- add the new task to that same grouped list
- make the new task active/expanded

### Agent drag-and-drop behavior

Keep the existing reorder command path and persistence semantics, but harden the UI to real drag behavior:

- use explicit drag payload state that works in the actual DOM/webview event model
- keep the reorder computation deterministic
- verify reorder from real component interaction tests, not only helper tests

No agent model changes are needed here; this is strictly a UI interaction reliability fix.

## Non-Goals

- Telegram routing/ownership changes
- SQLite migration for all daemon state
- Task panel visual redesign beyond what is needed to keep the list working
- Reordering tasks themselves

## Why Not A Frontend-Only Patch

A frontend-only normalization layer would keep the broken semantics in the stored task model and force the UI to reverse-engineer “real project root” from worktree paths. That is worse for long-term maintenance.

The cleaner model is to represent both concepts explicitly in the backend and let the frontend use the right field directly.

## Acceptance Criteria

- tasks in the same project remain visible after creating additional tasks
- task list filtering/grouping uses stable project root semantics, not worktree path equality
- active task hydration does not overwrite the selected project root with the task worktree path
- agent drag-and-drop works in the real app and persists order through the existing reorder command path
- focused regression tests cover both behaviors
