# Unified Task/Session Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a task-centric product flow where one lead parent session plans and reviews while separate coder child sessions implement under strict review gates.

**Architecture:** Add a normalized task/session/artifact domain over the existing Tauri daemon, then adapt Codex and Claude into that domain via provider-specific adapters. Expose the new model to React through explicit commands and event payloads, then move the UI from provider-first controls toward task-first orchestration.

**Tech Stack:** Tauri 2, Rust async daemon, React 19, TypeScript, Zustand, Codex app-server, Claude Code PTY/session integration

---

## Planned File Structure

### Backend create

- `src-tauri/src/daemon/task_graph/mod.rs` - task graph module exports
- `src-tauri/src/daemon/task_graph/types.rs` - task/session/artifact domain types
- `src-tauri/src/daemon/task_graph/store.rs` - persistence and in-memory registry
- `src-tauri/src/daemon/task_graph/session_index.rs` - parent/child/session lookup helpers
- `src-tauri/src/daemon/task_graph/task_index.rs` - task lookup helpers
- `src-tauri/src/daemon/task_graph/artifact_index.rs` - artifact lookup helpers
- `src-tauri/src/daemon/provider/mod.rs` - provider adapter exports
- `src-tauri/src/daemon/provider/codex.rs` - Codex adapter list/resume/fork/archive helpers
- `src-tauri/src/daemon/provider/claude.rs` - Claude session capture/history/resume helpers
- `src-tauri/src/daemon/provider/shared.rs` - shared provider DTOs
- `src-tauri/src/daemon/orchestrator/mod.rs` - orchestrator exports
- `src-tauri/src/daemon/orchestrator/task_flow.rs` - lead/coder task orchestration
- `src-tauri/src/daemon/orchestrator/review_gate.rs` - strict todo review state machine
- `src-tauri/src/daemon/gui_task.rs` - UI event emitters for task/session/artifact state

### Backend modify

- `src-tauri/src/daemon/mod.rs` - wire new modules
- `src-tauri/src/daemon/state.rs` - hold task graph store and current task/session pointers
- `src-tauri/src/daemon/types.rs` - add task/session payload DTOs
- `src-tauri/src/daemon/routing.rs` - route by task/session, not only role
- `src-tauri/src/daemon/routing_user_input.rs` - send planning to lead and implementation to coder
- `src-tauri/src/daemon/session_manager.rs` - attach task/session metadata to Codex launch lifecycle
- `src-tauri/src/daemon/codex/session.rs` - persist returned thread id into normalized session records
- `src-tauri/src/daemon/codex/session_event.rs` - sync Codex thread status into task graph
- `src-tauri/src/claude_session/process.rs` - emit Claude session metadata into the registry
- `src-tauri/src/commands.rs` - add create/list/select/resume task-session commands
- `src-tauri/src/main.rs` - register new commands
- `src-tauri/src/mcp.rs` - expose any required Claude session metadata hooks/config

### Frontend create

- `src/stores/task-store/types.ts` - frontend task/session/artifact types
- `src/stores/task-store/index.ts` - Zustand task store
- `src/stores/task-store/events.ts` - task/session/artifact listener wiring
- `src/components/TaskPanel/index.tsx` - top-level task panel
- `src/components/TaskPanel/TaskList.tsx` - task list
- `src/components/TaskPanel/TaskDetail.tsx` - task detail shell
- `src/components/TaskPanel/SessionTree.tsx` - lead/coder parent-child tree
- `src/components/TaskPanel/ArtifactTimeline.tsx` - artifact history UI
- `src/components/TaskPanel/HistoryPicker.tsx` - unified history picker
- `src/components/TaskPanel/ReviewGateBadge.tsx` - strict review status UI

### Frontend modify

- `src/App.tsx` - mount task-centric layout
- `src/types.ts` - add task/session/artifact DTOs
- `src/stores/bridge-store/index.ts` - coordinate task actions with legacy bridge state
- `src/stores/bridge-store/listener-payloads.ts` - add task/session/artifact event payloads
- `src/components/AgentStatus/index.tsx` - show task-aware launch context
- `src/components/AgentStatus/CodexHeader.tsx` - show normalized session identity
- `src/components/MessagePanel/index.tsx` - filter/render by active task/session
- `src/components/ReplyInput.tsx` - route to lead or active coder by task context

### Tests

- `src-tauri/src/daemon/task_graph/tests.rs`
- `src-tauri/src/daemon/provider/codex_tests.rs`
- `src-tauri/src/daemon/provider/claude_tests.rs`
- `src-tauri/src/daemon/orchestrator/tests.rs`
- `tests/task-store.test.ts`
- `tests/task-panel-view-model.test.ts`

---

### Task 1: Build normalized task/session/artifact domain

