# Task-Scoped Runtime Registry Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current global live lead/coder runtime model with a task-scoped runtime registry so multiple tasks can execute concurrently without routing or provider-binding races.

**Architecture:** Introduce a daemon-side `TaskRuntimeRegistry` keyed by `task_id`, move provider runtime state into per-task runtime slots, make provider launch/resume explicit about `task_id`, route by `task_id` instead of `active_task_id`, and add a race-free Codex dynamic port pool. Keep `active_task_id` only as the foreground UI focus and maintain legacy fallback only for old messages/commands that truly lack task context.

**Tech Stack:** Rust, tokio, Tauri daemon, React, Zustand, Codex app-server, Claude SDK, Cargo, Bun.

---

## Baseline Evidence

- Current runtime singleton proof:
  - `src-tauri/src/daemon/state.rs` holds global `claude_sdk_ws_tx`, `codex_inject_tx`, `claude_connection`, `codex_connection`, roles, epochs, preview state.
  - `src-tauri/src/daemon/routing.rs` reads those global fields directly to deliver messages.
  - `src-tauri/src/daemon/provider/claude.rs` and `provider/codex.rs` bind launches through `state.active_task_id`.
- Existing task/session scaffolding already present:
  - `src-tauri/src/daemon/task_graph/types.rs` has `Task`, `SessionHandle`, provider + role metadata.
  - `BridgeMessage` already carries `task_id` and `session_id`.
  - `state_delivery.rs` already has task-aware buffered-message helpers.
- Codex transport limitation:
  - `src-tauri/src/daemon/codex/lifecycle.rs` launches one `codex app-server --listen ws://127.0.0.1:{port}` per instance.
  - `src-tauri/src/daemon/ports.rs` currently exposes only a single global Codex port.

## Project Memory

### Recent related commits

- `577751e7` — task/session foundation
- `d676d2f4` — task session history workspace
- `c403ad69` — Codex provider history and resume adapter
- `0b1c29c6` — repair active-task routing and Claude launch binding
- `fa688747` — enforce task-scoped workspace routing boundaries
- `dda01a6c` — remember reply target per task
- `68d0ffd6` — Codex WS pump/session lifecycle stabilization
- `46488de7` — centralized daemon/Codex ports

### Relevant prior plans

