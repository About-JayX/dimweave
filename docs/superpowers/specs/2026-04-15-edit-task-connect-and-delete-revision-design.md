# Edit Task Connect And Delete Revision Design

> **Status:** Accepted

## Summary

The previous edit-connect-delete change closed the obvious missing actions, but real app testing exposed two product-level failures:

- deleting a task used `window.confirm(...)`, which did not produce a reliable in-app secondary confirmation flow in the Tauri runtime
- `Save & Connect` launched by provider family instead of binding to existing task agents, so edit-mode connect could create duplicate same-provider agents instead of attaching sessions to the saved agent list

This revision keeps the already-accepted deletion and edit-connect goals, but replaces the weak confirmation path with a project-native React dialog and changes edit-mode connection from provider-driven launch to agent-bound launch.

## Product Goal

- Replace browser-native delete confirmation with a project-native React confirmation dialog.
- Keep delete entry points in both the task card and the edit dialog.
- Make `Save & Connect` operate on saved task agents, not provider family shortcuts.
- Support multiple same-provider agents on one task as separate independent sessions.
- Only connect currently offline agents; leave already-online agents unchanged.

## Scope

### Included

- React confirmation dialog for task deletion
- Wiring both delete entry points to the shared confirmation flow
- Agent-bound edit-mode `Save & Connect`
- Backend launch command updates needed to bind launches to existing `agentId`
- Focused tests for confirmation flow and multi-agent edit-connect behavior
- Plan and CM documentation

### Excluded

- Redesigning task card or dialog layout
- Changing trigger styling again
- Changing task deletion semantics already accepted in the previous follow-up
- Changing create-mode connect semantics unless needed for shared launch plumbing

## Root Cause

### Delete confirmation

The prior implementation used `window.confirm(...)` in `TaskPanel/index.tsx`.

That path is outside the project’s normal React surface model and does not produce a dependable product-grade confirmation flow in the running desktop app.

### Edit-mode connect

The prior implementation reused create-mode launch helpers too literally:

- it reduced the saved agent list to one Claude and one Codex entry
- it launched without a stable existing `agentId`

In the backend, missing explicit `agentId` means the daemon creates a fresh task agent identity for launch. That is correct for new launches, but wrong for edit-mode reconnect of already-saved agents.

So with multiple same-provider agents, edit-mode `Save & Connect` could:

- ignore additional same-provider agents
- or create a duplicate new agent instead of attaching to the saved one

## Product Decision

### Confirmation UI

Task deletion uses one shared React `ConfirmDialog` component:

- same styling language as existing custom modal surfaces
- explicit title, body copy, confirm, and cancel actions
- destructive action styling

Both delete entry points open the same confirmation dialog.

### Agent-bound edit connect

`Save & Connect` in edit mode behaves like this:

1. save the edited agent list first
2. inspect the saved task agents after persistence
3. for each saved task agent:
   - if it is already online, do nothing
   - if it is offline, connect that specific agent by its `agentId`

This means same-provider agents are valid and independent:

- two `codex` agents mean two separate Codex sessions
- two `claude` agents mean two separate Claude sessions

Provider family is not the identity. `agentId` is the identity.

### Online/offline rule

Edit-mode `Save & Connect` only launches offline agents.

Already-online agents remain connected and unchanged.

## Architecture

### Shared confirmation state

`TaskPanel` owns one confirmation state object:

- which task is being deleted
- whether the request came from task card or edit dialog

The card and dialog only trigger this state. The actual delete action happens only when the shared confirmation dialog is confirmed.

### Agent-bound launch plumbing

The backend launch path needs explicit existing-agent support:

- Claude launch command must accept `agentId`
- Codex launch command must accept `agentId`
- daemon launch handlers must reuse that existing `agentId` instead of calling `create_agent_id(...)`

That preserves task-agent identity across edit-mode reconnects.

### Frontend edit connect flow

After edit persistence completes, `TaskPanel` should derive the saved task agents from store state and pair them with the submitted config payload.

The connect pass should iterate over the saved agent list, not collapse by provider.

The frontend should not make the final online/offline decision for a specific saved agent from role-level summary data. It may submit the saved `agentId` targets, but daemon must remain authoritative for whether that exact agent is already online.

### Daemon online/no-op decision

For explicit `agentId` launches:

- daemon checks whether that specific task agent is already online
- if yes, it returns success/no-op without creating a new agent or restarting the session
- if no, it launches and binds the runtime to that existing `agentId`

This is the only way to support multiple same-provider agents cleanly, because role-level or provider-level frontend summaries cannot distinguish one `codex` task agent from another.

## Behavior Changes

### Delete flow

- Clicking delete on a task card opens a React confirmation dialog.
- Clicking delete inside `Edit Task` opens the same confirmation dialog.
- Confirming performs the already-accepted disconnect-before-delete sequence.

### Edit Task

- `Save` still only persists edits.
- `Save & Connect` persists edits, then connects only offline saved agents.
- Multiple same-provider agents are treated as distinct sessions, not as duplicates or one-provider-one-session shortcuts.

## File Plan

### Modified files

- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.interaction.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/ui/confirm-dialog.tsx`
- `src/components/ui/confirm-dialog.test.tsx`
- `src/components/AgentStatus/codex-launch-config.ts`
- `src/components/ClaudePanel/launch-request.ts`
- `src-tauri/src/commands.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`

## Testing Strategy

- Add focused React tests for the confirmation dialog component.
- Extend task-panel tests so task-card and dialog delete entry points open the shared confirmation dialog instead of deleting immediately.
- Extend dialog interaction tests so edit-mode `Save & Connect` proves offline-only launch intent.
- Add backend or launch-level checks proving explicit `agentId` launch paths reuse existing task agents instead of creating duplicates, and that already-online explicit agents become daemon-side no-ops.
- Re-run focused frontend tests, Rust verification for the changed launch path, and the full frontend build.

## Acceptance Criteria

- No task deletion path uses `window.confirm(...)`.
- Task card and edit dialog both use the same React confirmation dialog before deleting.
- `Save & Connect` in edit mode only connects offline saved agents, with daemon making the final online/no-op decision per explicit `agentId`.
- Multiple same-provider agents remain distinct task-bound sessions rather than collapsing to one provider instance or duplicating agents.
- Focused verification commands pass.
