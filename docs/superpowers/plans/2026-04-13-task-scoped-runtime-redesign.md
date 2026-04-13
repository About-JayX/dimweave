# Task-Scoped Runtime and Workspace Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make each task a full execution-isolation unit by provisioning a dedicated workspace per task, binding lead/coder providers per task, routing by `task_id`, and eliminating Codex port races through a centralized dynamic port pool.

**Architecture:** The implementation proceeds in phases. First add task workspace provisioning and a task runtime registry without changing behavior. Then move provider launch/binding onto explicit `task_id`s, route delivery by task runtime instead of UI focus, add a race-free Codex port allocator for app-server instances, and finally make the frontend treat `active_task_id` as a pure view selector over already-running background tasks.

**Tech Stack:** Rust, tokio, Tauri daemon, React, Zustand, Git worktrees, Codex app-server, Claude SDK, Cargo, Bun.

---

## Baseline Evidence

- Current singleton runtime proof:
  - `src-tauri/src/daemon/state.rs` still owns global `claude_sdk_ws_tx`, `codex_inject_tx`, connection state, roles, epochs, preview state.
  - `src-tauri/src/daemon/routing.rs` reads those global fields directly to deliver messages.
  - `src-tauri/src/daemon/provider/claude.rs` / `provider/codex.rs` bind launches via `state.active_task_id`.
- Current shared-workspace proof:
  - `src-tauri/src/daemon/state_snapshot.rs::create_and_select_task()` stores the caller-provided workspace directly; it does not provision a dedicated task workspace.
- Codex transport proof:
  - `src-tauri/src/daemon/codex/lifecycle.rs` launches `codex app-server --listen ws://127.0.0.1:{port}`.
  - `src-tauri/src/daemon/ports.rs` currently exposes only a single global Codex port.
- Frontend proof:
  - `src/stores/task-store/index.ts` already has task-scoped task/session history state.
  - `src/stores/bridge-store/listener-setup.ts` still appends all messages into one global timeline, leaving task filtering to the view layer.

## Project Memory

### Recent related commits

- `577751e7` — task/session foundation
- `d676d2f4` — task session history workspace
- `c403ad69` — Codex provider history/resume
- `0b1c29c6` — active-task routing and Claude launch binding remediation
- `fa688747` — task-scoped workspace routing boundaries
- `dda01a6c` — reply target remembered per task
- `68d0ffd6` — Codex lifecycle stabilization
- `46488de7` — centralized daemon/Codex ports

### Relevant prior plans

- `docs/superpowers/plans/2026-04-06-task-binding-routing-remediation.md`
- `docs/superpowers/plans/2026-04-07-reply-target-terminal-exit-polish.md`
- `docs/superpowers/plans/2026-04-13-telegram-route-loop-fix.md`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`

### Lessons carried forward

- `active_task_id` must leave the correctness path; it is too fragile as a binding source.
- Workspace ownership and reply-target memory have already moved to task scope; runtime ownership must align.
- Codex port management cannot rely on single-port cleanup heuristics once concurrency is introduced.
- A task without an isolated workspace is not a real isolation boundary.

## File Map

### Task workspace provisioning

- Create: `src-tauri/src/daemon/task_workspace.rs`
  - validate git root
  - create `.worktrees/tasks`
  - derive branch/worktree names from `task_id`
  - provision task worktree
- Modify: `src-tauri/src/daemon/state_snapshot.rs`
  - `create_and_select_task()` provisions workspace before task becomes active
- Modify: `src-tauri/src/commands_task.rs`
  - task creation semantics/documentation align with task-scoped workspace
- Modify: `src/components/workspace-entry-state.ts`
  - frontend wording/flow stays aligned with “start a new task workspace”

### Task runtime registry

- Create: `src-tauri/src/daemon/task_runtime.rs`
  - `TaskRuntime`, `RuntimeSlot`, provider runtime metadata
- Modify: `src-tauri/src/daemon/state.rs`
  - add `task_runtimes`
- Modify: `src-tauri/src/daemon/state_runtime.rs`
  - move runtime helpers behind task-scoped accessors
- Modify: `src-tauri/src/daemon/state_delivery.rs`
  - task-scoped buffered messages
- Modify: `src-tauri/src/daemon/state_task_flow.rs`
  - task-scoped routing/session matching

### Provider lifecycle and routing

- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/launch_task_sync.rs`
- Modify: `src-tauri/src/daemon/provider/claude.rs`
- Modify: `src-tauri/src/daemon/provider/codex.rs`
- Modify: `src-tauri/src/daemon/routing.rs`
- Modify: `src-tauri/src/daemon/routing_user_input.rs`
- Modify: `src-tauri/src/daemon/routing_target_session.rs`

### Codex port pool

