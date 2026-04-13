# Task-Scoped Runtime and Workspace Redesign

## Summary

The current system already has a multi-task graph, but the live execution layer is still global:

- one global Claude runtime
- one global Codex runtime
- one global active-task-driven routing context

That mismatch is the source of the current task race. Creating or switching tasks while providers are live can bind a launch to the wrong task, route against the wrong session, or let one task's stream/runtime state overwrite another's.

The redesign changes the execution model from:

- **global live lead/coder runtimes**

to:

- **per-task isolated workspace**
- **per-task lead/coder runtime slots**
- **routing by `task_id`**
- **foreground task selection as UI-only state**

Codex remains on **app-server**, not MCP server or Responses API. Because app-server is port-bound, the redesign adds a daemon-owned dynamic port pool with strict reservation and cooldown rules to eliminate port races.

## Product Goal

- Creating a task creates a dedicated task workspace.
- Each task owns its own lead/coder provider binding.
- Each task can continue running in the background while another task is selected.
- Routing correctness depends on `task_id`, not `active_task_id`.
- Codex multi-instance launches are race-free.

## Scope

### Included

- Task-scoped workspace provisioning.
- Task-scoped runtime registry in the daemon.
- Task-scoped provider launch/resume/stop semantics.
- Task-scoped routing and buffered delivery.
- Codex dynamic port pool for app-server instances.
- Frontend task-scoped message and runtime display.

### Excluded

- Replacing Codex app-server with another transport.
- Changing the normalized task/session graph shape in incompatible ways.
- Multi-user / remote scheduler support.
- Automatic deletion of task workspaces when tasks close.
- Support for automatic task workspace provisioning from non-git source folders in v1.

## Current Architecture Facts

### 1. Task graph is already task-scoped

`src-tauri/src/daemon/task_graph/types.rs` already models:

- `Task { task_id, workspace_root, lead_session_id, current_coder_session_id, ... }`
- `SessionHandle { session_id, task_id, provider, role, external_session_id, ... }`

This is the correct ownership boundary for normalized history and artifacts.

### 2. Task creation still reuses one shared workspace

`src-tauri/src/daemon/state_snapshot.rs::create_and_select_task()` currently stores the caller-provided workspace directly onto the new task. It does **not** create a dedicated task worktree/workspace.

That means two tasks created from one repository still share one mutable filesystem surface.

### 3. Live runtime is still singleton global state

`src-tauri/src/daemon/state.rs` currently stores exactly one live Claude runtime and one live Codex runtime:

- `claude_sdk_ws_tx`
- `codex_inject_tx`
- `claude_connection` / `codex_connection`
- `claude_role` / `codex_role`
- Claude nonce/preview/direct-text state
- single runtime epochs

### 4. Routing correctness still depends on UI focus

`src-tauri/src/daemon/state_task_flow.rs` and `src-tauri/src/daemon/routing.rs` still use:

- `active_task_id`
- global role ownership
- global provider channels

This is wrong for background execution. UI focus should not be part of delivery correctness.

### 5. Provider launch binding still uses `active_task_id`

`src-tauri/src/daemon/provider/claude.rs::register_on_launch()` and `provider/codex.rs::register_on_launch()` still bind live launches to the active task.

That is inherently race-prone once multiple tasks exist.

### 6. Codex concurrency is blocked by a single port assumption

`src-tauri/src/daemon/codex/lifecycle.rs` launches:

- `codex app-server --listen ws://127.0.0.1:{port}`

`src-tauri/src/daemon/ports.rs` currently exposes one Codex port.

So concurrent Codex runtimes require a real allocator. Without one, concurrent task launches will race for ports and orphan cleanup.

## Project Memory

### Recent related commits

- `577751e7` — unified task/session foundation
- `d676d2f4` — task session history workspace
- `c403ad69` — Codex provider history and resume adapter
- `0b1c29c6` — repair active-task routing and Claude launch binding
- `fa688747` — enforce task-scoped workspace routing boundaries
- `dda01a6c` — remember reply target per task
- `68d0ffd6` — Codex WS pump/session lifecycle stabilization
- `46488de7` — centralized daemon/Codex ports

### Relevant prior plans and docs

