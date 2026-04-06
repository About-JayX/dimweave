# Task Binding and Cross-Role Routing Remediation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the false `coder offline, buffered` behavior by correctly binding Claude launches into the active task graph, routing cross-role task messages against the target session instead of the sender session, and surfacing accurate buffer reasons in the UI logs.

**Architecture:** Keep the existing task-centric daemon model. The fix has three parts: (1) make Claude launch/update the normalized task/session graph the same way Codex already does, (2) teach routing to resolve the target session from `task_id + to-role` when an inter-role message carries the sender's `session_id`, and (3) preserve the real buffer reason so the GUI stops misreporting every task-binding failure as "offline".

**Tech Stack:** Rust, tokio, Tauri, existing daemon task graph/routing modules, cargo test

---

## Root Cause Summary

### Confirmed cause 1: normal Claude launch does not bind into the active task graph

Evidence:
- `src-tauri/src/daemon/codex/mod.rs` calls `provider::codex::register_on_launch(...)` for a fresh Codex launch.
- `src-tauri/src/daemon/mod.rs` normal `LaunchClaudeSdk` flow does **not** call `provider::claude::register_on_launch(...)`.
- `provider::claude::register_on_launch(...)` is currently only reached from the attach/resume path, not the everyday "Connect Claude" path.

Impact:
- Claude lead/coder sessions can be online but absent from `task_graph`.
- `TaskPanel` shows no Claude-bound session for the active task.
- `current_coder_session_id` may remain empty even while Claude is online as `coder`.

### Confirmed cause 2: inter-role messages are stamped with the sender session but routing interprets that field as the target session

Evidence:
- `src-tauri/src/daemon/control/handler.rs` and `src-tauri/src/daemon/codex/session_event.rs` both call `stamp_message_context(role, &mut msg)`.
- `src-tauri/src/daemon/state_task_flow.rs::stamp_message_context` writes `session_id` based on the **sender** role.
- `src-tauri/src/daemon/state_task_flow.rs::bound_session_for_message` prefers that same `session_id` when checking whether the **target** agent matches the message.

Impact:
- `lead -> coder` gets matched against the lead session.
- `coder -> lead` gets matched against the coder session.
- Cross-provider task routing buffers even when both agents are online and correctly assigned.

### Confirmed cause 3: buffered logging hides the actual failure mode

Evidence:
- `src-tauri/src/daemon/routing_display.rs` prints `[Route] {to} offline, buffered` for every buffered message.
- It does not distinguish review gate vs missing task session vs stale provider-session mismatch vs true offline state.

Impact:
- Operators see "offline" even when the real problem is task/session binding.
- Debugging gets pushed in the wrong direction.

## Scope Constraints

- Do not change `bridge/**`.
- Do not redesign the whole `BridgeMessage` wire schema in this fix wave.
- Preserve current task graph semantics: `provider history`, `live provider connection`, and `normalized task session` remain distinct.
- Keep the fix focused on current message routing and task binding; no Telegram work in this plan.

## File Map

### New files

