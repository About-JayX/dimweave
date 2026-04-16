# Agent-Directed Routing Redesign

> **Status:** Proposed

## Summary

The daemon runtime is already keyed by concrete `agent_id`, but the message protocol is still keyed by role strings.

That split is now the main source of ambiguity in the core dispatch chain:

- messages know which concrete agent sent them (`sender_agent_id`)
- messages do **not** know which concrete agent should receive them
- provider output contracts still only express `user` / `lead` / `coder`
- bridge startup still collapses unknown roles to `lead`
- default reply flow still depends on role routing and global compatibility fields

This works when each role has at most one live agent, but it becomes fundamentally ambiguous when a task contains multiple same-role agents or multiple same-provider agents.

The redesign replaces role-string targeting with an explicit structured routing protocol.
The final merged protocol is still a hard cut, but the implementation will use a staged migration inside the feature branch so each task can compile and be verified independently.

## Product Goal

- make the dispatch chain natively support multiple same-role agents within one task
- make “who sent this” and “who should receive this” first-class protocol concepts
- preserve explicit role-broadcast as a feature
- make normal work reporting default to a concrete `agent_id`, not a role name
- rebuild the bridge / Claude / Codex / daemon contracts around one message model

## Scope

### Included

- daemon message DTO redesign
- bridge protocol redesign
- bridge runtime boundary redesign so structured targets survive end-to-end
- Claude/Codex provider output contract redesign
- daemon routing rewrite to consume structured targets
- default reply-target propagation for delegation/reporting
- backend side-effect and display paths that still consume legacy role-string `BridgeMessage` fields
- frontend bridge/message display contracts that still consume legacy role-string `BridgeMessage` fields
- comprehensive communication test coverage across bridge/provider/daemon boundaries

### Excluded

- task-pane visual redesign
- provider launch UI changes
- task persistence schema beyond what is needed to support message routing
- long-lived mixed-version compatibility between old and new bridge/daemon runtimes

## Current Protocol

### Current message fields

The current daemon `BridgeMessage` effectively carries:

- `id`
- `from`
- `display_source`
- `to`
- `content`
- `timestamp`
- `reply_to`
- `priority`
- `status`
- `task_id`
- `session_id`
- `sender_agent_id`
- `attachments`

### Current problems

1. `from` and `to` are role strings, not structured identities.
2. `sender_agent_id` exists, but there is no symmetric receiver-side `agent_id`.
3. Bridge reply tooling only allows `to in ["user","lead","coder"]`.
4. Codex structured output only allows `send_to in ["user","lead","coder"]`.
5. Bridge startup still coerces unknown roles to `lead`.
6. Routing still contains compatibility shortcuts using `claude_role` / `codex_role`.

## Final Protocol

## Field Model

### Removed fields

- `from`
- `display_source`
- `to`
- `sender_agent_id`

### Retained fields

- `id`
- `content`
- `timestamp`
- `reply_to`
- `priority`
- `status`
- `task_id`
- `session_id`
- `attachments`

### Added fields

- `source`
- `target`
- `reply_target`

## Transitional Execution Model

The **final merged state** of this redesign is:

- no `from`
- no `display_source`
- no `to`
- no `sender_agent_id`
- only structured `source`, `target`, and `reply_target`

However, Rust test commands compile the whole crate, so removing those fields in a DTO-only task would make the repository unverifiable before downstream producers and consumers migrate.

To keep the work auditable and testable, implementation will proceed in two layers:

1. introduce the new structured message types first
2. migrate bridge, provider, and daemon boundaries onto those new types
3. remove the legacy role-string message fields only after the whole call graph has moved

This is an execution strategy only. It does **not** change the final protocol described in this spec.

### Transitional shared types

During migration the codebase may temporarily contain:

- legacy `BridgeMessage` for still-unmigrated call sites
- new structured message types for the migration target

The final cleanup task removes the legacy role-string message shape.

## New Types

### `MessageSource`

```ts
type MessageSource =
  | { kind: "user" }
  | { kind: "system" }
  | {
      kind: "agent";
      agentId: string;
      role: string;
      provider: "claude" | "codex";
      displaySource?: string;
    };
```

### `MessageTarget`

```ts
type MessageTarget =
  | { kind: "user" }
  | { kind: "role"; role: string }
  | { kind: "agent"; agentId: string };
```

### Final `BridgeMessage`

```ts
type BridgeMessage = {
  id: string;
  source: MessageSource;
  target: MessageTarget;
  replyTarget?: MessageTarget;
  content: string;
  timestamp: number;
  replyTo?: string;
  priority?: string;
  status?: "in_progress" | "done" | "error";
  taskId?: string;
  sessionId?: string;
  attachments?: Attachment[];
};
```

