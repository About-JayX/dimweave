# Task-Scoped Runtime and Workspace Redesign

## Summary

The current system already has a task graph, but the live execution layer is still effectively singleton:

- one global Claude runtime
- one global Codex runtime
- one global `claude_role` / `codex_role`
- one global `active_task_id` used in too many correctness paths

That model cannot support real multi-task execution. If multiple tasks share one Claude runtime or one Codex runtime, launches race, routing can bind to the wrong task, and tasks serialize on one provider instance instead of running independently.

The redesign changes the execution model from:

- one global live Claude agent
- one global live Codex agent

to:

- one isolated workspace per task
- one task-local Claude runtime slot per task
- one task-local Codex runtime slot per task
- one task-local role binding map per task
- routing by `task_id`, with `active_task_id` demoted to UI focus only

Codex remains on `app-server`, not MCP server or Responses API. Because `app-server` is port-bound, multi-task Codex support requires a daemon-owned dynamic port pool with reservation, handshake promotion, and cooldown rules.

## Product Goal

- Creating a task creates a dedicated task workspace/worktree.
- Every task owns its own Claude agent state and its own Codex agent state.
- Every task persists its own role bindings instead of relying on global `claude_role` / `codex_role`.
- Two tasks can run concurrently without sharing provider runtime state.
- Routing correctness depends on `BridgeMessage.task_id`, not `active_task_id`.
- Codex concurrent launches are race-free.

## Scope

### Included

- Task-scoped workspace provisioning.
- Task-scoped provider binding metadata on the task graph.
- Task-scoped Claude runtime state.
- Task-scoped Codex runtime state.
- Task-scoped routing, status lookup, and buffered delivery.
- Codex dynamic port pool for app-server instances.
- Frontend task-scoped launch/send/status/message display.

### Excluded

- Replacing Codex app-server with another transport.
- Generalizing non-Claude/non-Codex `attached_agents` into multi-instance task runtimes in this wave.
- Automatic deletion of task workspaces when tasks close.
- Multi-user or remote scheduler support.
- A generic provider-plugin system.

## Current Architecture Facts

### 1. The normalized task graph exists, but it does not persist provider bindings

`src-tauri/src/daemon/task_graph/types.rs` currently has:

- `Task { task_id, workspace_root, title, status, lead_session_id, current_coder_session_id, ... }`
- `SessionHandle { session_id, task_id, provider, role, external_session_id, ... }`

What is missing is task-local provider ownership metadata such as:

- `lead_provider`
- `coder_provider`

Without those fields, task semantics still leak through the global `claude_role` / `codex_role`.

### 2. Task creation still reuses one shared workspace

`src-tauri/src/daemon/state_snapshot.rs::create_and_select_task()` currently stores the caller-provided workspace directly on the task. It does not provision a dedicated task worktree.

That means two tasks created from the same repo still share one mutable filesystem surface.

### 3. Live runtime state is still singleton global state

`src-tauri/src/daemon/state.rs` currently stores exactly one live Claude runtime and one live Codex runtime:

- `claude_sdk_ws_tx`
- `claude_sdk_event_tx`
- `claude_sdk_ready_tx`
- `codex_inject_tx`
- `claude_connection` / `codex_connection`
- `claude_role` / `codex_role`
- Claude nonce / preview / direct-text state
- one Codex launch epoch

That is incompatible with multiple tasks owning independent Claude/Codex agents.

### 4. Routing and message stamping still depend on global focus/role state

`src-tauri/src/daemon/routing.rs`, `state_task_flow.rs`, `state_delivery.rs`, `control/handler.rs`, `codex/session_event.rs`, and `codex/handler.rs` still resolve too much through:

- `active_task_id`
- global `claude_role`
- global `codex_role`

That is wrong for background task execution.

### 5. Claude and Codex runtime attachment points are still global

Claude and Codex launch/attach/reconnect logic still hangs off global state transitions:

- Claude: `claude_sdk/runtime.rs`, `control/claude_sdk_handler.rs`, `claude_sdk/reconnect.rs`
- Codex: `codex/mod.rs`, `codex/runtime.rs`, `codex/session.rs`

Those paths currently protect against stale callbacks only inside a singleton runtime, not across task-local runtime slots.

### 6. Status and UI contracts are still global-provider oriented

`state_snapshot.rs`, `types.rs`, `types_dto.rs`, `gui_task.rs`, and the frontend bridge/task stores still assume there is one global Claude status and one global Codex status.

That is insufficient once every task owns its own provider pair.

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
- `8cb38d7e` — revise redesign docs for per-task workspaces

