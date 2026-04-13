# Task-Scoped Runtime and Workspace Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver real multi-task execution by making each task own an isolated workspace, a task-local Claude runtime, a task-local Codex runtime, persisted task-local provider bindings, and task-scoped routing that no longer depends on global provider state.

**Architecture:** The implementation proceeds in six stages. First lock the task graph and workspace boundary so each task is a real isolation unit. Then move Claude and Codex runtime state into task-local provider slots, with Codex gaining a launch-id-aware dynamic port pool. After that, rewrite routing, status, and buffered delivery to resolve through `task_id + task-local provider binding`, update the frontend to treat `activeTaskId` as view-only focus, and finally remove or clearly quarantine the remaining singleton compatibility shims.

**Tech Stack:** Rust, tokio, Tauri daemon, React, Zustand, Git worktrees, Claude SDK, Codex app-server, Cargo, Bun.

---

## Revision History

- `f829c414` introduced the first runtime-redesign spec/plan.
- `8cb38d7e` revised the design around per-task workspaces.
- This revision tightens the model again after review: every task must own its own Claude/Codex agent state, provider ownership moves onto the task graph, and all task scopes are now exact instead of conditional or open-ended.

## Baseline Evidence

- Current task graph gap:
  - `src-tauri/src/daemon/task_graph/types.rs` stores `workspace_root`, `lead_session_id`, and `current_coder_session_id`, but it does not persist `lead_provider` / `coder_provider`.
- Current shared-workspace gap:
  - `src-tauri/src/daemon/state_snapshot.rs::create_and_select_task()` stores the caller-provided workspace directly and does not provision a task worktree.
- Current singleton runtime gap:
  - `src-tauri/src/daemon/state.rs` still owns singleton `claude_sdk_ws_tx`, `claude_sdk_event_tx`, `claude_sdk_ready_tx`, `codex_inject_tx`, `claude_connection`, `codex_connection`, `claude_role`, and `codex_role`.
- Current global-attachment gap:
  - Claude attachment is still global in `src-tauri/src/daemon/claude_sdk/runtime.rs` and `src-tauri/src/daemon/control/claude_sdk_handler.rs`.
  - Codex attachment is still global in `src-tauri/src/daemon/codex/mod.rs`, `src-tauri/src/daemon/codex/runtime.rs`, and `src-tauri/src/daemon/codex/session.rs`.
- Current routing/status gap:
  - `src-tauri/src/daemon/routing.rs`, `state_task_flow.rs`, `state_delivery.rs`, `control/handler.rs`, `codex/session_event.rs`, `codex/handler.rs`, and `control/claude_sdk_handler_processing.rs` still read global role or active-task state along correctness paths.
- Current Codex transport gap:
  - `src-tauri/src/daemon/ports.rs` still exposes a single global Codex port.

### Baseline verification on `2026-04-13` in `/Users/jason/floder/agent-bridge/.worktrees/task-scoped-runtime-redesign`

- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot -- --nocapture` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml commands_task -- --nocapture` passed.
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture` failed before implementation on `online_role_conflict_only_blocks_live_other_agent`.
- Root cause: pre-existing mismatch between `online_role_conflict()` in `src-tauri/src/daemon/state_delivery.rs` and `is_agent_online("claude")` semantics in `src-tauri/src/daemon/state_runtime.rs`. This failure is outside Task 1 scope and must not remain on Task 1's acceptance path.

## Design Locks

- No task may share live Claude runtime state with another task.
- No task may share live Codex runtime state with another task.
- No provider launch/resume path may infer task ownership from `active_task_id`.
- No provider launch command may accept free-form `cwd` as the source of truth after task workspaces exist; `task_id` must be the authority.
- No routing path may use global `claude_role` / `codex_role` when `BridgeMessage.task_id` is present.
- `claude_role` / `codex_role` may remain only as compatibility mirrors until Task 6, never as business-semantic ownership.

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
- `8cb38d7e` — revise redesign docs for per-task workspaces
- `85baf85b` — widen Task 1 scope to include daemon command boundary

### Relevant prior plans

- `docs/superpowers/plans/2026-04-06-task-binding-routing-remediation.md`
- `docs/superpowers/plans/2026-04-07-reply-target-terminal-exit-polish.md`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`

