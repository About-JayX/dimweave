# Provider History Resume Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix two regressions: (1) resuming a known historical Claude/Codex provider session from the provider panels must switch into the already-normalized task/session instead of staying on the currently selected task; (2) after Codex disconnects, Session Tree must stop showing the disconnected paused coder session as if it were still active.

**Architecture:** Keep the existing normalized `task -> session` model. Add a launch-sync path that, after provider launch succeeds, checks whether the provider `external_session_id` is already known in the task graph; if yes, resume that normalized session/task, otherwise register a new session on the active task. Separately, make the frontend Session Tree render only active task-context sessions so paused/disconnected coder sessions disappear immediately after disconnect.

**Tech Stack:** Rust, TypeScript, Tauri, cargo test, bun test

---

## Root Cause Summary

1. **Known provider history resumed from provider panels does not reactivate the normalized task/session.**
   - `LaunchCodex` resume path only calls `register_on_launch(...)` when the thread is unknown.
   - If the thread is already in the task graph, the provider reconnects but the daemon does **not** call `resume_session(existing_session_id)`, so `active_task_id` and task pointers stay on the wrong task.
   - Claude’s launch-sync helper also assumes “active task wins”; when a known historical Claude session is resumed, it can point the current task at a session owned by another task instead of switching to that task.

2. **Session Tree shows disconnected Codex entries because paused descendants are still traversed.**
   - On disconnect, the daemon marks the Codex session `Paused` and clears `current_coder_session_id`.
   - `buildSessionTreeRows(...)` still walks child sessions beneath the active lead session regardless of paused status, so the disconnected coder remains visible.

## File Map

### Modified files

- `src-tauri/src/daemon/launch_task_sync.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/provider/claude_tests.rs`
- `src-tauri/src/daemon/provider/codex_tests.rs`
- `src/components/TaskPanel/view-model.ts`
- `tests/task-panel-view-model.test.ts`

---

### Task 1: Restore the correct normalized task/session when a known provider history entry is resumed

**Files:**
- Modify: `src-tauri/src/daemon/launch_task_sync.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/provider/claude_tests.rs`
- Modify: `src-tauri/src/daemon/provider/codex_tests.rs`

- [ ] **Step 1: Add failing Codex regression coverage**

Add a test in `src-tauri/src/daemon/provider/codex_tests.rs` that creates:
- `task_current`
- `task_history`
- a Codex normalized session under `task_history` with `external_session_id = "thread_hist_1"`
- `active_task_id = task_current`

Then call the new launch-sync helper (or the extracted Codex sync function) with:

```rust
sync_codex_launch_into_task(&mut state, "coder", "/ws", "thread_hist_1");
```

Expected assertions:

```rust
assert_eq!(state.active_task_id.as_deref(), Some(task_history.task_id.as_str()));
assert_eq!(
    state
        .task_graph
        .get_task(&task_history.task_id)
        .and_then(|t| t.current_coder_session_id.as_deref()),
    Some(history_session.session_id.as_str())
);
assert!(
    state
        .task_graph
        .get_task(&task_current.task_id)
        .and_then(|t| t.current_coder_session_id.as_deref())
        .is_none()
);
```

- [ ] **Step 2: Add failing Claude regression coverage**

Add a test in `src-tauri/src/daemon/provider/claude_tests.rs` that creates:
- `task_current`
- `task_history`
- a Claude normalized session under `task_history` with `external_session_id = "claude_hist_1"`
- `active_task_id = task_current`

Then call:

```rust
sync_claude_launch_into_active_task(
    &mut state,
    "lead",
    "/ws",
    "claude_hist_1",
    "/tmp/.claude/projects/-ws/claude_hist_1.jsonl",
);
```

Expected assertions:

