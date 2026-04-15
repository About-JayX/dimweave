# Daemon Dispatch Chain Fixes Design

> **Status:** Accepted

## Summary

The current daemon dispatch chain still has three real production bugs plus one verification blocker:

- live launch/connect paths still reject same-role cross-provider sessions even though role-broadcast routing is an accepted feature
- task-targeted sends to a missing role still buffer forever instead of failing clearly
- global online-agent snapshots still collapse all Claude/Codex sessions to one row each
- targeted daemon test commands cannot currently compile because pre-existing `TaskSnapshot` test fixtures were not updated after `agent_runtime_statuses` was added

The repair must preserve the newer `task_agents[]` / `agent_id` model instead of falling back to provider-singleton behavior.

## Product Goal

- Allow official Claude/Codex launch/connect paths to enter the same-role live states that routing already supports.
- Make missing-role sends fail clearly when a task has agents but none match the requested role.
- Make `get_online_agents()` / daemon online snapshots report the real online agent instances, not provider singletons.
- Restore a green daemon verification baseline before touching routing behavior so the dispatch fixes can be proven instead of guessed.

## Scope

### Included

- daemon launch/connect conflict handling
- task-targeted broadcast resolution for missing roles
- daemon online-agent snapshot generation and its callers
- focused daemon test and fixture updates needed to verify the repaired behavior
- design / plan / CM documentation for this follow-up

### Excluded

- no task-pane / task-card UI work
- no task setup dialog changes
- no delete / connect-flow rework
- no new routing model beyond the already-accepted `task_agents[]` + `agent_id` architecture

## Root Causes

### 1. Same-role live states are still blocked in official launch/connect paths

`task_agents[]` and routing tests already treat same-role broadcasts as a supported feature, but the live connect chain still rejects them through `online_role_conflict(...)`.

That mismatch means the production launch path cannot actually enter states that the routing layer is supposed to handle.

### 2. Missing task role still looks like a temporary offline state

When a message is task-scoped and the task already has agents, but none match the requested role, routing still returns `NeedBuffer`.

That hides a real configuration error as a fake “maybe later” delivery condition.

### 3. Global online-agent snapshots still speak in provider singletons

`online_agents_snapshot()` still returns at most one `claude` and one `codex`, even though runtime ownership is now keyed by concrete `agent_id`.

Any caller depending on that snapshot for dispatch visibility or status reporting receives lossy information whenever multiple same-provider agents are online.

### 4. Verification baseline is red for a pre-existing reason

After `TaskSnapshot.agent_runtime_statuses` was added, some test-only `TaskSnapshot` initializers were not updated.

That is not part of the dispatch bug itself, but it currently prevents the daemon routing test commands from compiling, so it must be fixed first before the dispatch changes can be verified cleanly.

## Product Decisions

### Same-role live sessions remain supported

Same-role cross-provider sessions are a feature, not an edge case.

The daemon must stop rejecting them in official launch/connect paths.

### Task-scoped missing-role sends fail clearly

Once a task already owns explicit agents, asking that task for a non-existent role is a deterministic configuration error, not a buffering case.

### `agent_id` is the online identity boundary

The authoritative online view is per concrete `agent_id`.

Provider-family summaries can remain compatibility helpers, but they must not be the source of truth for online-agent snapshots.

## Architecture

### Baseline unblock

First repair the pre-existing `TaskSnapshot` test fixtures so daemon-focused verification commands compile again.

This is a narrow prerequisite task, not a behavioral change.

### Launch/connect conflict repair

Remove the same-role conflict gate from the official Claude/Codex launch/connect chain while keeping existing explicit-`agent_id` no-op semantics intact.

The runtime should continue preventing duplicate launches of the same concrete agent, but it must not reject a different agent just because another provider already owns the same role.

### Missing-role routing repair

For task-scoped sends:

- if the task has no agents yet, buffering remains valid
- if the task has agents and at least one matches the role, broadcast as usual
- if the task has agents and none match the role, fail clearly instead of buffering

### Online snapshot repair

Build `online_agents_snapshot()` from the real online task-agent runtime slots so every online `agent_id` is represented.

Keep the existing output shape (`agent_id`, `role`, `model_source`) but stop collapsing multiple sessions into provider singletons.

## Acceptance Criteria

- official launch/connect paths no longer reject same-role cross-provider live sessions
- task-scoped sends to a missing role fail clearly once the task owns explicit agents
- global online-agent snapshots enumerate real online `agent_id` instances
- daemon routing verification commands compile and pass from a clean baseline

## Outcome

- daemon verification baseline was restored by updating the remaining `TaskSnapshot` test fixtures to include `agent_runtime_statuses`
- singleton-era same-role launch/connect conflict gates were removed from the official Claude/Codex live paths while preserving explicit-`agent_id` duplicate no-op guards
- task-scoped missing-role sends now drop clearly when a task already owns explicit agents, while zero-agent tasks still buffer
- `online_agents_snapshot()` now enumerates real online task-agent instances and avoids phantom legacy singleton rows when compatibility mirrors are populated