### Lessons carried forward

- `active_task_id` is too fragile to remain on the correctness path.
- Workspace ownership is part of runtime isolation, not a separate concern.
- Codex multi-instance work must be launch-id-aware from day one.
- Any remaining global provider snapshot must be clearly labeled compatibility-only.

## File Map

### Task graph and workspace foundation

- Create: `src-tauri/src/daemon/task_workspace.rs`
  - validate git root
  - create `.worktrees/tasks`
  - derive branch/worktree names from `task_id`
  - provision task worktree
- Create: `src-tauri/src/daemon/task_runtime.rs`
  - define `TaskRuntime`
  - define provider-local runtime slot structs
- Modify: `src-tauri/src/daemon/task_graph/types.rs`
  - persist `lead_provider` / `coder_provider`
- Modify: `src-tauri/src/daemon/task_graph/store.rs`
  - initialize provider bindings during task creation
- Modify: `src-tauri/src/daemon/state.rs`
  - add `task_runtimes`
- Modify: `src-tauri/src/daemon/state_snapshot.rs`
  - create task worktree before task is returned
- Modify: `src-tauri/src/daemon/cmd.rs`
  - create-task reply becomes fallible
- Modify: `src-tauri/src/daemon/mod.rs`
  - propagate task-creation errors
- Modify: `src-tauri/src/commands_task.rs`
  - surface create-task failure cleanly

### Claude task-local runtime

- Modify: `src-tauri/src/daemon/state_runtime.rs`
  - move Claude runtime fields behind task-local accessors
- Modify: `src-tauri/src/daemon/claude_sdk/runtime.rs`
  - launch against explicit `task_id`
- Modify: `src-tauri/src/daemon/claude_sdk/mod.rs`
  - reconnect/teardown through task-local slot state
- Modify: `src-tauri/src/daemon/claude_sdk/reconnect.rs`
  - stale reconnect cleanup becomes task-local
- Modify: `src-tauri/src/daemon/control/claude_sdk_handler.rs`
  - WS attach/detach resolves against the launching task slot
- Modify: `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
  - event dispatch resolves task-local role binding, not global role
- Modify: `src-tauri/src/daemon/provider/claude.rs`
  - registration/binding becomes task-bound

### Codex task-local runtime and port pool

- Create: `src-tauri/src/daemon/codex/port_pool.rs`
  - allocator, lease, cooldown state
- Modify: `src-tauri/src/daemon/ports.rs`
  - evolve from single `codex` port to pool config
- Modify: `src-tauri/src/daemon/state_runtime.rs`
  - move Codex runtime fields behind task-local accessors
- Modify: `src-tauri/src/daemon/codex/mod.rs`
  - attach to task-local slot and lease
- Modify: `src-tauri/src/daemon/codex/runtime.rs`
  - cleanup path respects task-local slot and lease ownership
- Modify: `src-tauri/src/daemon/codex/session.rs`
  - handshake completion and failure are task/launch scoped
- Modify: `src-tauri/src/daemon/codex/lifecycle.rs`
  - orphan cleanup remains fallback only
- Modify: `src-tauri/src/daemon/provider/codex.rs`
  - registration/binding becomes task-bound

### Routing, delivery, and status contracts

- Modify: `src-tauri/src/daemon/routing.rs`
- Modify: `src-tauri/src/daemon/routing_display.rs`
- Modify: `src-tauri/src/daemon/routing_user_input.rs`
- Modify: `src-tauri/src/daemon/routing_target_session.rs`
- Modify: `src-tauri/src/daemon/state_task_flow.rs`
- Modify: `src-tauri/src/daemon/state_delivery.rs`
- Modify: `src-tauri/src/daemon/control/handler.rs`
- Modify: `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
- Modify: `src-tauri/src/daemon/codex/handler.rs`
- Modify: `src-tauri/src/daemon/codex/session_event.rs`
- Modify: `src-tauri/src/daemon/state_snapshot.rs`
- Modify: `src-tauri/src/daemon/gui_task.rs`
- Modify: `src-tauri/src/daemon/types.rs`
- Modify: `src-tauri/src/daemon/types_dto.rs`