**Files:**
- Create: `src-tauri/src/daemon/task_graph/mod.rs`
- Create: `src-tauri/src/daemon/task_graph/types.rs`
- Create: `src-tauri/src/daemon/task_graph/store.rs`
- Create: `src-tauri/src/daemon/task_graph/session_index.rs`
- Create: `src-tauri/src/daemon/task_graph/task_index.rs`
- Create: `src-tauri/src/daemon/task_graph/artifact_index.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/daemon/types.rs`
- Test: `src-tauri/src/daemon/task_graph/tests.rs`

- [ ] **Step 1: Write failing Rust tests for task/session/artifact creation, parent-child linking, and persistence round-trip**
- [ ] **Step 2: Run `cargo test task_graph --manifest-path src-tauri/Cargo.toml` and confirm failure**
- [ ] **Step 3: Implement the task graph types and store with focused modules under `src-tauri/src/daemon/task_graph/`**
- [ ] **Step 4: Add the store into daemon state without changing existing message routing behavior**
- [ ] **Step 5: Re-run `cargo test task_graph --manifest-path src-tauri/Cargo.toml` and confirm pass**
- [ ] **Step 6: Send diff summary to lead for strict review before any next todo**

### Task 2: Add Codex provider adapter with history, resume, fork, and archive support

**Files:**
- Create: `src-tauri/src/daemon/provider/mod.rs`
- Create: `src-tauri/src/daemon/provider/shared.rs`
- Create: `src-tauri/src/daemon/provider/codex.rs`
- Modify: `src-tauri/src/daemon/codex/session.rs`
- Modify: `src-tauri/src/daemon/codex/session_event.rs`
- Modify: `src-tauri/src/daemon/session_manager.rs`
- Test: `src-tauri/src/daemon/provider/codex_tests.rs`

- [ ] **Step 1: Write failing adapter tests for Codex thread registration, history listing DTO mapping, resume, fork, and archive flows**
- [ ] **Step 2: Run `cargo test codex_tests --manifest-path src-tauri/Cargo.toml` and confirm failure**
- [ ] **Step 3: Implement a Codex adapter that registers returned `thread.id` into the normalized session graph**
- [ ] **Step 4: Add RPC helpers for `thread/list`, `thread/resume`, `thread/fork`, and `thread/archive` with bounded error handling**
- [ ] **Step 5: Sync Codex thread status updates into task/session records and UI payloads**
- [ ] **Step 6: Re-run `cargo test codex_tests --manifest-path src-tauri/Cargo.toml` and confirm pass**
- [ ] **Step 7: Send diff summary to lead for strict review before any next todo**

### Task 3: Add Claude provider adapter with session capture, local history index, and resume entry

**Files:**
- Create: `src-tauri/src/daemon/provider/claude.rs`
- Modify: `src-tauri/src/claude_session/process.rs`
- Modify: `src-tauri/src/mcp.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Test: `src-tauri/src/daemon/provider/claude_tests.rs`

- [ ] **Step 1: Write failing tests for Claude session metadata capture, local history indexing, and normalized DTO mapping**
- [ ] **Step 2: Run `cargo test claude_tests --manifest-path src-tauri/Cargo.toml` and confirm failure**
- [ ] **Step 3: Capture `session_id` and transcript metadata from Claude runtime and register them in the task graph**
- [ ] **Step 4: Implement a local Claude history index that can list resumable sessions for a workspace**
- [ ] **Step 5: Add a resume entry path that reconnects a selected Claude session into the normalized model**
- [ ] **Step 6: Re-run `cargo test claude_tests --manifest-path src-tauri/Cargo.toml` and confirm pass**
- [ ] **Step 7: Send diff summary to lead for strict review before any next todo**

### Task 4: Build task orchestrator and strict review gate

**Files:**
- Create: `src-tauri/src/daemon/orchestrator/mod.rs`
- Create: `src-tauri/src/daemon/orchestrator/task_flow.rs`
- Create: `src-tauri/src/daemon/orchestrator/review_gate.rs`
- Modify: `src-tauri/src/daemon/routing.rs`
- Modify: `src-tauri/src/daemon/routing_user_input.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Test: `src-tauri/src/daemon/orchestrator/tests.rs`

- [ ] **Step 1: Write failing orchestrator tests for task creation, lead session assignment, coder child session creation, and review gating**
- [ ] **Step 2: Run `cargo test orchestrator --manifest-path src-tauri/Cargo.toml` and confirm failure**
- [ ] **Step 3: Implement task state transitions: `planning -> implementing -> reviewing -> done|error`**
- [ ] **Step 4: Enforce routing rules so planning/review messages go only to lead and implementation messages go only to the active coder**
- [ ] **Step 5: Implement a strict per-todo review gate that blocks the next coder todo until lead review passes**
- [ ] **Step 6: Re-run `cargo test orchestrator --manifest-path src-tauri/Cargo.toml` and confirm pass**
- [ ] **Step 7: Send diff summary to lead for strict review before any next todo**

