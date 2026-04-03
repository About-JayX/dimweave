# Unified Task/Session Architecture Design

## Summary

Dimweave will become a single product centered on a **Task Graph** rather than raw provider sessions. A user creates one task, the system opens exactly one **lead parent session** for planning and review, and the lead session creates one or more **coder child sessions** for implementation. Claude and Codex remain separate providers underneath, but both are normalized behind a shared session/task model so the UI presents one coherent workflow.

## Product Goal

- Users manage **tasks**, not provider-specific chats.
- Conversations stay **separated by role**:
  - `lead`: research, planning, review, final synthesis
  - `coder`: implementation only
- Every coder todo is executed under a strict review loop before the next todo proceeds.

## Architecture Decision

Adopt **a unified task tree with provider adapters**:

- **Task** is the top-level product object.
- **Session** is a normalized runtime/persisted record.
- **Lead session** is the parent session for a task.
- **Coder session** is a child session linked to the lead session.
- **Provider adapter** maps internal session operations onto:
  - Codex `thread/start`, `thread/list`, `thread/resume`, `thread/fork`, `thread/archive`
  - Claude PTY/session metadata and local transcript/session history

This avoids forcing Claude into Codex-native semantics while still presenting a single product model.

## Core Domain Model

### Task

- `task_id`
- `workspace_root`
- `title`
- `status` (`draft | planning | implementing | reviewing | done | error`)
- `lead_session_id`
- `current_coder_session_id`
- `created_at`
- `updated_at`

### SessionHandle

- `session_id` (internal UUID-like stable id)
- `task_id`
- `parent_session_id` (`null` for lead)
- `provider` (`claude | codex`)
- `role` (`lead | coder`)
- `external_session_id` (`Claude session_id` or `Codex thread.id`)
- `status`
- `cwd`
- `title`
- `created_at`
- `updated_at`

### Artifact

- `artifact_id`
- `task_id`
- `session_id`
- `kind` (`research | plan | review | diff | verification | summary`)
- `title`
- `content_ref`
- `created_at`

## Storage Strategy

Persist task/session/artifact metadata locally in an Dimweave-owned store instead of treating provider rollout files as the source of truth.

### Persisted sources

- **Dimweave registry**
  - tasks
  - normalized sessions
  - artifacts
  - session relationships
- **Provider-native history**
  - Codex rollout/thread data
  - Claude transcript/session data

### Rule

Dimweave owns orchestration metadata; providers own their raw conversation history.

## Runtime Components

### 1. Task Registry

Responsible for creating, loading, updating, and listing tasks.

### 2. Session Registry

Responsible for:

- registering normalized sessions
- linking parent/child sessions
- resolving provider session ids
- exposing unified history to the UI

### 3. Provider Adapters

#### Codex Adapter

Must support:

- create lead/coder sessions
- list historical threads
- resume selected thread
- fork thread into a child coder session when needed
- archive old threads

#### Claude Adapter

Must support:

- capture session_id and transcript path from live session output/hooks
- build a local history index
- resume/continue a selected Claude session
- map transcript metadata into normalized session records

### 4. Task Orchestrator

Controls the product workflow:

1. user creates/selects task
2. lead session receives request
3. lead produces research + plan artifacts
4. orchestrator creates coder child session per executable work unit
5. coder returns implementation status/artifacts
6. lead runs strict review and verification
7. orchestrator advances task state

### 5. Review Pipeline

Every coder todo must pass:

1. **implementation completion gate**
2. **spec/plan compliance review**
3. **code quality review**
4. **verification evidence**
5. **lead approval**

No next todo starts until the current todo is accepted.

## UX Model

### Primary screen

The main product object is a **Task Detail** page with:

- task header/status
- lead session panel
- coder session tree
- artifact timeline
- unified history picker

### History picker

Users can:

- list lead sessions
- expand a task to see child coder sessions
- resume a lead session
- resume or inspect a coder session
- see provider badges only as metadata, not as the main organizing concept

## Routing Rules

- User messages for planning/review route only to the **lead** parent session.
- Implementation work routes only to the active **coder** child session.
- Coder cannot directly become the final user-facing summarizer.
- Lead is the only session allowed to:
  - create plans
  - approve/reject coder output
  - generate final summary

## Implementation Constraints

- Reuse current Tauri daemon as the orchestration core.
- Do not break existing Claude/Codex routing while introducing task/session normalization.
- Keep source files under the repository’s 200-line soft cap by splitting new Rust/TS modules aggressively.

## Required Backend Additions

- `task_registry`
- `session_registry`
- `artifact_store`
- `provider adapter` layer
- daemon events for task/session tree synchronization
- commands for history listing, task creation, session resume, coder task launch, artifact listing

## Required Frontend Additions

- task store/view-model
- task list and task detail UI
- session tree UI
- unified history picker modal/panel
- artifact timeline UI
- strict review status visualization per coder todo

## Migration Strategy

### Phase 1

Introduce normalized task/session models and persistence without removing current direct Claude/Codex launch paths.

### Phase 2

Route new UX through the task orchestrator while preserving compatibility shims.

### Phase 3

Make task-driven workflow the primary UX and reduce provider-first controls to advanced settings.

## Acceptance Criteria

- User can create a task and see one lead parent session.
- Lead can create at least one coder child session.
- Codex history can be listed and resumed through the unified session model.
- Claude session metadata is captured and resumable through the same UI model.
- UI shows task → lead → coder relationships clearly.
- Each coder todo records review state before the next todo starts.
- Final output to the user is task-centric, not provider-centric.