### Frontend task-scoped UI/state

- Modify: `src/stores/task-store/types.ts`
- Modify: `src/stores/task-store/index.ts`
- Modify: `src/stores/task-store/events.ts`
- Modify: `src/stores/bridge-store/types.ts`
- Modify: `src/stores/bridge-store/index.ts`
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/bridge-store/selectors.ts`
- Modify: `src/stores/bridge-store/sync.ts`
- Modify: `src/components/workspace-entry-state.ts`
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/ReplyInput/index.tsx`
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/TaskPanel/index.tsx`
- Modify: `src/components/TaskPanel/TaskHeader.tsx`
- Modify: `src/components/TaskPanel/view-model.ts`
- Modify: `src/components/AgentStatus/index.tsx`
- Modify: `src/components/AgentStatus/provider-session-view-model.ts`

## Port-Race Constraint

This plan must not introduce Codex port races.

**Required invariant:** a Codex port may only move through these states under one serialized allocator:

`Free -> Reserved(task_id, role, launch_id) -> Live(task_id, role, launch_id) -> CoolingDown(until_ms) -> Free`

Rules:

- no launch may pick a port without first creating a `Reserved` lease
- no stale handshake-success callback may promote a lease unless `launch_id` still matches
- no stale stop/failure callback may release a lease unless `launch_id` still matches
- every released port must enter cooldown before reuse
- `lsof` cleanup remains defensive fallback only and is not the allocator

## CM Memory

| Task | Commit | Summary | Verification | Status |
|------|--------|---------|--------------|--------|
| Task 1 | `6938ba4d` | Add per-task git worktree provisioning, concrete Claude/Codex task bindings, `TaskRuntime` initialization, and rollback for failed task creation | `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture` ✅ 26 passed; `cargo test --manifest-path src-tauri/Cargo.toml task_workspace -- --nocapture` ✅ 5 passed; `cargo test --manifest-path src-tauri/Cargo.toml task_runtime -- --nocapture` ✅ 4 passed; `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot -- --nocapture` ✅ 10 passed; `cargo test --manifest-path src-tauri/Cargo.toml commands_task -- --nocapture` ✅ 0 tests / wrapper path; `git diff --check` ✅ | accepted |
| Task 2 | `e3497bf6` | Move Claude SDK lifecycle state into `ClaudeTaskSlot`, thread explicit `task_id` through launch/reconnect/sync paths, and add focused `claude_task_slot` coverage. **Accepted: `e3497bf6`**, follow-up **`e708538f`** fixes cross-task nonce/event/invalidation isolation, follow-up **`b3ca85f4`** fixes task-graph binding isolation during task-local invalidation. | `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::claude_tests:: -- --nocapture` ✅ 17 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::control::claude_sdk_handler::tests:: -- --nocapture` ✅ 9 passed; `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk:: -- --nocapture` ✅ 38 passed; `cargo test --manifest-path src-tauri/Cargo.toml claude_task_slot -- --nocapture` ✅ 12 passed; `git diff --check` ✅ | accepted |
| Task 3 | `5f8773fa` | Move Codex runtime state into task-local slots and add a reservation-aware port pool. **Accepted: `5f8773fa`**, follow-up **`e93cd0b5`** fixes task-graph binding isolation for clear paths, follow-up **`f4225782`** threads explicit `task_id` through resume sync and stores task-local connection ownership, follow-up **`4d5fab2a`** replaces singleton `CodexHandle` with task-keyed handle ownership and adds reservation tracking, follow-up **`f269bdb6`** wires `reserve -> promote -> release` and natural-exit notifications, follow-up **`5aa7b7fc`** makes lease ownership launch-id-aware and extracts focused port-pool tests. | `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture` ✅ 18 passed; `cargo test --manifest-path src-tauri/Cargo.toml codex_task_slot -- --nocapture` ✅ 10 passed; `cargo test --manifest-path src-tauri/Cargo.toml codex_port_pool -- --nocapture` ✅ 15 passed; `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture` ✅ 50 passed; `git diff --check` ✅ | accepted |
| Task 4 | `14dd2b70` | Make routing and provider-originated message stamping task-scoped. **Accepted: `14dd2b70`**, follow-up **`0a661996`** fixes task-local channel delivery, threads explicit `task_id` through Codex/Claude provider event workers, and makes task-scoped status derived from task-local runtime slots. | `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture` ✅ 19 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_shared_role_tests:: -- --nocapture` ✅ 0 matched; `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_user_target_tests:: -- --nocapture` ✅ 0 matched; `cargo test --manifest-path src-tauri/Cargo.toml task_runtime_routing -- --nocapture` ✅ 7 passed; `cargo test --manifest-path src-tauri/Cargo.toml types_tests -- --nocapture` ✅ 0 matched; `git diff --check` ✅ | accepted |
| Task 5 | `pending_commit` | Update frontend launch/status/message flows to true task scope | Use Task 5 verification commands | planned |
| Task 6 | `pending_commit` | Remove/quarantine singleton shims and document the final model | Use Task 6 verification commands | planned |

---

### Task 1: Lock task graph/provider bindings and task workspace provisioning

**task_id:** `task-runtime-foundation`

**Acceptance criteria:**

- Creating a task provisions a dedicated task worktree at `.worktrees/tasks/<task_id>`.
- Task creation fails cleanly for non-git workspace roots, returns the error through the daemon command boundary, and does not leave a partially-created task behind.
- `Task.workspace_root` stores the task worktree path, not the shared repo root.
- `Task` persists `lead_provider` and `coder_provider`.
- `DaemonState` gains a `task_runtimes` registry with one initialized task runtime per task.

**allowed_files:**

- `src-tauri/src/daemon/task_workspace.rs`
- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/task_graph/types.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/state_snapshot_tests.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/commands_task.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/commands_artifact.rs`
- `src-tauri/src/daemon/gui_task.rs`