- `src-tauri/src/daemon/launch_task_sync.rs`
  - helper functions to register provider launches against the active task and emit task-context refreshes without growing `daemon/mod.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
  - pure helper logic for deciding which session should be used when validating the target agent for a task-bound message

### Modified files

- `src-tauri/src/daemon/mod.rs`
  - invoke Claude task-binding sync after successful SDK launch
- `src-tauri/src/daemon/provider/claude_tests.rs`
  - add regression coverage for the normal Claude launch path binding into the task
- `src-tauri/src/daemon/state_task_flow.rs`
  - stop treating a sender-stamped `session_id` as the authoritative target session when the target role differs
- `src-tauri/src/daemon/routing.rs`
  - preserve structured buffer reasons through route outcome metadata
- `src-tauri/src/daemon/routing_display.rs`
  - map real buffer reasons to accurate system-log messages
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
  - add cross-role regression tests for `lead -> coder` and `coder -> lead`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
  - add active-task target-selection coverage when Claude is the coder and must be recognized as task-bound
- `CLAUDE.md`
  - document the corrected task-binding behavior for normal provider launches
- `UPDATE.md`
  - record the root cause and the routing/task-binding fix

---

### Task 1: Reproduce the failure with targeted regression tests

**Files:**
- Create: `src-tauri/src/daemon/routing_target_session.rs`
- Modify: `src-tauri/src/daemon/routing_shared_role_tests.rs`
- Modify: `src-tauri/src/daemon/routing_user_target_tests.rs`

- [x] **Step 1: Write failing tests for the current broken routing semantics**

Add a `lead -> coder` regression to `src-tauri/src/daemon/routing_shared_role_tests.rs`:

```rust
#[tokio::test]
async fn lead_to_coder_uses_target_coder_session_not_sender_lead_session() {
    let state = seeded_task_with_codex_lead_and_claude_coder().await;
    let msg = BridgeMessage {
        id: "lead-to-coder-1".into(),
        from: "lead".into(),
        display_source: Some("codex".into()),
        to: "coder".into(),
        content: "implement task 1".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::Done),
        task_id: Some("task_1".into()),
        session_id: Some("lead_session".into()),
        sender_agent_id: Some("codex".into()),
        attachments: None,
    };

    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Delivered));
}
```

Add the mirror `coder -> lead` regression:

```rust
#[tokio::test]
async fn coder_to_lead_uses_target_lead_session_not_sender_coder_session() {
    let state = seeded_task_with_codex_lead_and_claude_coder().await;
    let msg = BridgeMessage {
        id: "coder-to-lead-1".into(),
        from: "coder".into(),
        display_source: Some("claude".into()),
        to: "lead".into(),
        content: "task 1 complete".into(),
        timestamp: 2,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::Done),
        task_id: Some("task_1".into()),
        session_id: Some("coder_session".into()),
        sender_agent_id: Some("claude".into()),
        attachments: None,
    };

    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Delivered));
}
```

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml lead_to_coder_uses_target_coder_session_not_sender_lead_session -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml coder_to_lead_uses_target_lead_session_not_sender_coder_session -- --nocapture
```

Expected: FAIL on current code because routing uses the sender session for target validation.

- [x] **Step 2: Extract pure target-session resolution helpers**

Create `src-tauri/src/daemon/routing_target_session.rs` with helpers shaped like:

```rust
pub fn target_role_session<'a>(
    task_graph: &'a TaskGraphStore,
    task_id: &str,
    to_role: &str,
) -> Option<&'a SessionHandle>

pub fn resolve_target_bound_session<'a>(
    task_graph: &'a TaskGraphStore,
    message: &BridgeMessage,
) -> Option<&'a SessionHandle>
```

Resolution rules:
- If `message.to` is not `lead` or `coder`, return `None`.
- If `message.session_id` exists and belongs to the same role as `message.to`, honor it.
- If `message.session_id` exists but belongs to the opposite role, treat it as sender context and fall back to the task’s bound target-role session.
- If only `task_id` exists, resolve via `task_id + to-role`.

- [x] **Step 3: Verify the regression tests now pass**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml lead_to_coder_uses_target_coder_session_not_sender_lead_session -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml coder_to_lead_uses_target_lead_session_not_sender_coder_session -- --nocapture
```

Expected: PASS.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/daemon/routing_target_session.rs \
  src-tauri/src/daemon/routing_shared_role_tests.rs \
  src-tauri/src/daemon/routing_user_target_tests.rs
git commit -m "test: reproduce cross-role task routing regressions"
```

### Task 2: Bind normal Claude launches into the active task and refresh TaskPanel state

**Files:**
- Create: `src-tauri/src/daemon/launch_task_sync.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/provider/claude_tests.rs`

- [x] **Step 1: Write a failing test for the missing Claude task binding**

Add a helper-level regression test:

```rust
#[test]
fn sync_claude_launch_sets_current_coder_session_for_active_task() {
    let mut state = DaemonState::new();
    let task = state.create_and_select_task("/ws", "Task");

    sync_claude_launch_into_active_task(
        &mut state,
        "coder",
        "/ws",
        "claude_session_1",
        "/tmp/.claude/projects/-ws/claude_session_1.jsonl",
    );

    let task = state.task_graph.get_task(&task.task_id).unwrap();
    assert!(task.current_coder_session_id.is_some());
}
```

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml sync_claude_launch_sets_current_coder_session_for_active_task -- --nocapture
```

Expected: FAIL until the new sync helper is wired into the normal Claude launch path.

- [x] **Step 2: Implement a launch-sync helper instead of growing `daemon/mod.rs`**

Create `src-tauri/src/daemon/launch_task_sync.rs`:

```rust
pub fn sync_claude_launch_into_active_task(
    state: &mut DaemonState,
    role_id: &str,
    cwd: &str,
    session_id: &str,
    transcript_path: &str,
) -> Option<String> {
    crate::daemon::provider::claude::register_on_launch(
        state,
        role_id,
        cwd,
        session_id,
        transcript_path,
    );
    state.active_task_id.clone()
}
```

- [x] **Step 3: Call the helper from the successful `LaunchClaudeSdk` path**

Update `src-tauri/src/daemon/mod.rs` so the success branch mirrors Codex:

```rust
Ok(handle) => {
    let transcript_path =
        crate::daemon::provider::claude::default_transcript_path(&cwd, &session_id)?
            .to_string_lossy()
            .to_string();
    let task_id = {
        let mut daemon = state.write().await;
        crate::daemon::launch_task_sync::sync_claude_launch_into_active_task(
            &mut daemon,
            &role_id,
            &cwd,
            &session_id,
            &transcript_path,
        )
    };
    claude_sdk_handle = Some(handle);
    if let Some(task_id) = task_id {
        emit_task_context_events(&state, &app, &task_id).await;
    }
    let _ = reply.send(Ok(()));
}
```

- [x] **Step 4: Verify Claude binding behavior**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml register_on_launch_captures_transcript_path -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml sync_claude_launch_sets_current_coder_session_for_active_task -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/daemon/launch_task_sync.rs \
  src-tauri/src/daemon/mod.rs \
  src-tauri/src/daemon/provider/claude_tests.rs
git commit -m "fix: bind claude launches into the active task graph"
```

### Task 3: Make task-bound routing validate against the target session

**Files:**
- Modify: `src-tauri/src/daemon/state_task_flow.rs`
- Modify: `src-tauri/src/daemon/routing.rs`

- [x] **Step 1: Replace the current target-session lookup with the new helper**

Update the routing lookup path in `src-tauri/src/daemon/state_task_flow.rs`:

```rust
fn bound_session_for_message<'a>(
    &'a self,
    message: &BridgeMessage,
) -> Option<&'a SessionHandle> {
    crate::daemon::routing_target_session::resolve_target_bound_session(
        &self.task_graph,
        message,
    )
}
```

- [x] **Step 2: Keep sender stamping for provenance, but stop letting it override the target role**

Do **not** remove `stamp_message_context(...)`; it is still useful for:
- preserving `task_id`
- preserving sender-side provenance
- downstream artifact/review logic

The fix is only to reinterpret `session_id` safely during routing.

- [x] **Step 3: Verify cross-role routing end-to-end**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml routing_shared_role_tests -- --nocapture
```

Expected: PASS with the new regressions included.

- [x] **Step 4: Commit**

```bash
git add src-tauri/src/daemon/state_task_flow.rs \
  src-tauri/src/daemon/routing.rs
git commit -m "fix: route task messages against target sessions"
```

### Task 4: Report real buffer reasons instead of always saying offline

**Files:**
- Modify: `src-tauri/src/daemon/routing.rs`
- Modify: `src-tauri/src/daemon/routing_display.rs`

- [x] **Step 1: Add failing tests for buffered reason classification**

Add a test shaped like:

```rust
#[test]
fn task_session_mismatch_buffer_message_is_not_reported_as_offline() {
    let text = buffered_route_message("coder", Some("task_session_mismatch"));
    assert!(text.contains("task session mismatch"));
    assert!(!text.contains("offline"));
}
```

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml task_session_mismatch_buffer_message_is_not_reported_as_offline -- --nocapture
```

Expected: FAIL because buffered logs still always say offline.

- [x] **Step 2: Preserve buffer reasons in route metadata**

Extend `RouteOutcome` in `src-tauri/src/daemon/routing.rs`:

```rust
struct RouteOutcome {
    result: RouteResult,
    emit_claude_thinking: bool,
    buffer_reason: Option<&'static str>,
}
```

Populate at least these reasons:
- `review_gate`
- `target_session_missing`
- `task_session_mismatch`
- `target_agent_offline`