## Semantics

### Target semantics

- `target.kind = "user"`: deliver to the user/UI
- `target.kind = "role"`: explicit role-broadcast / role-routing
- `target.kind = "agent"`: deliver to one exact concrete `agent_id`

### Reply semantics

- `reply_target` is the default report-back target for follow-up work
- delegation writes a concrete `reply_target`
- worker status/progress replies default to `reply_target` if one is present
- role broadcast remains explicit; it is no longer the default reporting path

### Role semantics

- role strings remain part of agent identity and broadcast behavior
- role strings are no longer the sole addressing primitive

## Example Messages

### User to all coder agents

```json
{
  "id": "msg_1",
  "source": { "kind": "user" },
  "target": { "kind": "role", "role": "coder" },
  "content": "Implement this task.",
  "timestamp": 1770000000000,
  "taskId": "task_1"
}
```

### Lead delegating to one concrete coder

```json
{
  "id": "msg_2",
  "source": {
    "kind": "agent",
    "agentId": "agent_lead_1",
    "role": "lead",
    "provider": "claude",
    "displaySource": "claude"
  },
  "target": { "kind": "agent", "agentId": "agent_coder_2" },
  "replyTarget": { "kind": "agent", "agentId": "agent_lead_1" },
  "content": "Review and implement the daemon fix.",
  "timestamp": 1770000000100,
  "taskId": "task_1",
  "sessionId": "session_9"
}
```

### Worker reporting back to the delegating lead

```json
{
  "id": "msg_3",
  "source": {
    "kind": "agent",
    "agentId": "agent_coder_2",
    "role": "coder",
    "provider": "codex",
    "displaySource": "codex"
  },
  "target": { "kind": "agent", "agentId": "agent_lead_1" },
  "content": "Task is implemented and verified.",
  "timestamp": 1770000000200,
  "status": "done",
  "taskId": "task_1",
  "sessionId": "session_10"
}
```

## Routing Rules

### Resolution priority

1. `target.kind = "agent"` resolves exactly one concrete agent
2. `target.kind = "role"` resolves all matching task agents in the task scope
3. `target.kind = "user"` routes to GUI/user

### Delivery rules

- `target.agentId` must be validated against the task’s `task_agents[]`
- agent-targeted sends fail clearly if the target agent does not belong to the stamped task
- role-targeted sends broadcast to every matching task agent
- role-targeted sends fail clearly when the task owns agents but none match the role
- zero-agent task buffering remains allowed only where product behavior explicitly requires it

### Reply rules

- when `replyTarget` exists and the provider emits a normal terminal reply without an explicit override, the daemon targets `replyTarget`
- explicit provider output may still override `target` deliberately
- `replyTarget` is not a UI-only hint; it is part of daemon routing truth

## Provider Contract Changes

## Bridge Tooling

The bridge `reply` tool must stop accepting `to: "lead" | "coder" | "user"` as its core schema.

Instead it should accept:

```json
{
  "target": {
    "kind": "user" | "role" | "agent",
    "role": "...",
    "agentId": "..."
  },
  "message": "...",
  "status": "in_progress|done|error"
}
```

## Bridge Runtime Boundary

It is not enough for the bridge to merely parse a structured `target`.

The bridge runtime boundary must carry the structured message shape end-to-end:

- bridge outbound replies must not down-convert back into legacy `from/to` message fields
- bridge inbound channel metadata must remain valid for arbitrary role strings
- removing startup role coercion and leaving hard-coded channel sender allowlists in place is not acceptable

## Codex Structured Output

Codex structured output must stop using `send_to`.

It should emit:

```json
{
  "message": "...",
  "target": { "kind": "agent", "agentId": "agent_lead_1" },
  "status": "done"
}
```

or

```json
{
  "message": "...",
  "target": { "kind": "role", "role": "coder" },
  "status": "in_progress"
}
```

## Claude Event Routing

Claude output handling must construct the same structured message shape as Codex.

Claude direct terminal fallback remains user-targeted, but structured agent replies must no longer be flattened into role-only `to` strings.

## Bridge Runtime Identity

Bridge startup must stop coercing unknown roles to `lead`.

Any role present in task configuration must be preserved as-is, because role is descriptive while `agent_id` is authoritative for routing.

## Daemon Architecture Changes

## Core message model

All daemon internals must route through the structured `source/target/reply_target` model.