**max_files_changed:** `15`
**max_added_loc:** `520`
**max_deleted_loc:** `180`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml task_workspace -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml task_runtime -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml commands_task -- --nocapture`
- `git diff --check`

## Plan Revision 1 — 2026-04-13

**Reason:** Task 1 needed two extra compile-fix test files that construct `Task` directly after concrete `lead_provider` / `coder_provider` fields were introduced. Baseline verification also showed `daemon::state::state_tests::` contains a pre-existing unrelated failure, so Task 1 verification must stay focused on the task-specific suites.

**Added to Task 1 allowed_files:**

- `src-tauri/src/commands_artifact.rs`
- `src-tauri/src/daemon/gui_task.rs`

**Revised Task 1 budgets:**

- `max_files_changed: 15`
- `max_added_loc: 520`
- `max_deleted_loc: 180`

## Plan Revision 2 — 2026-04-13

**Reason:** Baseline verification proved `daemon::state::state_tests::` contains a pre-existing unrelated failure (`online_role_conflict_only_blocks_live_other_agent`). Task 2 and Task 3 therefore use focused new test prefixes instead of the entire `state_tests` module, so acceptance stays on task-local Claude/Codex runtime behavior only.

**Revised verification focus for Task 2 / Task 3:**

- Task 2 must add and run focused tests matched by `claude_task_slot`.
- Task 3 must add and run focused tests matched by `codex_task_slot` and `codex_port_pool`.

## Plan Revision 3 — 2026-04-13

**Reason:** Task 2 acceptance criterion 2 requires Claude live state to move into a task-local Claude slot. That slot belongs in `src-tauri/src/daemon/task_runtime.rs`, which was omitted from the original Task 2 scope.

**Added to Task 2 allowed_files:**

- `src-tauri/src/daemon/task_runtime.rs`

**Revised Task 2 budgets:**

- `max_files_changed: 15`
- `max_added_loc: 580`
- `max_deleted_loc: 220`

## Plan Revision 4 — 2026-04-13

**Reason:** Task 3 acceptance criterion 2 requires Codex live state to move into a task-local Codex slot. That slot belongs in `src-tauri/src/daemon/task_runtime.rs`, which was omitted from the original Task 3 scope.

**Added to Task 3 allowed_files:**

- `src-tauri/src/daemon/task_runtime.rs`

**Revised Task 3 budgets:**

- `max_files_changed: 14`
- `max_added_loc: 620`
- `max_deleted_loc: 220`

## Plan Revision 5 — 2026-04-13

**Reason:** Task 3 changed `src-tauri/src/daemon/launch_task_sync.rs` to keep the Codex launch registration path compiling after `provider::codex::register_on_launch` switched to explicit `task_id`. That file is a direct part of the approved launch binding path and must be in Task 3 scope.

**Added to Task 3 allowed_files:**

- `src-tauri/src/daemon/launch_task_sync.rs`

**Revised Task 3 budgets:**

- `max_files_changed: 15`
- `max_added_loc: 620`
- `max_deleted_loc: 220`

## Plan Revision 6 — 2026-04-13

**Reason:** Task 3 extracted `src-tauri/src/daemon/codex/port_pool_tests.rs` from `port_pool.rs` to keep the allocator module within file-size limits while adding the required launch-id-aware lease regressions. The cumulative accepted implementation also exceeded the original `max_added_loc` once the reservation tracker, task-keyed handle map, exit-notice plumbing, and extracted tests were all included.

**Added to Task 3 allowed_files:**

- `src-tauri/src/daemon/codex/port_pool_tests.rs`

**Revised Task 3 budgets:**

- `max_files_changed: 16`
- `max_added_loc: 1120`
- `max_deleted_loc: 220`

---

### Task 2: Move Claude runtime state into task-local slots

**task_id:** `task-runtime-claude-slot`

**Acceptance criteria:**

- Claude launch/resume/reconnect surfaces carry explicit `task_id`.
- Claude live state (`ready_tx`, `event_tx`, nonce, generation, direct-text, preview buffer) is stored in the task-local Claude slot, not singleton daemon fields.
- Claude WS attach/detach can only mutate the matching task slot.
- One task's Claude reconnect/disconnect cannot clear another task's Claude runtime.
- Claude session registration binds against the requested task instead of `active_task_id`.

**allowed_files:**

- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/launch_task_sync.rs`
- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/claude_sdk/runtime.rs`
- `src-tauri/src/daemon/claude_sdk/mod.rs`
- `src-tauri/src/daemon/claude_sdk/reconnect.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
- `src-tauri/src/daemon/provider/claude.rs`
- `src-tauri/src/daemon/provider/claude_tests.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_tests.rs`
- `src-tauri/src/daemon/state_tests.rs`