```rust
assert_eq!(state.active_task_id.as_deref(), Some(task_history.task_id.as_str()));
assert_eq!(
    state
        .task_graph
        .get_task(&task_history.task_id)
        .and_then(|t| t.lead_session_id.as_deref()),
    Some(history_session.session_id.as_str())
);
assert!(
    state
        .task_graph
        .get_task(&task_current.task_id)
        .and_then(|t| t.lead_session_id.as_deref())
        .is_none()
);
```

- [ ] **Step 3: Implement launch-sync behavior**

In `src-tauri/src/daemon/launch_task_sync.rs`:
- Keep the “unknown external session → register_on_launch” behavior.
- Add “known external session → resume the normalized session” behavior.
- Claude sync must also refresh transcript path/status for the known session before `resume_session(...)`.
- Codex sync must do the same task/session reactivation for known thread ids.

Implementation rule:
- If `find_session_by_external_id(provider, external_id)` returns a session:
  - update provider-specific metadata if needed
  - call `state.resume_session(existing_session_id)`
  - return that task id
- Else:
  - register on the active task
  - return `state.active_task_id.clone()`

- [ ] **Step 4: Wire the new sync helper(s) into the actual launch paths**

In `src-tauri/src/daemon/mod.rs`:
- Replace the ad-hoc Codex resume-path logic with the shared sync helper.
- Keep the provider launch order unchanged: first provider resume succeeds, then daemon task/session sync runs.
- Update the Claude launch path to use the new “known external session resumes normalized task” semantics.

- [ ] **Step 5: Verify targeted Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml sync_claude_launch -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml codex -- --nocapture
```

Expected: the new known-history resume tests pass, and existing provider tests still pass.

---

### Task 2: Remove disconnected paused coder sessions from Session Tree

**Files:**
- Modify: `src/components/TaskPanel/view-model.ts`
- Modify: `tests/task-panel-view-model.test.ts`

- [ ] **Step 1: Add failing frontend regression coverage**

Add a test that reproduces the exact broken case:

```ts
const rows = buildSessionTreeRows(
  [
    makeSession("sess_lead", "lead", { status: "active" }),
    makeSession("sess_coder_paused", "coder", {
      parentSessionId: "sess_lead",
      status: "paused",
    }),
  ],
  makeTask({ leadSessionId: "sess_lead", currentCoderSessionId: null }),
);

expect(rows.map((row) => row.sessionId)).toEqual(["sess_lead"]);
```

- [ ] **Step 2: Update Session Tree derivation**

In `src/components/TaskPanel/view-model.ts`:
- derive rows from task-context sessions that are still `status === "active"`
- do not traverse paused/disconnected children beneath the active lead
- if no active sessions remain, return an empty list

Do **not** change artifact timeline behavior or provider-history behavior in this task.

- [ ] **Step 3: Update obsolete expectations**

Update the existing “falls back to all top-level sessions when task has no bound session pointers” test to match the new semantics:
- if all sessions are paused/disconnected, Session Tree should now be empty

- [ ] **Step 4: Verify targeted frontend tests**

Run:

```bash
bun test tests/task-panel-view-model.test.ts
```

Expected: all Session Tree tests pass, including the new paused-coder regression.

---

### Task 3: Final regression pass

- [ ] **Step 1: Run combined verification**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
bun test
```

Expected: all tests pass.

- [ ] **Step 2: Manual behavior checklist**

Verify these flows after the automated tests:
- Resuming a known Claude history entry from the Claude provider panel switches to the correct normalized task.
- Resuming the matching known Codex history entry from the Codex provider panel switches to the same normalized task/session context.
- Disconnecting Codex removes the coder row from Session Tree immediately.
- Provider history still lists the disconnected Codex thread for later resume.

---

## Commit Record

| Commit | Scope | Verification |
| --- | --- | --- |
| `97d0e2fb` | Implemented the committed portion of this plan: known-history launch sync for Claude/Codex, regression coverage in provider tests, and Session Tree filtering so paused/disconnected coder children disappear from active task context. The broader provider-panel convergence follow-up landed in the companion plan `2026-04-07-historical-provider-session-convergence.md`. | `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`; `bun test` |