### Relevant prior plans and docs

- `docs/superpowers/plans/2026-04-06-task-binding-routing-remediation.md`
- `docs/superpowers/plans/2026-04-07-reply-target-terminal-exit-polish.md`
- `docs/agents/codex-chain.md`
- `docs/agents/claude-chain.md`

### Constraints carried forward

- `active_task_id` must leave the correctness path.
- Workspace ownership must remain explicit and task-scoped.
- Reply-target memory already moved to task scope; runtime ownership must match.
- Codex lifecycle bugs commonly appear as stale success/failure callbacks and stale port holders; the allocator must be launch-id-aware.
- This wave is about real multi-task execution, so every task must own both a Claude runtime slot and a Codex runtime slot even if one slot is idle.

## Options Considered

### Option 1: Per-task workspace + per-task Claude/Codex slots + task-local role bindings (recommended)

Each task owns:

- its own workspace root
- its own persisted role bindings
- its own Claude slot
- its own Codex slot
- its own buffered delivery scope

Routing becomes:

`task_id -> task role binding -> provider slot -> send channel`

**Pros**

- Matches the product requirement that each task owns dedicated Claude/Codex agents.
- Eliminates shared-workspace collisions and provider-state races together.
- Makes background execution deterministic.

**Cons**

- Highest implementation cost.
- Requires explicit Codex multi-instance lifecycle management.

### Option 2: Per-task role slots only

Each task owns `lead` and `coder` slots, but provider live state remains mixed inside those role slots.

**Pros**

- Smaller conceptual change from the current model.

**Cons**

- It still hides provider-specific live state behind role ownership.
- It does not directly satisfy the requirement that every task own its own Claude and Codex agent state.

### Option 3: Keep global Claude/Codex and patch with `active_task_id`

**Pros**

- Lowest implementation cost.

**Cons**

- Preserves the race.
- Forces tasks to serialize on shared provider state.
- Not acceptable for real multi-task execution.

## Recommended Design

Use Option 1.

The core ownership model becomes:

```text
task_id
  -> task workspace root
  -> Task { lead_provider, coder_provider, ... }
  -> TaskRuntime
      -> claude slot
      -> codex slot
      -> task-scoped buffered delivery
      -> task-scoped runtime status
```

The runtime routing rule becomes:

```text
BridgeMessage.task_id
  -> task
  -> resolve target role (lead | coder)
  -> resolve bound provider from task.lead_provider / task.coder_provider
  -> deliver to that provider's task-local runtime slot
```

## Architecture

### 1. Task model gains persisted provider bindings

This wave needs explicit task-local provider ownership.

`src-tauri/src/daemon/task_graph/types.rs::Task` must gain:

- `lead_provider: Provider`
- `coder_provider: Provider`

`src-tauri/src/daemon/task_graph/store.rs::create_task()` must initialize those fields explicitly.

For this wave, the default task pair is:

- `lead_provider = Provider::Claude`
- `coder_provider = Provider::Codex`

That matches the current product expectation that each task gets a Claude lead and a Codex coder. If per-task provider reassignment is needed later, it can build on these persisted fields instead of reintroducing global provider semantics.

### 2. Per-task workspace provisioning

Creating a task from a git-backed workspace must create a dedicated task worktree before the task is considered ready.

Canonical shape:

- base repo root: user-selected workspace
- task worktree root: `<repo>/.worktrees/tasks/<task_id>`
- task branch: `task/<task_id>`

`Task.workspace_root` must store the task worktree path, not the shared repo root.

New module:

- `src-tauri/src/daemon/task_workspace.rs`

Responsibilities:

- validate the selected workspace is a git repository root
- create `.worktrees/tasks` if needed
- verify `.worktrees` remains ignored
- allocate deterministic branch/worktree names from `task_id`
- provision the worktree before task creation succeeds

### 3. `TaskRuntimeRegistry`

New daemon-local registry:

- `HashMap<String, TaskRuntime>`

New file:

- `src-tauri/src/daemon/task_runtime.rs`

Recommended shape:

```rust
pub struct TaskRuntime {
    pub task_id: String,
    pub workspace_root: String,
    pub claude: ProviderRuntimeSlot,
    pub codex: ProviderRuntimeSlot,
    pub buffered_messages: Vec<BridgeMessage>,
}

pub struct ProviderRuntimeSlot {
    pub provider: Provider,
    pub bound_role: Option<SessionRole>,
    pub normalized_session_id: Option<String>,
    pub external_session_id: Option<String>,
    pub connection: Option<ProviderConnectionState>,
    pub send_channel: Option<RuntimeSendChannel>,
    pub launch_id: Option<String>,
    pub epoch: u64,
    pub runtime_meta: ProviderRuntimeMeta,
}
```