- [x] **Step 3: Map reasons to accurate system logs**

Update `emit_route_side_effects(...)` in `src-tauri/src/daemon/routing_display.rs` so buffered logs become:

```rust
match buffer_reason {
    Some("review_gate") => format!("[Route] {} blocked by review gate", msg.to),
    Some("target_session_missing") => format!("[Route] {} has no bound session in the active task, buffered", msg.to),
    Some("task_session_mismatch") => format!("[Route] {} does not match the active task session, buffered", msg.to),
    _ => format!("[Route] {} offline, buffered", msg.to),
}
```

- [x] **Step 4: Verify diagnostics**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml routing_behavior_tests -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/daemon/routing.rs \
  src-tauri/src/daemon/routing_display.rs
git commit -m "fix: report accurate buffered route reasons"
```

### Task 5: Document the corrected task-binding model and run focused verification

**Files:**
- Modify: `CLAUDE.md`
- Modify: `UPDATE.md`

- [x] **Step 1: Update architecture docs**

Required doc updates:
- note that normal Claude SDK launches now register normalized task sessions the same way Codex launches do
- document that `session_id` on inter-role messages is sender provenance, not blindly the target binding
- record that buffered route logs now distinguish task-binding failures from true offline failures

- [x] **Step 2: Run the focused verification suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml register_on_launch_captures_transcript_path -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml sync_claude_launch_sets_current_coder_session_for_active_task -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml routing_shared_role_tests -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml routing_behavior_tests -- --nocapture
```

Expected: PASS.

- [x] **Step 3: Manual verification checklist**

1. Start from a selected workspace so an active task exists.
2. Connect Codex as `lead`.
3. Connect Claude as `coder`.
4. Open the Task sidebar and confirm both lead and coder sessions appear.
5. Send a message that makes `lead` delegate work to `coder`.
6. Confirm the message is delivered immediately, not buffered as offline.
7. Have `coder` send a terminal result back to `lead`.
8. Confirm review-state progression still occurs.
9. Disconnect `coder` and repeat; now the system log should truthfully report offline buffering.

- [x] **Step 4: Commit**

```bash
git add CLAUDE.md UPDATE.md
git commit -m "docs: record task binding and routing remediation"
```

## Implementation Verification (2026-04-06)

All 5 tasks confirmed implemented and landed on main. Commit trail:

| Commit | Summary | Plan Tasks Covered |
|--------|---------|-------------------|
| `62e60d45` | fix: enforce task-bound routing and workspace starts | Task 3 (partial), frontend guards |
| `0b1c29c6` | fix: repair active-task routing and claude launch binding | Task 1, 2, 3, 4 |
| `4c948241` | fix: fan out auto routing and clear stale task sessions | Task 1 (additional tests), frontend view-model |
| `48caac42` | fix: refresh task context after disconnect | Related: disconnect event emission |

Code-level verification:
- `routing_target_session.rs` — `target_role_session()` + `resolve_target_bound_session()` created
- `launch_task_sync.rs` — `sync_claude_launch_into_active_task()` created, wired into `LaunchClaudeSdk`
- `state_task_flow.rs::bound_session_for_message` — delegates to `resolve_target_bound_session`
- `routing.rs::RouteOutcome` — has `buffer_reason: Option<&'static str>` field
- `routing_display.rs::buffered_route_message` — maps `review_gate` / `target_session_missing` / `task_session_mismatch` to accurate logs
- Regression tests: `lead_to_coder_uses_target_coder_session`, `coder_to_lead_uses_target_lead_session`, `sync_claude_launch_sets_current_coder_session_for_active_task`

Runtime verification passed (2026-04-06): connected Codex lead + Claude coder, cross-role messages delivered immediately, buffer reasons display accurately.

## Final Acceptance Criteria

- [x] Normal Claude SDK launches create/update the normalized session binding for the active task.
- [x] `TaskPanel` shows the correct lead/coder session tree when Codex is lead and Claude is coder.
- [x] `lead -> coder` and `coder -> lead` task messages deliver when both agents are online and bound to the active task.
- [x] Buffered logs distinguish missing task binding / session mismatch from true offline buffering.
- [x] Existing review-gate behavior still works.
