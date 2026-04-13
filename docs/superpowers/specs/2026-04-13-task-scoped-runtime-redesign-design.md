# Task-Scoped Runtime Registry Redesign

## Summary

The current daemon mixes two different models:

- the task graph already supports many `task_id`s, each with its own normalized lead/coder session handles
- the live runtime still exposes exactly one global Claude runtime and one global Codex runtime

That mismatch is the source of the current task race. Routing, provider launch binding, stream state, and buffered delivery still depend on global singleton fields plus `active_task_id`. As soon as the user creates or switches tasks while agents are online, the system can bind a provider launch to the wrong task, buffer against the wrong task session, or let one task's runtime state overwrite another's.

The redesign keeps the existing task graph and normalized `session_id` model, but replaces the global live-runtime singleton with a task-scoped runtime registry:

- each `task_id` owns its own lead/coder runtime slots
- routing resolves the target runtime from `task_id`, not from UI focus
- switching tasks only changes the foreground view
- background tasks keep running

The highest-risk part is Codex. Our current Codex integration launches `codex app-server --listen ws://127.0.0.1:{port}` and therefore behaves as one runtime instance per bound port. To support concurrent Codex tasks without port races, the redesign introduces an explicit daemon-side port-pool allocator with reservation and cooldown semantics.

## Product Goal

- Multiple tasks can be online at the same time.
- Each task owns its own lead/coder runtime binding.
- Switching the visible task does not stop or rebind background tasks.
- Routing, buffering, and provider lifecycle no longer depend on `active_task_id`.
- Codex multi-instance launch is race-free.

## Scope

### Included

- Task-scoped runtime registry inside the daemon.
- Task-scoped provider launch/resume binding.
- Task-scoped routing and buffered delivery.
- Task-scoped Codex port allocation.
- Frontend task-scoped message/status selection.
- Compatibility fallback for legacy messages that still omit `task_id`.

### Excluded

- Redesigning the normalized task graph schema itself.
- Changing the lead/coder role protocol or prompt content.
- Replacing Codex app-server with a different transport.
- Multi-user or remote cluster scheduling.
- Persisting full runtime registry state across app restarts beyond existing session/task graph persistence.

## Current Architecture Facts

### 1. Task graph is already multi-task

`src-tauri/src/daemon/task_graph/types.rs` already persists:

- `Task { task_id, lead_session_id, current_coder_session_id, ... }`
- `SessionHandle { session_id, task_id, provider, role, external_session_id, ... }`

This is the right ownership model. The problem is not the task graph.

### 2. Live runtime is still global

`src-tauri/src/daemon/state.rs` currently holds singleton runtime state:

- `claude_sdk_ws_tx`
- `codex_inject_tx`
- `claude_connection` / `codex_connection`
- `claude_role` / `codex_role`
- `claude_sdk_session_epoch` / `codex_session_epoch`
- `claude_sdk_pending_nonce` / `claude_sdk_active_nonce`
- `claude_sdk_direct_text_state`
- `claude_preview_buffer`

These fields encode exactly one live Claude runtime and one live Codex runtime.

### 3. Routing still depends on UI focus

`src-tauri/src/daemon/state_task_flow.rs` and `src-tauri/src/daemon/routing.rs` still rely on:

- `active_task_id` for `stamp_message_context()`
- `active_task_id` for `preferred_auto_target()`
- global runtime fields for deciding which provider is online

This makes UI focus part of the delivery path. That is the wrong boundary for background execution.

### 4. Provider launch binding still uses `active_task_id`

`src-tauri/src/daemon/provider/claude.rs::register_on_launch()` and
`src-tauri/src/daemon/provider/codex.rs::register_on_launch()` both bind new live sessions to the active task.

That is acceptable only in a single-live-task world. It is incorrect once tasks can execute concurrently.

### 5. Codex concurrency is currently blocked by a single fixed port

`src-tauri/src/daemon/ports.rs` exposes one `codex` port.
`src-tauri/src/daemon/codex/lifecycle.rs` launches a single app-server instance per port.
`src-tauri/src/daemon/codex/runtime.rs::ensure_port_available()` is written for one chosen port, not a pool.

So concurrent Codex runtimes require an allocation layer. Without it, task concurrency would immediately degrade into port races and orphan cleanup fights.

## Project Memory

### Recent related commits

- `577751e7` — initial unified task/session foundation
- `d676d2f4` — task session history workspace
- `c403ad69` — Codex provider history and resume adapter
- `0b1c29c6` — repair active-task routing and Claude launch binding
- `fa688747` — enforce task-scoped workspace routing boundaries
- `dda01a6c` — remember reply target per task
- `68d0ffd6` — stabilize Codex WS pump loop and session lifecycle
- `46488de7` — centralize daemon/Codex port config

### Relevant prior plans and docs