There should be no long-lived compatibility path that treats `from/to` strings as the primary truth.

## Control handler

Inbound agent replies must be normalized into:

- concrete `source.agent_id`
- concrete `source.role`
- concrete `source.provider`
- structured `target`

If sender identity cannot be validated, the message should fail clearly instead of silently resolving to “the first online slot”.

## User-input routing

`auto` target resolution should produce a structured `MessageTarget`, not a role string that is later reinterpreted.

Task-scoped user input should stay task-first.

## Global compatibility fields

`claude_role` / `codex_role` may survive temporarily as runtime compatibility state, but they must not remain on the critical path for sender validation or final receiver selection.

## Migration Strategy

This redesign is a deliberate hard cut in the **final merged state**, not a long-lived compatibility patch.

That means:

- bridge and daemon must ultimately be upgraded together
- old `to/send_to` producers are not considered supported after the final cleanup task
- migration work still happens in tasks so each step remains compilable and verifiable

## Testing

## Communication test requirements

This redesign is not acceptable without a broad communication test matrix.

The implementation plan must include explicit tests for every boundary below.

### 1. DTO / serialization tests

- `BridgeMessage` serializes/deserializes the new `source`, `target`, and `replyTarget`
- invalid `target.kind` / missing required fields fail clearly
- agent-targeted and role-targeted messages round-trip cleanly

### 2. Bridge tool schema tests

- reply tool accepts `target.kind = "agent"` and `target.kind = "role"`
- invalid combinations are rejected
- old `to/send_to` payloads are rejected once the cut happens

### 3. Codex output parsing tests

- structured output with `target.agentId` routes to one agent
- structured output with `target.role` broadcasts
- malformed target objects fail clearly
- `replyTarget` propagation is preserved

### 4. Claude output handling tests

- direct SDK user replies still surface correctly
- structured Claude replies preserve concrete sender identity
- agent-targeted Claude replies route only to the intended `agent_id`

### 5. Daemon routing tests

- agent-targeted delivery reaches exactly one recipient
- role-targeted delivery reaches every matching same-role agent
- same-provider same-role agents can receive distinct targeted replies
- invalid `target.agentId` drops clearly
- cross-task `target.agentId` mismatch drops clearly

### 6. Default report-chain tests

- lead delegates to coder with `replyTarget = lead-agent-id`
- coder reply without explicit override goes back to that exact lead agent
- two leads delegating to two coders in one task do not cross-report

### 7. User-input tests

- user `target.role` still fans out correctly
- task-scoped auto target produces structured targets
- no stale global `claude_role/codex_role` shortcut can override task-scoped truth

### 8. Reconnect / resume tests

- resumed provider sessions preserve concrete sender identity
- reconnect does not collapse `replyTarget`
- multiple same-provider agents remain independently routable after reconnect

### 9. End-to-end communication tests

- user -> lead(role) -> coder(agent) -> lead(agent) -> user
- two same-role leads + two coders in one task maintain independent report chains
- explicit role broadcast still works after agent-directed routing is introduced

### 10. Headless live runtime tests

- run real daemon/provider communication scenarios directly through code-level entrypoints, not by clicking the frontend UI
- cover `Codex=lead / Claude=coder`
- cover `Claude=lead / Codex=coder`
- cover at least one multi-agent task with a same-role case

Because the currently exposed external interfaces do not provide a complete code-level orchestration surface for task creation, agent creation, and provider launch, these live scenarios require a dedicated headless validation harness in the daemon test layer.

That harness is part of the redesign deliverable, not an optional follow-up.

## Implementation sequencing note

Because the bridge and daemon currently compile against legacy `BridgeMessage` across many subsystems, the remaining implementation must separate:

1. provider-side parsing/building of the new structured target model
2. core routing-kernel activation of structured targets
3. reply-target propagation
4. final hard-cut cleanup of all legacy role-string message consumers

That sequencing is required so each task remains independently compilable and verifiable while the final merged result is still a complete hard cut.

In practice, the staged sequence now means:

1. introduce new structured message types
2. migrate bridge/provider/routing behavior onto them
3. convert backend consumers away from direct legacy field reads
4. only after backend, bridge wire, and frontend message consumers are all aligned, remove the legacy role-string fields from the shared contract itself

## Acceptance Criteria

- the protocol no longer relies on `from/to/send_to` role strings as primary routing truth
- normal work reporting can target one exact `agent_id`
- explicit role broadcast remains supported
- multiple same-role agents can coexist without report-chain ambiguity
- communication tests cover bridge, provider, daemon routing, and reconnect paths