The role binding lives on the task. The live runtime state lives on the provider slot.

### 4. `DaemonState` migration strategy

`src-tauri/src/daemon/state.rs` gains:

- `task_runtimes: HashMap<String, TaskRuntime>`

Migration strategy:

- keep current global fields temporarily as compatibility mirrors
- add task-scoped accessors first
- move Claude/Codex attachment points onto task-local runtime slots
- remove or deprecate singleton fields only after routing and frontend contracts are updated

This avoids a flag-day rewrite.

### 5. Launch/resume APIs must accept explicit `task_id`

Affected surfaces:

- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/provider/claude.rs`
- `src-tauri/src/daemon/provider/codex.rs`

New rules:

- every Claude launch/resume/send/stop operation carries explicit `task_id`
- every Codex launch/resume/send/stop operation carries explicit `task_id`
- launches derive `cwd` from the provisioned task workspace
- no provider lifecycle path infers task ownership from `active_task_id`

### 6. Claude live state becomes task-scoped

Claude does not need a port pool, but it does need per-task live state.

State that must move into the Claude task slot:

- `claude_sdk_ready_tx`
- `claude_sdk_event_tx`
- `claude_sdk_pending_nonce`
- `claude_sdk_active_nonce`
- `claude_sdk_ws_generation`
- `claude_sdk_direct_text_state`
- `claude_preview_buffer`
- `claude_preview_flush_scheduled`

Without that move, one task's Claude reconnect or preview flush can still corrupt another task's state.

### 7. Codex uses a dynamic port pool

The current single-port model is incompatible with per-task Codex agents.

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

Allocation rules:

1. Generate a unique `launch_id`.
2. Under one serialized allocator, choose a free or expired-cooldown port.
3. Mark it `Reserved(task_id, role, launch_id)`.
4. Spawn Codex on that reserved port.
5. Promote to `Live` only if the same `launch_id` completes the handshake.
6. Move to `CoolingDown` only if the same `launch_id` owns the stop/failure callback.
7. Reuse only after cooldown expires.

`lsof` cleanup remains defensive fallback only.

### 8. Routing becomes task-scoped and role-binding-aware

Affected paths:

- `routing.rs`
- `routing_user_input.rs`
- `routing_target_session.rs`
- `state_task_flow.rs`
- `state_delivery.rs`
- `control/handler.rs`
- `codex/session_event.rs`
- `codex/handler.rs`
- `control/claude_sdk_handler_processing.rs`

New routing rules:

1. If `BridgeMessage.task_id` exists, resolve the owning task runtime from that `task_id`.
2. Resolve `to = lead | coder`.
3. Read the bound provider from the task's persisted role bindings.
4. Deliver to that provider's task-local runtime slot.
5. Only messages that truly lack `task_id` may fall back to `active_task_id`.

This demotes `active_task_id` to UI focus and legacy compatibility only.

### 9. Buffered delivery becomes task-scoped

`buffered_messages` must move from daemon-global storage to task-local storage:

- `TaskRuntime.buffered_messages`

That prevents one task's backlog from displacing another's and makes `check_messages` / replay semantics task-correct.

### 10. Task context and status snapshots become task-aware

Affected files:

- `state_snapshot.rs`
- `types.rs`
- `types_dto.rs`
- `gui_task.rs`

Required behavior:

- task snapshots and task events expose per-task provider bindings and runtime summaries
- global daemon status becomes compatibility-oriented, not the source of truth for task ownership
- task-local tool calls such as `get_status` and `check_messages` must resolve against the caller's task runtime, not the globally active task

### 11. Frontend changes

Affected areas:

- task store
- bridge store
- task panel
- message panel
- Claude panel
- agent status panel

Design rules:

- task creation flow becomes: choose workspace -> create task -> daemon provisions task workspace -> task shows Claude/Codex pair for that task
- frontend launches providers with explicit `taskId`
- visible panels filter by `activeTaskId`
- background tasks continue running even when not selected
- the UI no longer treats one global Claude/Codex status as the ownership truth

## Behavior Changes

### Before

- one live Claude runtime and one live Codex runtime effectively gate the whole app
- multiple tasks can share one mutable workspace
- routing and status resolution can change when `active_task_id` changes
- tasks serialize on shared provider state

### After

- each task owns its own isolated workspace
- each task owns its own Claude slot and Codex slot
- each task persists its own provider bindings
- routing resolves by `task_id` first
- background tasks continue executing while another task is selected
- Codex launches use a race-free port allocator

## File Plan

### New files

- `src-tauri/src/daemon/task_workspace.rs`
- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/codex/port_pool.rs`