- `docs/superpowers/plans/2026-04-06-task-binding-routing-remediation.md`
- `docs/superpowers/plans/2026-04-07-reply-target-terminal-exit-polish.md`
- `docs/superpowers/specs/2026-03-31-unified-task-session-architecture-design.md`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`

### Lessons carried forward

- Active-task-based binding is fragile even in the single-runtime design and must not remain on the correctness path.
- Reply-target and workspace ownership already moved toward task-scoped semantics; the runtime layer should follow that same boundary.
- Codex lifecycle bugs tend to appear as stale port holders and stale session completion events, so the port allocator must be launch-id aware rather than "best effort".

## File Map

### Backend runtime model

- Create: `src-tauri/src/daemon/task_runtime.rs`
  - `TaskRuntime`, `RuntimeSlot`, provider-specific runtime metadata, registry helpers
- Modify: `src-tauri/src/daemon/state.rs`
  - add `task_runtimes`, keep temporary global-field shims
- Modify: `src-tauri/src/daemon/state_runtime.rs`
  - move runtime epoch/nonce/preview helpers behind task-scoped accessors
- Modify: `src-tauri/src/daemon/state_delivery.rs`
  - per-task buffered delivery instead of one global vector
- Modify: `src-tauri/src/daemon/state_task_flow.rs`
  - task-scoped session matching, message stamping, preferred target lookup

### Backend command + provider lifecycle

- Modify: `src-tauri/src/daemon/cmd.rs`
  - add explicit `task_id` to launch/send/stop commands where required
- Modify: `src-tauri/src/commands.rs`
  - thread `task_id` through Tauri command surfaces
- Modify: `src-tauri/src/daemon/mod.rs`
  - replace single `Option<CodexHandle>` / `Option<ClaudeSdkHandle>` with task-scoped handle registries
- Modify: `src-tauri/src/daemon/launch_task_sync.rs`
  - task-scoped sync helpers
- Modify: `src-tauri/src/daemon/provider/claude.rs`
  - register/bind/resume against explicit `task_id`
- Modify: `src-tauri/src/daemon/provider/codex.rs`
  - register/bind/resume against explicit `task_id`

### Backend routing

- Modify: `src-tauri/src/daemon/routing.rs`
  - resolve target channel from task runtime slot
- Modify: `src-tauri/src/daemon/routing_user_input.rs`
  - require/stamp explicit task context from command payload
- Modify: `src-tauri/src/daemon/routing_target_session.rs`
  - resolve target session by `task_id + role`

### Codex port pool

- Create: `src-tauri/src/daemon/codex/port_pool.rs`
  - allocator, lease state, reservation/promotion/cooldown
- Modify: `src-tauri/src/daemon/ports.rs`
  - move from single `codex` port to `codex_base` + `codex_pool_size`
- Modify: `src-tauri/src/daemon/codex/mod.rs`
  - accept reserved port + launch id, route stop/release through allocator
- Modify: `src-tauri/src/daemon/codex/runtime.rs`
  - remove single-port assumption from health/availability helpers
- Modify: `src-tauri/src/daemon/codex/lifecycle.rs`
  - keep orphan cleanup as fallback only, not allocator

### Frontend task-scoped UI/state

- Modify: `src/stores/task-store/index.ts`
  - current task remains UI focus only
- Modify: `src/stores/bridge-store/index.ts`
  - task-scoped launch/send arguments
- Modify: `src/stores/bridge-store/listener-setup.ts`
  - task-aware message/runtime event handling
- Modify: `src/components/ClaudePanel/index.tsx`
  - pass explicit `taskId` on launch/resume/stop
- Modify: `src/components/AgentStatus/index.tsx`
  - task-aware runtime display
- Modify: `src/components/ReplyInput/index.tsx`
  - send explicit `taskId`
- Modify: `src/components/MessagePanel/index.tsx`
  - render only active task's message stream
- Modify: `src/components/TaskPanel/*`
  - show per-task provider bindings/status

### Tests / docs

- Modify: `src-tauri/src/daemon/state_tests.rs`
- Modify: `src-tauri/src/daemon/routing_shared_role_tests.rs`
- Modify: `src-tauri/src/daemon/routing_user_target_tests.rs`
- Modify: `src-tauri/src/daemon/provider/claude_tests.rs`
- Modify: `src-tauri/src/daemon/provider/codex_tests.rs`
- Modify: `src-tauri/src/daemon/telegram_lifecycle_tests.rs` only if unrelated compile fallout occurs (not expected; avoid unless necessary)
- Modify/add focused frontend tests adjacent to touched TS/TSX files

## Port-Race Constraint

This plan must not introduce Codex port races.

**Required invariant:** a Codex port may only move through these allocator states under a single serialized authority:

`Free -> Reserved(task_id, role, launch_id) -> Live(task_id, role, launch_id) -> CoolingDown(until_ms) -> Free`

Rules:

- no launch may pick a port without first creating a `Reserved` lease
- no stale success callback may promote a lease unless `launch_id` still matches
- no stop/failure path may release a port unless `launch_id` still matches
- cooldown is mandatory before reuse
- `lsof` cleanup remains defensive cleanup, never the allocator

## CM Memory

| Task | Commit | Summary | Verification | Status |
|------|--------|---------|--------------|--------|
| Task 1 | to be filled after implementation | Introduce task runtime registry and compatibility shims | Use Task 1 verification commands | planned |
| Task 2 | to be filled after implementation | Make launch/resume/stop explicit about task ownership | Use Task 2 verification commands | planned |
| Task 3 | to be filled after implementation | Route/buffer by task runtime instead of active task | Use Task 3 verification commands | planned |
| Task 4 | to be filled after implementation | Add race-free Codex port pool and multi-instance lifecycle | Use Task 4 verification commands | planned |
| Task 5 | to be filled after implementation | Frontend task-scoped message/runtime isolation | Use Task 5 verification commands | planned |

---

### Task 1: Introduce `TaskRuntimeRegistry` and compatibility shims

**task_id:** `task-runtime-registry-skeleton`

**Acceptance criteria:**

- `DaemonState` gains a `task_runtimes` registry keyed by `task_id`.
- A new `TaskRuntime` / `RuntimeSlot` model exists in a dedicated module.
- Existing global provider/runtime fields still work through compatibility shims so behavior does not change yet.
- Backend tests cover registry creation, lookup, and shim fallback.

**allowed_files:**

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/state_task_snapshot_tests.rs` only if snapshot assertions need updates

**max_files_changed:** `5`
**max_added_loc:** `260`
**max_deleted_loc:** `80`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_task_snapshot_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider:: -- --nocapture`
- `git diff --check`

---

### Task 2: Make provider lifecycle explicitly task-bound

**task_id:** `task-runtime-provider-binding`

**Acceptance criteria:**

- Launch/resume/stop/send command surfaces accept explicit `task_id` where task ownership matters.
- `register_on_launch()` / `register_on_connect()` stop reading `active_task_id`.
- The daemon run loop tracks Claude/Codex handles per task instead of one global handle each.
- Switching UI task focus during a launch no longer changes which task owns the resulting runtime.

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
**max_added_loc:** `320`
**max_deleted_loc:** `120`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::claude_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture`
- `git diff --check`

---

### Task 3: Route and buffer by task runtime instead of `active_task_id`

**task_id:** `task-runtime-routing`

**Acceptance criteria:**

- `route_message_inner` resolves target runtime by `BridgeMessage.task_id`.
- Task-scoped buffered messages no longer share one global queue.
- `stamp_message_context()` and `preferred_auto_target()` only use `active_task_id` as UI fallback, not as the primary routing key.
- Legacy messages without `task_id` still work through explicit fallback.

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

- Codex runtime allocation uses a central allocator with `Reserved`, `Live`, and `CoolingDown` leases.
- Two concurrent Codex launches cannot reserve the same port.
- Stale launch success/failure callbacks cannot steal or release another task's port.
- Codex stop/restart behavior remains compatible with orphan cleanup, but port ownership is allocator-driven rather than `lsof`-driven.

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
**max_added_loc:** `360`
**max_deleted_loc:** `120`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture`
- `git diff --check`

---

### Task 5: Move frontend launch/send/display into task scope

**task_id:** `task-runtime-frontend-isolation`

**Acceptance criteria:**

- Frontend launches Claude/Codex with explicit `taskId`.
- ReplyInput sends explicit `taskId`.
- Bridge message rendering shows only the selected task's messages.
- Task panel and agent-status surfaces show per-task provider bindings instead of one global provider status.
- Switching the selected task does not stop or rebind background tasks.

**allowed_files:**

- `src/stores/task-store/index.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/components/ClaudePanel/index.tsx`
- `src/components/ReplyInput/index.tsx`
- `src/components/MessagePanel/index.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/AgentStatus/index.tsx`
- focused adjacent test files for the touched frontend modules

**max_files_changed:** `12`
**max_added_loc:** `360`
**max_deleted_loc:** `180`

**verification_commands:**

- `bun test tests/task-store.test.ts tests/task-panel-view-model.test.ts src/components/ReplyInput/index.test.tsx src/components/MessagePanel/index.test.tsx`
- `bun test src/components/TaskPanel/TaskHeader.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx src/components/ClaudePanel/connect-state.test.ts src/components/ClaudePanel/launch-request.test.ts src/components/AgentStatus/codex-launch-config.test.ts`
- `bun run build`
- `git diff --check`

---

### Task 6: End-to-end migration cleanup and regression barrier

**task_id:** `task-runtime-final-regression-barrier`

**Acceptance criteria:**

- Old global runtime fields are either removed or reduced to clearly marked fallback-only shims.
- Background multi-task execution is verified with focused runtime tests and manual smoke instructions.
- Docs and CM memory are updated with the final runtime ownership model and port-pool invariants.

**allowed_files:**

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/mod.rs`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`
- `docs/superpowers/plans/2026-04-13-task-scoped-runtime-redesign.md`
- only directly relevant regression tests for touched daemon modules

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

- Do not skip directly to Task 4. The port pool depends on the task-scoped handle registry from Tasks 1-3.
- Do not start frontend isolation before provider binding is explicit; otherwise the UI will look task-scoped while the backend is still globally bound.
- Preserve legacy fallback for task-less messages until Task 6, then delete only after verification proves there are no live callers left.
