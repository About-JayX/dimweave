# Task Provider Binding And Codex Final-Message Repair Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the confirmed task-provider drift in the TaskPanel while separately eliminating the "Codex thinking, then disappears with no visible reply" failure mode.

**Architecture:** Treat the repair as two independent tracks because the review established two different defects. First, realign frontend task creation/edit/connect flows with the existing task-config and task-worktree contracts so UI/provider badges match persisted task truth. Second, harden Codex completion handling so dropped or empty terminal turns no longer clear transient state without leaving a visible diagnostic result.

**Tech Stack:** React 19, Zustand 5, Bun test, Rust async daemon, Tauri 2

---

## Memory

- Recent related commits:
  - `629e711e` — added the task-config contract for explicit `leadProvider` / `coderProvider` create and update paths
  - `6938ba4d` — introduced per-task git worktrees, persisted provider bindings, and task runtimes
  - `636a4107` — made task-scoped target resolution and provider summary task-local
  - `590adb4e` / `bb21affc` / `21571244` — moved routing to per-agent-id authoritative resolution and explicit-role drop behavior when `task_agents[]` exist
  - `3283dd1d` / `30b7d6fd` / `70dabf89` — hardened Codex structured output, reply-target routing, and the structured BridgeMessage contract
  - `90fa8994` — fixed prior streaming/routing interaction issues in the chat surface
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-13-task-first-sidebar-and-ui-error-log.md`
  - `docs/superpowers/plans/2026-04-14-task-agent-identity-role-broadcast.md`
  - `docs/superpowers/plans/2026-04-14-sqlite-full-migration-and-task-root.md`
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-provider-source-fix.md`
- Relevant CM / addendum references:
  - `docs/superpowers/plans/2026-04-13-task-first-sidebar-and-ui-error-log.md` Task 2 CM (`629e711e`) for the existing task-config contract
  - `docs/superpowers/plans/2026-04-14-sqlite-full-migration-and-task-root.md` Task 5 CM (`787890b2`) for the `taskWorktreeRoot` split and task-root regression coverage
  - `docs/superpowers/plans/2026-04-15-task-agent-dialog-provider-source-fix.md` Post-Release Addendum for the live-path review miss
- Lessons carried forward:
  - Do not treat `leadProvider` / `coderProvider` drift and silent Codex turn completion as one bug. The review established separate root causes.
  - `task_agents[]` remains the authoritative routing source whenever agents exist. Singleton provider fields are compatibility and display state unless a task has no agents.
  - User-visible regressions must be validated on the full live path, not only through leaf component tests.
  - A transient stream indicator is not an acceptable terminal state. A completed turn must leave either a durable routed message or a durable diagnostic.

## Scope Notes

- This plan does not redesign multi-agent prompts or the `task_agents[]` routing model.
- This plan does not relax the explicit-role drop behavior for tasks with authoritative `task_agents[]`.
- This plan does not change unrelated stream styling; it only removes the silent-failure path.

## Task 1: Realign task config bindings and connect cwd in TaskPanel

**task_id:** `task-panel-provider-binding-and-worktree-alignment`

**Acceptance criteria:**

- Create-mode task submission persists explicit `leadProvider` / `coderProvider` using the existing task-config contract instead of relying on default singleton values.
- Edit-mode task submission updates persisted singleton provider bindings to match the current `lead` and `coder` agent assignments without disturbing additional non-lead/non-coder roles.
- `selectActiveTaskProviderBindings()` prefers `providerSummary.leadProvider` / `providerSummary.coderProvider` when a summary exists, falling back to task singleton fields only when the summary is absent.
- `Create & Connect` and `Save & Connect` launch providers from `task.taskWorktreeRoot`, not `selectedWorkspace` or `task.projectRoot`.
- Regression tests cover create/edit binding sync, selector precedence, and task-worktree launch cwd.

**allowed_files:**

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/index.test.tsx`
- `src/stores/task-store/selectors.ts`
- `tests/task-store-selectors.test.ts`

**max_files_changed:** `4`
**max_added_loc:** `260`
**max_deleted_loc:** `80`

**verification_commands:**

- `bun test tests/task-store-selectors.test.ts src/components/TaskPanel/index.test.tsx`
- `bun run build`
- `git diff --check`

## Plan Revision 1 — 2026-04-16

**Reason:** Task 1 implementation stayed within the approved 4-file write scope and acceptance criteria, but the required regression coverage for create/edit binding sync, selector precedence, and task-worktree launch cwd pushed the additive diff above the original `max_added_loc=260` budget. The user approved a budget-only revision with no scope expansion.

**Revised Task 1 budgets:**

- `max_added_loc: 440`
- `max_deleted_loc: 80` (unchanged)

## Task 2: Remove silent Codex turn completion and harden reply outcome reporting

**task_id:** `codex-final-message-visibility-hardening`

**Acceptance criteria:**

- The Codex `reply()` tool no longer reports unconditional success; it must distinguish delivered, buffered, and dropped routing outcomes so the model is not told a dropped reply was sent.
- A non-empty Codex terminal message that is dropped by task-scoped routing produces a task-scoped visible diagnostic and a precise system log, without leaking the dropped internal content into chat.
- A Codex turn that showed transient activity/reasoning/delta but produced no durable terminal output leaves a visible fallback diagnostic instead of clearing the indicator and disappearing silently.
- Existing mixed-provider task routing behavior remains intact for the verified `Codex lead / Claude coder` topology.
- Regression tests cover dropped reply acknowledgement, no-final-message fallback, and the mixed-provider role-routing guard.

**allowed_files:**

- `src-tauri/src/daemon/codex/handler.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/codex/structured_output.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`
- `src-tauri/src/daemon/routing_display.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`

**max_files_changed:** `7`
**max_added_loc:** `420`
**max_deleted_loc:** `120`

**verification_commands:**

- `cargo check --manifest-path src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::codex::handler::tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::codex::session_event::tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing::shared_role_tests:: -- --nocapture`
- `git diff --check`

## Task 3: Final regression and plan close-out

**task_id:** `provider-binding-and-codex-final-message-closeout`

**Acceptance criteria:**

- The CM record captures the real Task 1 and Task 2 commits and verification evidence.
- The plan addendum records that the frontend provider drift and the silent Codex turn path were fixed as separate issues, plus any remaining live-capture unknowns if they still exist after review.
- Final targeted regressions rerun one frontend suite and one backend suite against the integrated change set before close-out.

**allowed_files:**

- `docs/superpowers/plans/2026-04-16-task-provider-binding-and-codex-final-message-fix.md`
- `src/components/TaskPanel/index.test.tsx`
- `src-tauri/src/daemon/codex/session_event.rs`

**max_files_changed:** `3`
**max_added_loc:** `60`
**max_deleted_loc:** `20`

**verification_commands:**

- `bun test src/components/TaskPanel/index.test.tsx`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::codex::session_event::tests -- --nocapture`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `358813ca` | Realigned the TaskPanel live path with the existing task-config contract by deriving `leadProvider` / `coderProvider` from the current agent list during create and edit flows, preferring provider-summary bindings in the selector, and launching connect flows from `task.taskWorktreeRoot` instead of `selectedWorkspace` / `task.projectRoot`. Added focused regression coverage for selector precedence, create/edit binding sync, and task-worktree cwd selection. | `bun test tests/task-store-selectors.test.ts src/components/TaskPanel/index.test.tsx` ✅ 13 passed; `bun run build` ✅; `git diff --check` ✅ | accepted |