**max_files_changed:** `15`
**max_added_loc:** `580`
**max_deleted_loc:** `220`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::claude_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::control::claude_sdk_handler::tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml claude_task_slot -- --nocapture`
- `git diff --check`

---

### Task 3: Move Codex runtime state into task-local slots and add port pool

**task_id:** `task-runtime-codex-slot-and-port-pool`

**Acceptance criteria:**

- Codex launch/resume surfaces carry explicit `task_id`.
- Codex live send channel/connection ownership is stored in the task-local Codex slot, not a singleton daemon field.
- Codex launches reserve ports through a central allocator before spawn.
- Stale handshake-success/failure/stop callbacks cannot steal or release another task's port lease.
- Stopping task A Codex does not affect task B Codex.

**allowed_files:**

- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/launch_task_sync.rs`
- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/ports.rs`
- `src-tauri/src/daemon/codex/port_pool.rs`
- `src-tauri/src/daemon/codex/port_pool_tests.rs`
- `src-tauri/src/daemon/codex/mod.rs`
- `src-tauri/src/daemon/codex/runtime.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/codex/lifecycle.rs`
- `src-tauri/src/daemon/provider/codex.rs`
- `src-tauri/src/daemon/provider/codex_tests.rs`
- `src-tauri/src/daemon/state_tests.rs`

**max_files_changed:** `16`
**max_added_loc:** `1120`
**max_deleted_loc:** `220`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex_task_slot -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex_port_pool -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture`
- `git diff --check`