- Create: `src-tauri/src/daemon/codex/port_pool.rs`
- Modify: `src-tauri/src/daemon/ports.rs`
- Modify: `src-tauri/src/daemon/codex/mod.rs`
- Modify: `src-tauri/src/daemon/codex/runtime.rs`
- Modify: `src-tauri/src/daemon/codex/lifecycle.rs`

### Frontend task-scoped runtime display

- Modify: `src/stores/task-store/index.ts`
- Modify: `src/stores/bridge-store/index.ts`
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/ReplyInput/index.tsx`
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/TaskPanel/index.tsx`
- Modify: `src/components/TaskPanel/TaskHeader.tsx`
- Modify: `src/components/AgentStatus/index.tsx`

### Tests / docs

- Modify: `src-tauri/src/daemon/state_tests.rs`
- Modify: `src-tauri/src/daemon/provider/claude_tests.rs`
- Modify: `src-tauri/src/daemon/provider/codex_tests.rs`
- Modify: `src-tauri/src/daemon/routing_shared_role_tests.rs`
- Modify: `src-tauri/src/daemon/routing_user_target_tests.rs`
- Modify: `tests/task-store.test.ts`
- Modify: `tests/task-panel-view-model.test.ts`
- Modify: `src/components/ReplyInput/index.test.tsx`
- Modify: `src/components/MessagePanel/index.test.tsx`
- Modify: `src/components/TaskPanel/TaskHeader.test.tsx`
- Modify: `src/components/TaskPanel/ArtifactTimeline.test.tsx`
- Modify: `src/components/ClaudePanel/connect-state.test.ts`
- Modify: `src/components/ClaudePanel/launch-request.test.ts`
- Modify: `src/components/AgentStatus/codex-launch-config.test.ts`

## Port-Race Constraint

This plan must not introduce Codex port races.

**Required invariant:** a Codex port may only move through these states under one serialized allocator:

`Free -> Reserved(task_id, role, launch_id) -> Live(task_id, role, launch_id) -> CoolingDown(until_ms) -> Free`

Rules:

- no launch may pick a port without first creating a `Reserved` lease
- no stale success callback may promote a lease unless `launch_id` still matches
- no stale stop/failure callback may release a lease unless `launch_id` still matches
- every released port must enter cooldown before reuse
- `lsof` cleanup remains defensive fallback only, never the allocator itself

## CM Memory

| Task | Commit | Summary | Verification | Status |
|------|--------|---------|--------------|--------|
| Task 1 | to be filled after implementation | Add task workspace provisioning and task runtime registry skeleton | Use Task 1 verification commands | planned |
| Task 2 | to be filled after implementation | Make provider lifecycle explicitly task-bound | Use Task 2 verification commands | planned |
| Task 3 | to be filled after implementation | Route and buffer by task runtime | Use Task 3 verification commands | planned |
| Task 4 | to be filled after implementation | Add race-free Codex port pool | Use Task 4 verification commands | planned |
| Task 5 | to be filled after implementation | Frontend task-scoped runtime/message isolation | Use Task 5 verification commands | planned |
| Task 6 | to be filled after implementation | Final compatibility cleanup and regression barrier | Use Task 6 verification commands | planned |

---

### Task 1: Add task workspace provisioning and runtime registry skeleton

**task_id:** `task-workspace-and-runtime-skeleton`

**Acceptance criteria:**

- Creating a task provisions a dedicated task worktree at `.worktrees/tasks/<task_id>`.
- Task creation fails cleanly for non-git workspace roots.
- `Task.workspace_root` stores the task worktree path, not the shared source root.
- `DaemonState` gains a `task_runtimes` registry and task-runtime accessors.
- Existing singleton runtime behavior still works through compatibility shims.

**allowed_files:**

- `src-tauri/src/daemon/task_workspace.rs`
- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/commands_task.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `tests/task-store.test.ts` only if command contract fallout requires frontend fixture updates

**max_files_changed:** `7`
**max_added_loc:** `360`
**max_deleted_loc:** `120`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon:: -- --nocapture task_workspace`
- `cargo test --manifest-path src-tauri/Cargo.toml commands_task -- --nocapture`
- `git diff --check`

---

### Task 2: Make provider lifecycle explicitly task-bound

**task_id:** `task-runtime-provider-binding`

**Acceptance criteria:**

- Launch/resume/send/stop surfaces carry explicit `task_id` where task ownership matters.
- `register_on_launch()` / `register_on_connect()` no longer read `active_task_id`.
- Daemon runtime handles for Claude/Codex are tracked by task.
- Launching while switching UI focus cannot bind a provider to the wrong task.

**allowed_files:**

- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/launch_task_sync.rs`
- `src-tauri/src/daemon/provider/claude.rs`
- `src-tauri/src/daemon/provider/codex.rs`
- `src-tauri/src/daemon/provider/claude_tests.rs`
- `src-tauri/src/daemon/provider/codex_tests.rs`

