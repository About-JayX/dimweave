# Agent Status Sidebar Infinite Loop Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stop the Agents sidebar from triggering React's `Maximum update depth exceeded` error when `AgentStatusPanel` mounts.

**Architecture:** Keep the fix local to the task-store selector path that feeds `AgentStatusPanel`. The current selector returns a fresh object on every snapshot, which is unsafe with Zustand/React subscription semantics. Stabilize that selector output for unchanged task/summary inputs and lock the behavior with a focused regression test.

**Tech Stack:** React 19, Zustand 5, TypeScript, Bun test, Vite

---

## Memory

- Related commits:
  - `737746b5` — task-scoped frontend bindings landed and introduced `selectActiveTaskProviderBindings()`
  - `636a4107` — extended that selector with provider-session fields
  - `bca07674` — latest clean `main` baseline before this bugfix
- Related prior plan:
  - `docs/superpowers/plans/2026-04-13-task-scoped-runtime-redesign.md`
- Constraint carried forward:
  - Task-scoped provider bindings must remain the source of truth for AgentStatus.
  - Do not reintroduce global provider-session ownership into the sidebar.

## Root Cause

- `src/components/AgentStatus/index.tsx` subscribes with `useTaskStore(selectActiveTaskProviderBindings)`.
- `selectActiveTaskProviderBindings()` in `src/stores/task-store/selectors.ts` constructs a new object on every call, even when the input snapshot is unchanged.
- With React 19 + Zustand 5, that unstable selector result can cause repeated snapshot churn during mount, surfacing as `Maximum update depth exceeded`.

## Task 1: Stabilize active-task provider bindings selector

**task_id:** `agent-status-sidebar-infinite-loop`

**allowed_files:**

- `src/stores/task-store/selectors.ts`
- `tests/task-store-selectors.test.ts`

**max_files_changed:** `2`
**max_added_loc:** `120`
**max_deleted_loc:** `40`

**Acceptance criteria:**

- Opening the Agents sidebar no longer depends on a selector that returns a fresh object for an unchanged task-store snapshot.
- `selectActiveTaskProviderBindings()` returns the same object reference when the active task and its provider summary inputs are unchanged.
- The selector still returns updated values when the active task or provider summary actually changes.

**verification_commands:**

- `bun test tests/task-store-selectors.test.ts`
- `bun test src/components/TaskContextPopover.test.tsx`
- `bun run build`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | _pending_ | Stabilize `selectActiveTaskProviderBindings()` and add a focused regression test. | _pending_ | pending |