---

### Task 4: Route, buffer, and expose status by `task_id` and task-local bindings

**task_id:** `task-runtime-routing-and-status`

**Acceptance criteria:**

- `BridgeMessage.task_id` is the primary routing key.
- Routing resolves target provider through the task's persisted `lead_provider` / `coder_provider`, not global `claude_role` / `codex_role`.
- Provider-originated messages are stamped against their owning task runtime, not `active_task_id`.
- Buffered messages are isolated per task.
- Task context events and snapshots expose per-task provider binding/runtime summaries.
- `check_messages` and `get_status` return task-correct views when another task is active.

**allowed_files:**

- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/state_snapshot_tests.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_display.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
- `src-tauri/src/daemon/claude_sdk/runtime.rs`
- `src-tauri/src/daemon/codex/handler.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/state_tests.rs`

**max_files_changed:** `23`
**max_added_loc:** `900`
**max_deleted_loc:** `300`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_shared_role_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_user_target_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml task_runtime_routing -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml types_tests -- --nocapture`
- `git diff --check`

## Plan Revision 7 — 2026-04-13

**Reason:** Baseline verification already proved `daemon::state::state_tests::` contains a pre-existing unrelated failure (`online_role_conflict_only_blocks_live_other_agent`). Task 4 therefore needs a focused state-side regression prefix instead of the entire `state_tests` module.

**Revised verification focus for Task 4:**

- Task 4 must add and run focused tests matched by `task_runtime_routing`.

## Plan Revision 8 — 2026-04-13

**Reason:** Lead review found Task 4 cannot satisfy provider-originated message ownership with the original file set. Codex tool/session events need explicit task context threaded from `src-tauri/src/daemon/codex/session.rs`, and Claude SDK event processing needs explicit task context threaded from `src-tauri/src/daemon/claude_sdk/runtime.rs`.

**Added to Task 4 allowed_files:**

- `src-tauri/src/daemon/claude_sdk/runtime.rs`
- `src-tauri/src/daemon/codex/session.rs`

**Revised Task 4 budgets:**

- `max_files_changed: 23`
- `max_added_loc: 900`
- `max_deleted_loc: 300`

---

### Task 5: Update frontend launch/status/message flows to true task scope

**task_id:** `task-runtime-frontend-task-scope`

**Acceptance criteria:**

- Task store carries task-local provider bindings and runtime summaries.
- Frontend launch/send operations pass explicit `taskId`, not raw workspace ownership assumptions.
- Workspace entry flow reflects dedicated task workspace semantics.
- Message panel renders only the active task's messages while keeping background tasks alive.
- Task panel and agent status panel show per-task Claude/Codex runtime state instead of treating one global provider pair as authoritative.

**allowed_files:**

- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/commands_artifact.rs`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/events.ts`
- `src/stores/bridge-store/types.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/stores/bridge-store/listener-setup.test.ts`
- `src/stores/bridge-store/selectors.ts`
- `src/stores/bridge-store/sync.ts`
- `src/components/workspace-entry-state.ts`
- `src/components/WorkspaceEntryOverlay.tsx`
- `src/components/WorkspaceEntryOverlay.test.tsx`
- `src/components/ClaudePanel/index.tsx`
- `src/components/ReplyInput/index.tsx`
- `src/components/MessagePanel/index.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/view-model.ts`
- `src/components/AgentStatus/index.tsx`
- `src/components/AgentStatus/provider-session-view-model.ts`
- `tests/task-store.test.ts`
- `tests/task-panel-view-model.test.ts`
- `src/components/ReplyInput/index.test.tsx`
- `src/components/MessagePanel/index.test.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/TaskPanel/ArtifactTimeline.test.tsx`
- `src/components/ClaudePanel/connect-state.test.ts`
- `src/components/ClaudePanel/launch-request.test.ts`
- `src/components/AgentStatus/codex-launch-config.test.ts`

**max_files_changed:** `34`
**max_added_loc:** `920`
**max_deleted_loc:** `300`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `bun test tests/task-store.test.ts tests/task-panel-view-model.test.ts`
- `bun test src/components/ReplyInput/index.test.tsx src/components/MessagePanel/index.test.tsx`
- `bun test src/components/TaskPanel/TaskHeader.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx`
- `bun test src/components/ClaudePanel/connect-state.test.ts src/components/ClaudePanel/launch-request.test.ts src/components/AgentStatus/codex-launch-config.test.ts`
- `bun run build`
- `git diff --check`

## Plan Revision 9 — 2026-04-13

**Reason:** Lead review found Task 5 could not satisfy two approved acceptance criteria with frontend-only scope:

- explicit `taskId` on user send operations requires the daemon send command surface (`commands.rs`, `cmd.rs`, `mod.rs`, `routing_user_input.rs`)
- task-local runtime summaries in the task store require backend snapshot DTO wiring (`state_snapshot.rs`, `types_dto.rs`)

Task 5 also needs targeted Rust verification for those contract changes.

**Added to Task 5 allowed_files:**

- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/commands_artifact.rs`