**max_files_changed:** `8`
**max_added_loc:** `340`
**max_deleted_loc:** `140`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::claude_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture`
- `git diff --check`

---

### Task 3: Route and buffer by task runtime instead of UI focus

**task_id:** `task-runtime-routing`

**Acceptance criteria:**

- `route_message_inner` resolves target runtime by `BridgeMessage.task_id`.
- Task-scoped buffered messages no longer share one global queue.
- `stamp_message_context()` and `preferred_auto_target()` use `active_task_id` only as UI fallback.
- Legacy messages without `task_id` still work through explicit compatibility fallback.

**allowed_files:**

- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/state_tests.rs`

**max_files_changed:** `8`
**max_added_loc:** `320`
**max_deleted_loc:** `140`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_shared_role_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_user_target_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture`
- `git diff --check`

---

### Task 4: Add a race-free Codex dynamic port pool

**task_id:** `task-runtime-codex-port-pool`

**Acceptance criteria:**

- Codex launches reserve ports through a central allocator before spawn.
- Two concurrent Codex launches cannot reserve the same port.
- Stale launch success/failure callbacks cannot steal or release another task's lease.
- Released ports enter cooldown before reuse.
- Existing orphan-process cleanup remains a defensive fallback only.

**allowed_files:**

- `src-tauri/src/daemon/ports.rs`
- `src-tauri/src/daemon/codex/port_pool.rs`
- `src-tauri/src/daemon/codex/mod.rs`
- `src-tauri/src/daemon/codex/runtime.rs`
- `src-tauri/src/daemon/codex/lifecycle.rs`
- `src-tauri/src/daemon/provider/codex.rs`
- `src-tauri/src/daemon/provider/codex_tests.rs`
- `src-tauri/src/daemon/state_tests.rs`

**max_files_changed:** `8`
**max_added_loc:** `380`
**max_deleted_loc:** `140`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture`
- `git diff --check`

---

### Task 5: Move frontend launch/send/display into task scope

**task_id:** `task-runtime-frontend-isolation`

**Acceptance criteria:**

- Frontend launches Claude/Codex with explicit `taskId`.
- ReplyInput sends explicit `taskId`.
- Task creation and workspace-entry flow reflect “new task workspace” semantics.
- Message panel renders only the active task's messages.
- Task panel and runtime surfaces show per-task provider bindings instead of one global runtime status.

**allowed_files:**

- `src/stores/task-store/index.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/components/workspace-entry-state.ts`
- `src/components/ClaudePanel/index.tsx`
- `src/components/ReplyInput/index.tsx`
- `src/components/MessagePanel/index.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/AgentStatus/index.tsx`
- `tests/task-store.test.ts`
- `tests/task-panel-view-model.test.ts`
- `src/components/ReplyInput/index.test.tsx`
- `src/components/MessagePanel/index.test.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/TaskPanel/ArtifactTimeline.test.tsx`
- `src/components/ClaudePanel/connect-state.test.ts`
- `src/components/ClaudePanel/launch-request.test.ts`
- `src/components/AgentStatus/codex-launch-config.test.ts`

**max_files_changed:** `19`
**max_added_loc:** `420`
**max_deleted_loc:** `200`

**verification_commands:**

- `bun test tests/task-store.test.ts tests/task-panel-view-model.test.ts src/components/ReplyInput/index.test.tsx src/components/MessagePanel/index.test.tsx`
- `bun test src/components/TaskPanel/TaskHeader.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx src/components/ClaudePanel/connect-state.test.ts src/components/ClaudePanel/launch-request.test.ts src/components/AgentStatus/codex-launch-config.test.ts`
- `bun run build`
- `git diff --check`

---

### Task 6: Final compatibility cleanup and regression barrier

**task_id:** `task-runtime-final-regression-barrier`

**Acceptance criteria:**

- Remaining global runtime fields are either removed or reduced to clearly marked fallback-only shims.
- Multi-task runtime/manual smoke instructions are documented.
- Docs record the final ownership model and Codex allocator invariants.
- No new regressions appear in daemon/provider/routing/build verification.

**allowed_files:**

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/mod.rs`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`
- `docs/superpowers/plans/2026-04-13-task-scoped-runtime-redesign.md`
- only directly related daemon/provider/routing test files if needed

**max_files_changed:** `7`
**max_added_loc:** `220`
**max_deleted_loc:** `180`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture`
- `bun run build`
- `git diff --check`

## Rollout Notes

- Do not skip Task 1. Without per-task workspace provisioning, runtime isolation is incomplete.
- Do not start frontend isolation before Task 2 and Task 3 land; otherwise the UI will look task-scoped while backend ownership remains global.
- Do not start Codex pool work before Task 2; the allocator needs explicit task-bound lifecycle inputs.
