# Task Agent Identity And Role-Broadcast Design

> **Supersedes:** The task/agent modeling portions of [2026-04-13-task-first-sidebar-and-ui-error-log-design.md](2026-04-13-task-first-sidebar-and-ui-error-log-design.md). The single-workspace flow and UI error log work remain valid; the single-slot `lead_provider/coder_provider` task model does not.
>
> **Status: Stage-complete.** All implementation tasks (1-6) have been accepted. The `task_agents[]` model is the sole source of truth for task-agent identity, role targeting, and broadcast routing.

## Goal

Replace the current single-slot task/agent model with a final agent-identity model where each task owns an ordered list of agents, each agent has its own stable internal id and extensible role string, and role-targeted sends broadcast to every matching agent in that task.

## Why The Current Model Is Wrong

The currently-landed task-first UI still assumes that a task fundamentally has one `lead` slot and one `coder` slot. That assumption leaks into both backend data and frontend UX:

- `Task` still stores `lead_provider` / `coder_provider`
- `Task` still stores `lead_session_id` / `current_coder_session_id`
- routing still resolves “who should receive `lead` or `coder`” from task-level singleton bindings
- the task setup dialog still works by choosing one provider for `lead` and one provider for `coder`

That model is incompatible with the approved product direction:

- a task may have multiple agents with the same role
- agent identity belongs to the agent itself, not to a task slot
- roles are extensible strings, not a fixed `lead/coder` enum from the product’s point of view
- sends targeted at a role broadcast to all agents with that role

Once those requirements are accepted, `lead_provider/coder_provider` can no longer be the product truth. At best they are a migration source; at worst they create the exact race/confusion the user already called out.

## Product Model

### Workspace

- The app remains single-workspace for this phase.
- A workspace can exist with zero tasks.
- The selected workspace remains frontend state independent from `activeTaskId`.

### Task

- A task is metadata plus conversation/session/artifact ownership.
- A task may exist with zero agents.
- A new task still defaults its visible title to the generated `task_id`.

### Task Agent

Every task owns `task_agents[]`.

Each task agent has:

- `agent_id` — system-generated stable internal id
- `task_id`
- `provider` — e.g. `claude`, `codex`, future providers
- `role` — arbitrary non-empty string
- `display` / config fields needed by the provider setup flow
- runtime summary / connection summary
- ordering metadata
- timestamps

### Role

- `role` is not restricted to `lead` / `coder`.
- Multiple agents in the same task may share the same role.
- Roles that exist in the current task become valid message targets.

## Core Routing Semantics

### Explicit Target

If the user sends to `target=<role>`:

1. resolve the active task
2. find all task agents in that task whose `role == <role>`
3. deliver the message to every matching agent

This is broadcast-by-role, not single-recipient routing.

### Auto Target

`auto` resolves using the current task’s role inventory:

1. if the task contains any `lead` agents, use `lead`
2. otherwise use the first role in the task’s ordered role list
3. broadcast to every agent with that chosen role

### Provider-Originated Messages

Provider-originated events must stop inferring ownership through `lead_provider/coder_provider` or similar slot logic.

Instead:

- runtime/session ownership maps to a specific `agent_id`
- `agent_id` maps back to its owning `task_id`
- message stamping, buffering, status, and check-message retrieval follow that `agent_id -> task_id` chain

## Data Model Changes

### Replace Task Slots With Task Agents

Current task-level fields that encode singleton ownership are no longer valid as product truth:

- `lead_provider`
- `coder_provider`
- `lead_session_id`
- `current_coder_session_id`

The new primary model should instead be:

- `Task`
- `TaskAgent`
- `SessionHandle` belonging to a specific `agent_id`
- runtime summaries keyed by `agent_id`

### Compatibility Strategy

During migration, the old singleton task fields may remain as compatibility-only reads while the data migrates, but they must not remain the authoritative source for routing or UI state.

The target steady state is:

- product truth: `task_agents[]`
- derived/temporary compatibility fields only if strictly necessary

## UI Model

### Task Pane

The task pane remains the main task-management surface, but its content changes:

- task list / selection
- `New Task`
- `Edit Task`
- internal agent list for the active task
- `Add Agent`

The standalone `Agents` sidebar remains removed.

### New Task

- `New Task` may create an empty task
- no agent is required at creation time
- the created task is immediately selectable and usable as a container

### Edit Task / Add Agent

Agent management moves inside task scope:

- `Edit Task` edits task metadata and task-owned agents
- `Add Agent` creates a new task agent record
- editing an existing task agent changes that agent’s role/config/provider/runtime preferences

### Agent List

Each task shows its `task_agents[]` list inside the task pane.

The list supports:

- multiple agents sharing the same role
- drag-and-drop ordering
- per-agent edit/remove actions

### Reply Input Target Picker

The target picker is no longer a fixed `auto | lead | coder`.

It becomes:

- `auto`
- one item per distinct role currently present in the active task

Default target behavior:

- prefer `lead` if present
- otherwise choose the first role from the ordered role list

## Runtime And Status UI

The frontend should stop modeling runtime state as “Claude panel + Codex panel are the whole task”.

Instead:

- runtime summaries belong to task agents
- provider connection labels belong to task agents
- task-level summaries are derived from the task agent list

The UI may still group by provider for presentation, but the underlying state must remain agent-based.

## Migration Plan

Existing tasks created under the slot model need deterministic migration.

For each persisted task:

1. inspect existing singleton fields (`lead_provider`, `coder_provider`, `lead_session_id`, `current_coder_session_id`)
2. generate zero, one, or two initial `TaskAgent` records from that data
3. assign generated internal ids
4. associate any persisted session/runtime metadata with the new `agent_id`
5. mark legacy fields as compatibility-only after migration

Important:

- migration must preserve old task history as much as possible
- migration must be idempotent
- migrated tasks must not duplicate agents on repeated loads

## Error Handling

### No Matching Target

If a send targets a role that currently has no agents:

- do not silently reroute
- surface a clear task-scoped error to the sender

### Partial Broadcast Failure

If a broadcast role matches multiple agents and one launch/send path fails:

- keep successful deliveries intact
- record which agent ids failed
- surface per-agent failure details in task-scoped diagnostics

## Testing Strategy

### Backend

- migration from legacy singleton task fields to `task_agents[]`
- routing by `agent_id`
- role-broadcast delivery to multiple same-role agents
- `auto` target fallback: `lead` first, else first ordered role
- no-role match error path
- provider-originated message stamping by `agent_id`

### Frontend

- empty task creation with zero agents
- adding multiple agents with the same role
- dynamic role target picker from task agent list
- default target selection (`lead`, else first role)
- drag-and-drop order affecting role fallback order
- active-task UI no longer depending on singleton provider slots

## Rollout Notes

- This is a replacement plan, not a continuation of the single-slot model.
- Previously-landed single-workspace UX and UI error-log work remain useful and should be preserved where compatible.
- The task/agent identity model itself should move directly to the final `task_agents[]` design, not through another temporary slot-based layer.
