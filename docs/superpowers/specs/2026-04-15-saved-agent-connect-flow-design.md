# Saved Agent Connect Flow Design

## Summary

The current `Save & Connect` flow still has one critical gap:

- it launches from the draft payload instead of the saved agent list

That means newly added agents and newly created tasks do not have stable persisted `agentId` values at connect time. When the frontend launches with missing agent ids, daemon falls back to creating fresh task-agent identities. The visible result is that:

- a newly added agent may fail to connect as the intended saved agent
- a newly created task may connect through duplicate freshly-created agents instead of the saved list

## Product Goal

- Make `Save & Connect` always launch from the persisted agent list, not the draft payload.
- Ensure new agents use their newly created real `agentId`.
- Ensure existing agents keep their existing `agentId`.
- Preserve daemon-owned online/no-op decisions per explicit `agentId`.

## Scope

### Included

- Create-mode and edit-mode post-save connect flow in `TaskPanel`
- Focused dialog interaction tests for create/edit connect payload identity
- Documentation and CM records for this follow-up

### Excluded

- No change to delete confirmation
- No change to task-card status dots
- No change to daemon online/no-op semantics already implemented
- No layout or styling changes

## Root Cause

`TaskPanel` currently uses the dialog payload itself as the connect source.

That payload is correct for:

- provider
- role
- model / effort
- history action

But it is not authoritative for `agentId` when:

- the agent is newly added during edit mode
- the task itself is newly created in create mode

In those cases, persistence creates the real task-agent rows first, and only then do stable `agentId` values exist.

## Product Decision

### Connect source of truth

The source of truth for connect is the saved task-agent list after persistence completes.

That means:

- create mode: create task → add task agents → use returned created agents as connect targets
- edit mode: persist all add/update/remove/reorder changes → read the resulting saved task-agent list → use that list as connect targets

### Identity mapping

For each saved agent:

- existing agent keeps its persisted `agentId`
- newly added agent uses the `agentId` returned from `addTaskAgent(...)`

The frontend then combines:

- saved agent identity (`agentId`, `provider`, `role`)
- draft connect config (`model`, `effort`, `historyAction`)

into the final launch requests.

## Architecture

### Create mode

After `createTask(...)` and `addTaskAgent(...)` finish, build a saved-agent list from the returned `TaskAgentInfo[]`.

Do not launch from the original draft `payload.agents`.

### Edit mode

After edit persistence finishes:

- remove deleted agents
- update existing agents
- add new agents and collect returned `agentId`
- reorder

Then build the final saved-agent list and connect from that list.

This guarantees that every connect target has a real persisted `agentId`.

### Config pairing

The frontend should keep pairing connect config by the logical saved agent entry it just persisted, not by provider-family collapsing.

Same-provider agents remain independent.

## Acceptance Criteria

- `Save & Connect` no longer launches from the raw draft payload.
- Newly added agents connect using their persisted `agentId`.
- Newly created tasks connect using the saved task-agent list.
- No duplicate task-agent identities are created as a side effect of create/edit connect.
- Focused verification commands pass.
