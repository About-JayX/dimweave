# Task Card Agent Status Design

## Summary

Task cards currently render each agent pill with a hard-coded gray dot, even when the corresponding task agent is already connected. The daemon already owns runtime truth, but the task card does not consume agent-scoped online status from daemon-driven task context data.

This design makes task-card pills reflect daemon-owned per-agent runtime status directly.

## Product Goal

- Show each task-card agent pill with its own live status dot.
- Use green for online and gray for offline.
- Preserve current pill text and ordering.
- Support multiple same-provider agents in one task without collapsing them into one status.

## Scope

### Included

- Daemon/task-context DTO support for per-agent runtime status
- Frontend store hydration and event handling for per-task agent statuses
- Task-card pill rendering that maps by `agentId`
- Focused tests for daemon-backed multi-agent status display
- Plan and CM documentation

### Excluded

- Changing connection semantics
- Changing task deletion behavior
- Changing task-card layout, copy, or pill ordering
- Adding new provider summary semantics for unrelated surfaces

## Root Cause

The issue is not that status is missing from daemon. The issue is that the task card is not wired to consume daemon-owned agent-level status.

Today:

- `TaskHeader` renders each pill dot with a fixed gray class
- frontend task state only keeps `taskAgents[]` plus task-level provider summary
- current provider summary is `lead/coder` oriented and is not sufficient for multiple same-provider agents

So even after a task agent is connected, the pill cannot independently reflect that agent's live state.

## Product Decision

### Status ownership

Runtime status remains daemon-owned.

Frontend must not infer online/offline from provider family or role-level summary. It should consume task-scoped agent status that daemon emits.

### Granularity

Status is tracked per saved task agent:

- keyed by `agentId`
- scoped by `taskId`
- at minimum includes `online: boolean`

This is enough to correctly color each pill while preserving support for multiple same-provider agents.

## Architecture

### Daemon → frontend data path

Extend the task-context payloads so they include per-agent runtime status, for example:

- `task_snapshot.taskAgentStatuses`
- `task_agent_statuses_changed` event payload with `{ taskId, statuses }`

The exact naming can follow the existing DTO/event style, but the key point is the same:

- daemon derives status from task-scoped runtime slots
- frontend stores that result by `taskId`
- `TaskHeader` reads it by `agentId`

### Frontend rendering

`TaskHeader` should continue rendering pills from persisted `taskAgents[]`, but the dot color should be looked up from the daemon-backed runtime-status map:

- matching `agentId` and `online === true` → green dot
- otherwise → gray dot

This keeps persisted identity and runtime identity aligned.

## Behavior Changes

### Task card pills

- Connected task agents display green dots.
- Disconnected task agents display gray dots.
- Two same-provider agents can display different dots if one is online and the other is offline.

## File Plan

### Modified files

- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/events.ts`
- `src/stores/task-store/index.ts`
- `tests/task-store.test.ts`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`

## Testing Strategy

- Add backend/frontend DTO and reducer coverage for per-agent status hydration.
- Add task-card tests proving:
  - online agent pills render green dots
  - offline agent pills render gray dots
  - multiple same-provider agents can render different status dots independently
- Re-run focused frontend tests, Rust check if needed, and full frontend build.

## Acceptance Criteria

- Task card no longer uses a hard-coded gray dot for every agent pill.
- Each pill color is driven by daemon-owned per-agent status keyed by `agentId`.
- Multiple same-provider agents on one task can display different statuses independently.
- Focused verification commands pass.