**Revised Task 5 budgets:**

- `max_files_changed: 34`
- `max_added_loc: 920`
- `max_deleted_loc: 300`

**Revised Task 5 verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`

## Plan Revision 10 — 2026-04-13

**Reason:** Lead review found Task 5 still needed to align the workspace-entry copy with the approved “dedicated task workspace” product semantics. That requires the overlay component and its test file.

**Added to Task 5 allowed_files:**

- `src/components/WorkspaceEntryOverlay.tsx`
- `src/components/WorkspaceEntryOverlay.test.tsx`

## Plan Revision 11 — 2026-04-13

**Reason:** Task 5 needed `src-tauri/src/commands_artifact.rs` for a compile-fix update after `TaskSnapshot` gained `provider_summary`. This is a mechanical test-only ripple from the approved DTO contract change.

**Added to Task 5 allowed_files:**

- `src-tauri/src/commands_artifact.rs`

---

### Task 6: Remove or quarantine singleton shims and document the final model

**task_id:** `task-runtime-final-cleanup`

**Acceptance criteria:**

- Remaining singleton fields are either removed or clearly marked compatibility-only.
- Global `claude_role` / `codex_role` no longer act as business-semantic ownership.
- Docs record the final task-local Claude/Codex ownership model and allocator invariants.
- Full daemon/provider/routing/frontend verification passes without new regressions.

**allowed_files:**

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/state_snapshot_tests.rs`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`
- `docs/superpowers/specs/2026-04-13-task-scoped-runtime-redesign-design.md`
- `docs/superpowers/plans/2026-04-13-task-scoped-runtime-redesign.md`

**max_files_changed:** `12`
**max_added_loc:** `260`
**max_deleted_loc:** `220`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk:: -- --nocapture`
- `bun run build`
- `git diff --check`

## Rollout Notes

- Task 1 is mandatory. Without persisted provider bindings and a task worktree, nothing else is a real isolation boundary.
- Task 2 and Task 3 are sequential, not parallel. They both touch shared runtime plumbing and must land in order.
- Task 4 cannot start before Task 2 and Task 3 land; otherwise routing will target task-local models that do not exist yet.
- Task 5 cannot start before Task 4 lands; otherwise the UI will claim task isolation while backend ownership remains global.
- Task 6 must not add new behavior. It is a cleanup and regression barrier only.