- `docs/superpowers/plans/2026-04-06-task-binding-routing-remediation.md`
- `docs/superpowers/plans/2026-04-07-reply-target-terminal-exit-polish.md`
- `docs/superpowers/plans/2026-04-13-telegram-route-loop-fix.md`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`

### Constraints carried forward

- `0b1c29c6` proved that binding launches through `active_task_id` is fragile even in the current single-runtime model.
- `fa688747` established that workspace ownership must remain task-scoped and explicit.
- `dda01a6c` already moved reply-target memory into task-scoped UI state; the runtime redesign should continue that direction instead of re-globalizing UI state.
- Codex runtime history/resume is already provider-native and task-aware at the normalized session layer; the redesign should reuse that foundation instead of inventing another session identifier system.

## Options Considered

### Option 1: Task-scoped runtime registry with provider multi-instance support (recommended)

Each `task_id` owns its own runtime slots. Routing and provider lifecycle use `task_id` as the first-class dispatch key. Codex concurrency is achieved through a daemon-managed dynamic port pool.

**Pros**

- Matches the user requirement exactly.
- Removes `active_task_id` from runtime correctness.
- Gives the cleanest mental model: UI focus is not delivery scope.
- Supports background execution without fake switching or rebinding.

**Cons**

- Highest implementation cost.
- Codex needs explicit multi-instance lifecycle management.
- More runtime state to observe and test.

### Option 2: Single provider process with internal multi-session multiplexing

Keep one Claude runtime and one Codex runtime process, but multiplex many tasks inside them.

**Pros**

- Lower process and port count.

**Cons**

- Not supported by the current integration shape.
- Requires deeper changes to provider-specific runtime code than Option 1.
- Higher risk of hidden cross-task state contamination.

### Option 3: Single-live-provider switching

Only the foreground task is truly live; switching tasks resumes/binds the appropriate session.

**Pros**

- Smaller change.

**Cons**

- Violates the user requirement that background tasks continue running.

## Recommended Design

Use Option 1.

The new daemon runtime model is:

```text
task_id
  -> TaskRuntime
      -> lead slot
      -> coder slot
      -> task-scoped buffered messages
      -> task-scoped runtime status snapshot
```

`active_task_id` remains useful, but only as the current UI focus. It must no longer decide where launches bind or where messages route.

## Architecture

### 1. `TaskRuntimeRegistry`

Add a new daemon-local registry:

- `HashMap<String, TaskRuntime>`

New file:

- `src-tauri/src/daemon/task_runtime.rs`

Recommended core shapes:

```rust
pub struct TaskRuntime {
    pub task_id: String,
    pub lead: Option<RuntimeSlot>,
    pub coder: Option<RuntimeSlot>,
    pub buffered_messages: Vec<BridgeMessage>,
}

pub struct RuntimeSlot {
    pub provider: Provider,
    pub normalized_session_id: Option<String>,
    pub external_session_id: Option<String>,
    pub connection: Option<ProviderConnectionState>,
    pub send_channel: Option<RuntimeSendChannel>,
    pub role: String,
    pub epoch: u64,
    pub launch_nonce: Option<String>,
    pub runtime_meta: RuntimeSlotMeta,
}
```

`RuntimeSlotMeta` should hold provider-specific runtime state that is currently global:

- for Claude:
  - direct-text state
  - preview buffer
  - preview flush flag
  - pending/active nonce
  - ws generation
- for Codex:
  - bound port
  - runtime status / process metadata

The important design rule is that provider-specific live state moves with the task slot, not with the daemon singleton.

### 2. `DaemonState` migration strategy

`src-tauri/src/daemon/state.rs` should gain:

- `task_runtimes: HashMap<String, TaskRuntime>`

The current global fields should not be deleted in the first phase. Instead:

- keep them temporarily as compatibility shims
- add accessor methods that resolve task-scoped runtime first
- migrate callers in phases

This avoids a flag-day rewrite and gives a safe intermediate state for tests.

### 3. Routing becomes task-scoped

Affected files:

- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/state_delivery.rs`

New routing rules:

1. If `BridgeMessage.task_id` is present, resolve the target runtime from that `task_id`.
2. Within that task, resolve the target slot from `to = lead | coder`.
3. Deliver into that task slot's channel.
4. Only messages that truly lack `task_id` may fall back to `active_task_id`.

This changes `active_task_id` from a correctness input to a legacy fallback.

### 4. Launch/resume APIs must accept explicit `task_id`

Affected files:

- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/provider/claude.rs`
- `src-tauri/src/daemon/provider/codex.rs`
- `src-tauri/src/daemon/launch_task_sync.rs`

New rule:

- every launch or resume path receives `task_id` explicitly
- `register_on_launch()` and `register_on_connect()` bind to that `task_id`
- they never infer the target task from `active_task_id`

This is the most important semantic correction after the runtime registry itself.

### 5. Codex dynamic port pool with no port race

This is the critical design section.

#### Problem

The current system has one Codex port and uses:

- `ensure_port_available(port, ...)`
- `kill_port_holder(port)`

That is safe only in a single-instance world. In a multi-task world, two concurrent launches could race for the same port if allocation is not centrally serialized.

#### Required design

Add a daemon-owned allocator:

- new file: `src-tauri/src/daemon/codex/port_pool.rs`

Recommended structures:

```rust
pub struct CodexPortPool {
    pub base_port: u16,
    pub size: u16,
    pub leases: HashMap<u16, PortLease>,
}