- `docs/superpowers/plans/2026-04-06-task-binding-routing-remediation.md`
- `docs/superpowers/plans/2026-04-07-reply-target-terminal-exit-polish.md`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`

### Constraints carried forward

- Binding launches through `active_task_id` is already proven fragile and must leave the correctness path.
- Workspace ownership must remain task-scoped and explicit.
- Reply target and other UI state already moved toward task-scoped semantics; runtime ownership should match that.
- Codex lifecycle issues often appear as stale launch completion, stale port holders, and misbound runtime state; the allocator must be launch-id-aware.

## Options Considered

### Option 1: Task-scoped runtime + per-task worktree + Codex port pool (recommended)

Each task owns:

- its own workspace root
- its own lead/coder runtime slots
- its own buffered delivery scope

Codex concurrency is handled by a central dynamic port pool.

**Pros**

- Matches the intended product model exactly.
- Eliminates the shared-workspace collision in addition to message/runtime races.
- Clean mental model: task = workspace + runtime + history.

**Cons**

- Highest implementation cost.
- Requires explicit Codex multi-instance lifecycle management.

### Option 2: Task-scoped runtime without per-task workspace

Keep task-scoped routing/runtime but continue reusing one shared repo workspace.

**Pros**

- Smaller change.

**Cons**

- Leaves the most dangerous class of collisions intact: concurrent file edits and command execution in the same cwd.
- Not acceptable for the new product direction.

### Option 3: Single active provider switching

Switch provider binding as the user changes the selected task.

**Pros**

- Lowest implementation cost.

**Cons**

- Violates the requirement that background tasks keep running.

## Recommended Design

Use Option 1.

The new model is:

```text
task_id
  -> task workspace root
  -> TaskRuntime
      -> lead slot
      -> coder slot
      -> task-scoped buffered delivery
      -> task-scoped runtime status
```

The user flow becomes:

1. choose a base repository workspace
2. create task
3. daemon provisions a dedicated task workspace/worktree
4. user chooses provider roles for that task
5. lead/coder launch into the task workspace
6. task keeps running even when another task is selected

## Architecture

### 1. Per-task workspace provisioning

This is foundational, not optional.

#### Rule

Creating a task from a git-backed workspace must create a dedicated task worktree.

Recommended canonical shape:

- base repo root: user-selected workspace
- task worktree root: `<repo>/.worktrees/tasks/<task_id>`
- task branch: `task/<task_id>`

`Task.workspace_root` must become the **task workspace root**, not the shared repo root.

#### New module

- `src-tauri/src/daemon/task_workspace.rs`

Responsibilities:

- validate that the selected workspace is a git repository root
- create `.worktrees/tasks` if needed
- verify `.worktrees` remains ignored
- allocate deterministic branch/worktree names from `task_id`
- provision the worktree before the task is treated as ready

#### First-version cleanup policy

- task creation creates the workspace immediately
- task close/archive does **not** auto-delete the workspace
- cleanup is explicit and can be automated later

### 2. `TaskRuntimeRegistry`

New daemon-local registry:

- `HashMap<String, TaskRuntime>`

New file:

- `src-tauri/src/daemon/task_runtime.rs`

Recommended shape:

```rust
pub struct TaskRuntime {
    pub task_id: String,
    pub workspace_root: String,
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
    pub epoch: u64,
    pub launch_nonce: Option<String>,
    pub runtime_meta: RuntimeSlotMeta,
}
```

### 3. `DaemonState` migration strategy

`src-tauri/src/daemon/state.rs` gains:

- `task_runtimes: HashMap<String, TaskRuntime>`

Migration strategy:

- keep current global runtime fields temporarily as compatibility shims
- add task-scoped accessors first
- migrate callers phase-by-phase

This keeps the transition controlled instead of doing a flag-day rewrite.

### 4. Routing becomes task-scoped

Affected files:

- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/state_delivery.rs`

New routing rules:

1. If `BridgeMessage.task_id` is present, resolve the target runtime from that `task_id`.
2. Within that task, resolve the target slot from `to = lead | coder`.
3. Deliver into that slot's channel.
4. Only messages that truly lack `task_id` may fall back to `active_task_id`.

This demotes `active_task_id` from correctness input to legacy fallback/UI focus.

### 5. Launch/resume APIs must accept explicit `task_id`

Affected files:

- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/launch_task_sync.rs`
- `src-tauri/src/daemon/provider/claude.rs`
- `src-tauri/src/daemon/provider/codex.rs`

New rule:

- every launch/resume/send operation carries explicit `task_id`
- launches derive `cwd` from the task's provisioned workspace
- no provider lifecycle path infers its task from `active_task_id`

### 6. Codex dynamic port pool with no port race

This is the highest-risk part and must be explicit.

#### Problem

The current single-port logic assumes one live Codex instance:

- `ensure_port_available(port, ...)`
- `kill_port_holder(port)`

That is not safe once more than one task can launch Codex concurrently.

#### New allocator

New file:

- `src-tauri/src/daemon/codex/port_pool.rs`

Recommended structures:

```rust
pub struct CodexPortPool {
    pub base_port: u16,
    pub size: u16,
    pub leases: HashMap<u16, PortLease>,
}