### Modified backend files

- `src-tauri/src/daemon/task_graph/types.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/state_snapshot_tests.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/commands_task.rs`
- `src-tauri/src/daemon/provider/claude.rs`
- `src-tauri/src/daemon/provider/codex.rs`
- `src-tauri/src/daemon/claude_sdk/runtime.rs`
- `src-tauri/src/daemon/claude_sdk/mod.rs`
- `src-tauri/src/daemon/claude_sdk/reconnect.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/codex/mod.rs`
- `src-tauri/src/daemon/codex/runtime.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/codex/lifecycle.rs`
- `src-tauri/src/daemon/codex/handler.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_display.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/daemon/ports.rs`

### Modified frontend files

- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/events.ts`
- `src/stores/bridge-store/types.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/stores/bridge-store/sync.ts`
- `src/components/workspace-entry-state.ts`
- `src/components/ClaudePanel/index.tsx`
- `src/components/ReplyInput/index.tsx`
- `src/components/MessagePanel/index.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/view-model.ts`
- `src/components/AgentStatus/index.tsx`

## Testing Strategy

### Backend

- task creation tests: worktree path allocation, branch naming, non-git root rejection, persisted provider-binding defaults
- task runtime tests: one task gets one Claude slot and one Codex slot
- Claude tests: task A reconnect/disconnect cannot clear task B Claude state
- Codex tests: task A launch/stop cannot steal task B port or runtime slot
- routing tests: `task_id + role` resolves through task-local provider binding, not global role state
- status tests: task-local `get_status` / `check_messages` views remain correct when another task is active

### Frontend

- task creation flow reflects dedicated task workspace semantics
- task store holds task-local provider binding/runtime summaries
- launch/send flows pass explicit `taskId`
- message panel renders only active task messages
- task panel shows per-task Claude/Codex runtime state

### Runtime smoke

- two tasks create two distinct worktrees
- two tasks can each hold an independent Claude runtime
- two tasks can each hold an independent Codex runtime
- task A can keep streaming while task B is selected
- stopping task A Codex does not disconnect task B Codex

## Acceptance Criteria

- Each task gets its own isolated task workspace before providers launch.
- Each task owns its own Claude runtime state and Codex runtime state.
- Task-local provider bindings are persisted on the task graph.
- Routing no longer depends on `active_task_id` when `task_id` is present.
- Claude reconnect/preview/direct-text state cannot bleed across tasks.
- Codex concurrent launches cannot reserve or release the same port lease incorrectly.
- Background tasks continue executing while a different task is selected.
- Frontend surfaces only the selected task's message stream while preserving per-task runtime visibility.

## Final Task-Local Ownership Model (Post-Implementation)

### Authoritative ownership chain

```text
Task.lead_provider / Task.coder_provider   (persisted in task_graph)
  → resolve_task_provider_agent(task_id, role) → "claude" | "codex"
  → TaskRuntime.claude_slot / codex_slot       (live runtime state)
  → slot.connection                            (provider session metadata)
  → is_task_agent_online(task_id, agent)       (online check)
  → task_provider_summary(task_id)             (frontend DTO)
```

### Compatibility-only singletons

The following `DaemonState` fields are retained for legacy/pre-task callers
but are **not authoritative** for task-scoped operations:

| Field | Task-scoped replacement |
|-------|------------------------|
| `claude_role` / `codex_role` | `Task.lead_provider` / `Task.coder_provider` |
| `claude_connection` / `codex_connection` | `ClaudeTaskSlot.connection` / `CodexTaskSlot.connection` |
| `claude_sdk_ws_tx` / `codex_inject_tx` | `ClaudeTaskSlot.ws_tx` / `CodexTaskSlot.inject_tx` |
| `active_task_id` | Explicit `task_id` param in commands and messages |
| `DaemonStatusSnapshot.claude_role` / `.codex_role` | `TaskProviderSummary` per-task DTO |

### Allocator invariants

- **Port allocation**: Codex `CodexTaskSlot.port` is unique per-task; allocated via `begin_codex_task_launch` which checks `codex_used_ports()`.
- **Session epoch**: Each slot carries its own `session_epoch` to reject stale callbacks from prior launches of the same task.
- **Connection metadata**: `slot.connection` is set at connect time alongside the global mirror; `task_provider_connection()` reads from the slot, never the mirror.