pub enum PortLeaseState {
    Reserved { task_id: String, launch_id: String },
    Live { task_id: String, launch_id: String },
    CoolingDown { until_ms: u64 },
}
```

The allocator must be owned by daemon state and mutated only while holding the daemon write lock or a dedicated mutex.

#### Allocation algorithm

1. Generate a unique `launch_id`.
2. Under lock, scan the pool for:
   - an existing live lease for the same task/runtime slot, or
   - a free port, or
   - an expired cooldown port
3. Mark the chosen port `Reserved { task_id, launch_id }`.
4. Release the lock.
5. Spawn Codex on that port.
6. On successful handshake, promote the lease to `Live`.
7. On launch failure, mark it `CoolingDown { until_ms }`.
8. On stop, kill the child, release the port to cooldown, and only later return it to the free pool.

#### Why this avoids port races

- port assignment is serialized before process spawn
- launch success/failure is bound to `launch_id`, so stale completion cannot steal or release another launch's port
- cooldown prevents immediate port reuse after orphan cleanup or kernel release lag
- `lsof` cleanup remains a defensive fallback, not the primary allocation mechanism

#### Required configuration change

`src-tauri/src/daemon/ports.rs` must evolve from:

- single `codex: u16`

to something like:

- `codex_base: u16`
- `codex_pool_size: u16`

Default pool values can remain conservative, for example `4500` + pool size `8`.

### 6. Claude task-scoped live state

Claude does not need a port pool, but it does need per-task live state.

Affected files:

- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/claude_sdk/runtime.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/provider/claude.rs`

The existing single-instance Claude state that must move into `RuntimeSlotMeta`:

- `claude_sdk_pending_nonce`
- `claude_sdk_active_nonce`
- `claude_sdk_ws_generation`
- `claude_sdk_direct_text_state`
- `claude_preview_buffer`
- `claude_preview_flush_scheduled`

Without that migration, concurrent Claude task streams will still corrupt each other even if routing is corrected.

### 7. Task-scoped buffered delivery

Current `buffered_messages` is daemon-global.

That should become task-scoped:

- either inside `TaskRuntime.buffered_messages`
- or `HashMap<task_id, Vec<BridgeMessage>>`

Affected files:

- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_persistence.rs`

This prevents one task's offline backlog from displacing or contaminating another task's backlog.

### 8. Frontend changes

Affected files:

- `src/stores/task-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/components/TaskPanel/*`
- `src/components/MessagePanel/*`
- `src/components/ReplyInput/*`
- `src/components/AgentStatus/*`

Design rules:

- the frontend keeps receiving all bridge events
- display selectors filter them by `active_task_id`
- task switching swaps the visible task, not the underlying runtime binding
- task rows should show their own lead/coder provider status

The frontend must stop assuming that a single global `claudeRole` / `codexRole` is sufficient status for all tasks.

## Behavior Changes

### Before

- there is effectively one live lead/coder pair
- switching task changes routing semantics
- launch and resume can bind to the wrong task
- background tasks cannot safely continue

### After

- each task owns its own runtime binding
- switching task only changes the current viewport
- background tasks continue executing
- launch/resume is deterministic by explicit `task_id`
- Codex multi-instance launch is serialized and race-free

## File Plan

### New files

- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/codex/port_pool.rs`
- task-scoped tests alongside the touched daemon modules as needed

### Modified backend files

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/launch_task_sync.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/provider/claude.rs`
- `src-tauri/src/daemon/provider/codex.rs`
- `src-tauri/src/daemon/codex/mod.rs`
- `src-tauri/src/daemon/codex/runtime.rs`
- `src-tauri/src/daemon/codex/lifecycle.rs`
- `src-tauri/src/daemon/ports.rs`

### Modified frontend files

- `src/stores/task-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/components/TaskPanel/*`
- `src/components/MessagePanel/*`
- `src/components/ReplyInput/*`
- `src/components/AgentStatus/*`

## Testing Strategy

### Backend

- per-task routing tests: `lead/coder` delivery resolves by `task_id`
- launch binding tests: explicit `task_id` launch does not mutate another task
- port allocator tests: reserve/promote/release/cooldown; no duplicate leases
- Codex lifecycle tests: stale launch completion cannot steal/release another task port
- Claude state tests: preview/direct-text state stays isolated per task

### Frontend

- switching visible task does not mutate background task runtime state
- message panels only render messages for `active_task_id`
- task rows show per-task runtime status

### Runtime smoke

- two tasks can attach independent lead/coder sessions
- background task continues streaming while another task is selected
- stopping one task's Codex does not affect another task's Codex runtime

## Acceptance Criteria

- Multiple tasks can have independent live lead/coder runtimes at the same time.
- Routing no longer depends on `active_task_id` when `task_id` is present on the message.
- Provider launch/resume binds explicitly to the requested task.
- Codex concurrent launches are serialized through a central allocator and do not contend for the same port.
- Background tasks continue executing while a different task is selected in the UI.
- Frontend surfaces only the selected task's messages while preserving task-specific runtime visibility.