pub struct PortLease {
    pub task_id: String,
    pub role: SessionRole,
    pub launch_id: String,
    pub state: PortLeaseState,
}

pub enum PortLeaseState {
    Reserved,
    Live,
    CoolingDown { until_ms: u64 },
}
```

#### Allocation rules

1. Generate a unique `launch_id`.
2. Under a single serialized allocator, choose a free or expired-cooldown port.
3. Mark it `Reserved(task_id, role, launch_id)`.
4. Spawn Codex on that reserved port.
5. On successful handshake, promote the lease to `Live` **only if `launch_id` still matches**.
6. On stop/failure, move the lease to `CoolingDown` **only if `launch_id` still matches**.
7. Reuse only after cooldown expires.

#### Why this avoids port races

- no launch can spawn without first reserving a port
- stale success callbacks cannot promote another launch's lease
- stale stop/failure callbacks cannot release another launch's port
- cooldown prevents immediate reuse after orphan cleanup or kernel lag

`lsof` cleanup remains a defensive fallback only.

#### Config change

`src-tauri/src/daemon/ports.rs` must evolve from:

- single `codex: u16`

to:

- `codex_base: u16`
- `codex_pool_size: u16`

### 7. Claude task-scoped live state

Claude does not need a port pool, but it still needs task-scoped live state.

State that must move into the task slot:

- `claude_sdk_pending_nonce`
- `claude_sdk_active_nonce`
- `claude_sdk_ws_generation`
- `claude_sdk_direct_text_state`
- `claude_preview_buffer`
- `claude_preview_flush_scheduled`

Without this move, concurrent Claude tasks will still corrupt each other.

### 8. Task-scoped buffered delivery

Current `buffered_messages` is daemon-global.

It should become:

- `TaskRuntime.buffered_messages`

or equivalent `HashMap<task_id, Vec<BridgeMessage>>`.

That prevents one task's offline backlog from displacing another's.

### 9. Frontend changes

Affected files:

- `src/stores/task-store/index.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/components/ClaudePanel/index.tsx`
- `src/components/ReplyInput/index.tsx`
- `src/components/MessagePanel/index.tsx`
- `src/components/TaskPanel/*`
- `src/components/AgentStatus/*`

Design rules:

- the frontend still receives all bridge events
- visible selectors filter by `active_task_id`
- task creation flow becomes: create task -> workspace ready -> choose providers/roles
- task switching changes only the visible task, not the underlying runtime binding
- task rows show their own lead/coder provider status

## Behavior Changes

### Before

- there is effectively one live lead/coder pair
- two tasks can still share one mutable workspace
- switching task can change routing/launch semantics
- background tasks cannot safely keep running

### After

- each task owns an isolated workspace
- each task owns its own runtime binding
- switching task changes only the visible viewport
- background tasks continue executing
- launch/resume is deterministic by explicit `task_id`
- Codex app-server instances are allocated through a race-free port pool

## File Plan

### New files

- `src-tauri/src/daemon/task_workspace.rs`
- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/codex/port_pool.rs`

### Modified backend files

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
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
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/commands_task.rs`

### Modified frontend files

- `src/stores/task-store/index.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/components/ClaudePanel/index.tsx`
- `src/components/ReplyInput/index.tsx`
- `src/components/MessagePanel/index.tsx`
- `src/components/TaskPanel/*`
- `src/components/AgentStatus/*`
- `src/components/workspace-entry-state.ts`

## Testing Strategy

### Backend

- task workspace provisioning tests: worktree path allocation, branch naming, git-root validation, existing `.worktrees/tasks`
- per-task routing tests: `lead/coder` delivery resolves by `task_id`
- provider binding tests: explicit `task_id` launch does not mutate another task
- port allocator tests: reserve/promote/release/cooldown, no duplicate leases
- Codex lifecycle tests: stale launch completion cannot steal/release another task's port
- Claude state tests: preview/direct-text state stays isolated per task

### Frontend

- task creation flow shows task-scoped workspace semantics
- message panel only renders active task messages
- task rows show per-task runtime state
- switching visible task does not stop background execution

### Runtime smoke

- two tasks create two distinct worktrees
- two tasks can attach independent lead/coder sessions
- background task continues streaming while another task is selected
- stopping one task's Codex does not affect another task's Codex runtime

## Acceptance Criteria

- Each new task gets its own isolated task workspace before providers launch.
- Multiple tasks can have independent live lead/coder runtimes at the same time.
- Routing no longer depends on `active_task_id` when `task_id` is present.
- Provider launch/resume binds explicitly to the requested task.
- Codex concurrent launches are serialized through a central allocator and do not contend for the same port.
- Background tasks continue executing while a different task is selected in the UI.
- Frontend surfaces only the selected task's messages while preserving per-task runtime visibility.