### Task 5: Expose task/session/artifact commands and GUI events

**Files:**
- Create: `src-tauri/src/daemon/gui_task.rs`
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/daemon/types.rs`
- Modify: `src-tauri/src/daemon/state_snapshot.rs`

- [ ] **Step 1: Write failing tests for command payloads and task/session snapshot serialization**
- [ ] **Step 2: Run `cargo test commands --manifest-path src-tauri/Cargo.toml` and confirm failure where relevant**
- [ ] **Step 3: Add commands for create/list/select task, list session tree, list history, resume session, and record review verdict**
- [ ] **Step 4: Emit dedicated GUI events for task tree, active task, artifacts, and review gate changes**
- [ ] **Step 5: Re-run command/state serialization tests and confirm pass**
- [ ] **Step 6: Send diff summary to lead for strict review before any next todo**

### Task 6: Add frontend task store and task-centric UI shell

**Files:**
- Create: `src/stores/task-store/types.ts`
- Create: `src/stores/task-store/index.ts`
- Create: `src/stores/task-store/events.ts`
- Create: `src/components/TaskPanel/index.tsx`
- Create: `src/components/TaskPanel/TaskList.tsx`
- Create: `src/components/TaskPanel/TaskDetail.tsx`
- Modify: `src/App.tsx`
- Modify: `src/types.ts`
- Modify: `src/stores/bridge-store/listener-payloads.ts`
- Test: `tests/task-store.test.ts`

- [ ] **Step 1: Write failing frontend tests for task store hydration, active task selection, and event reduction**
- [ ] **Step 2: Run `bun test tests/task-store.test.ts` or project-equivalent frontend test command and confirm failure**
- [ ] **Step 3: Implement a dedicated task store that consumes backend task/session/artifact events**
- [ ] **Step 4: Mount the task-centric shell in `src/App.tsx` without removing legacy message functionality yet**
- [ ] **Step 5: Re-run the frontend task store tests and confirm pass**
- [ ] **Step 6: Send diff summary to lead for strict review before any next todo**

### Task 7: Add session tree, history picker, artifact timeline, and review status UI

**Files:**
- Create: `src/components/TaskPanel/SessionTree.tsx`
- Create: `src/components/TaskPanel/ArtifactTimeline.tsx`
- Create: `src/components/TaskPanel/HistoryPicker.tsx`
- Create: `src/components/TaskPanel/ReviewGateBadge.tsx`
- Modify: `src/components/AgentStatus/CodexHeader.tsx`
- Modify: `src/components/AgentStatus/index.tsx`
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/ReplyInput.tsx`
- Test: `tests/task-panel-view-model.test.ts`

- [ ] **Step 1: Write failing tests for session tree rendering logic, history picker grouping, and review gate badge states**
- [ ] **Step 2: Run `bun test tests/task-panel-view-model.test.ts` or project-equivalent frontend test command and confirm failure**
- [ ] **Step 3: Implement the session tree so lead is the parent and coder sessions are visible as children**
- [ ] **Step 4: Implement a unified history picker that can list Claude and Codex history through normalized session DTOs**
- [ ] **Step 5: Implement artifact timeline and review gate badges so each coder todo exposes review state**
- [ ] **Step 6: Re-run frontend view-model tests and confirm pass**
- [ ] **Step 7: Send diff summary to lead for strict review before any next todo**

### Task 8: Verification, docs, and product hardening

**Files:**
- Modify: `CLAUDE.md`
- Modify: `UPDATE.md`
- Modify: `docs/agents/codex-chain.md`
- Modify: `docs/agents/claude-chain.md`
- Modify: `docs/agentnexus-audit-summary.md`

- [ ] **Step 1: Add or update tests for the final integrated task workflow if gaps remain**
- [ ] **Step 2: Run `cargo test --manifest-path src-tauri/Cargo.toml`**
- [ ] **Step 3: Run `bun run build`**
- [ ] **Step 4: Validate the full workflow manually: create task -> lead plan -> create coder child -> review gate -> resume history**
- [ ] **Step 5: Update architecture and chain docs with root cause notes, implementation details, and verification evidence**
- [ ] **Step 6: Send final integrated diff summary to lead for superpowers deep review**

---

## Execution Protocol For Coder

- Work **one task at a time** in numerical order.
- Do **not** start the next checkbox group until lead approves the current one.
- After each task, send:
  - files changed
  - tests run
  - exact results
  - open concerns
- Treat each task handoff as a **mandatory strict review checkpoint**.
- If a task uncovers a plan gap, stop and escalate to lead instead of guessing.

## Strict Review Policy

For every task, lead will run:

1. spec-compliance review
2. code-quality review
3. verification review

Any Important issue blocks the next todo.
