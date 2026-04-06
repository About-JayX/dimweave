# Disconnect Session Recording Fix

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the bug where natural provider disconnects (WS close, process exit, reconnect failure) do not emit `session_tree_changed` to the frontend, causing the "Active sessions" UI to show stale data.

**Architecture:** Two changes: (1) Rust `_if_current` functions return `Option<String>` (task_id) instead of `bool`, and a public `emit_task_context_events` helper is extracted so submodules can call it; (2) frontend `buildSessionTreeRows` falls back to showing all top-level sessions when task pointers are cleared.

**Tech Stack:** Rust, tokio, Tauri events, TypeScript, Bun test

---

## Root Cause Summary

- `clear_codex_session_if_current()` and `invalidate_claude_sdk_session_if_current()` return `bool`, discarding the `task_id` from `clear_provider_connection()`.
- Their callers in `codex/session.rs`, `codex/runtime.rs`, `claude_sdk/mod.rs`, `claude_sdk/reconnect.rs` cannot emit `session_tree_changed` without the task_id.
- `emit_task_context_events` is private to `daemon/mod.rs`, structurally unreachable from submodules.
- Frontend `buildSessionTreeRows` only visits sessions pointed to by `task.leadSessionId` / `task.currentCoderSessionId`; after disconnect clears those pointers, paused sessions vanish.

## Scope Constraints

- Do not change `bridge/**`.
- Do not redesign `BridgeMessage` wire schema.
- Do not change the explicit stop paths (`stop_codex_session`, `stop_claude_sdk_session`) — they already work.

## File Map

### Modified files

- `src-tauri/src/daemon/state_runtime.rs`
  - change `clear_codex_session_if_current` return type to `Option<String>`
  - change `invalidate_claude_sdk_session_if_current` return type to `Option<String>`
- `src-tauri/src/daemon/state_tests.rs`
  - update existing test + add new regression tests
- `src-tauri/src/daemon/gui_task.rs`
  - add public `emit_task_context_events` async helper
- `src-tauri/src/daemon/mod.rs`
  - delegate to `gui_task::emit_task_context_events`
- `src-tauri/src/daemon/codex/session.rs`
  - emit task context events on natural disconnect
- `src-tauri/src/daemon/codex/runtime.rs`
  - emit task context events on health monitor disconnect
- `src-tauri/src/daemon/claude_sdk/mod.rs`
  - emit task context events on process exit
- `src-tauri/src/daemon/claude_sdk/reconnect.rs`
  - emit task context events on reconnect failure
- `src/components/TaskPanel/view-model.ts`
  - fallback to show all top-level sessions when task pointers are empty
- `tests/task-panel-view-model.test.ts`
  - update existing test + add regression test

---

### Task 1: Change `_if_current` return types and add regression tests

**Files:**
- Modify: `src-tauri/src/daemon/state_runtime.rs:48-55,131-137`
- Modify: `src-tauri/src/daemon/state_tests.rs`

- [ ] **Step 1: Write failing test for Codex `clear_codex_session_if_current` returning task_id**

Add to `src-tauri/src/daemon/state_tests.rs`:

```rust
#[test]
fn clear_codex_session_if_current_returns_task_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.active_task_id = Some(task.task_id.clone());
    let session = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Codex,
        role: crate::daemon::task_graph::types::SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
    });
    s.task_graph
        .set_coder_session(&task.task_id, &session.session_id);
    s.task_graph
        .set_external_session_id(&session.session_id, "thread_1");
    s.set_provider_connection(
        "codex",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Codex,
            external_session_id: "thread_1".into(),
            cwd: "/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::New,
        },
    );
    let epoch = s.begin_codex_launch();

    let result = s.clear_codex_session_if_current(epoch);
    assert_eq!(result, Some(task.task_id));
}
```

- [ ] **Step 2: Write failing test for Claude `invalidate_claude_sdk_session_if_current` returning task_id**

Add to `src-tauri/src/daemon/state_tests.rs`:

```rust
#[test]
fn invalidate_claude_sdk_session_if_current_returns_task_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.active_task_id = Some(task.task_id.clone());
    let session = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Claude,
        role: crate::daemon::task_graph::types::SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    s.task_graph
        .set_lead_session(&task.task_id, &session.session_id);
    s.task_graph
        .set_external_session_id(&session.session_id, "claude_sess_1");
    s.set_provider_connection(
        "claude",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Claude,
            external_session_id: "claude_sess_1".into(),
            cwd: "/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::New,
        },
    );
    let epoch = s.begin_claude_sdk_launch("nonce".into());

    let result = s.invalidate_claude_sdk_session_if_current(epoch);
    assert_eq!(result, Some(task.task_id));
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test --manifest-path src-tauri/Cargo.toml clear_codex_session_if_current_returns_task_id -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml invalidate_claude_sdk_session_if_current_returns_task_id -- --nocapture
```

Expected: FAIL — current return type is `bool`, cannot compare with `Option<String>`.

- [ ] **Step 4: Change `clear_codex_session_if_current` to return `Option<String>`**

In `src-tauri/src/daemon/state_runtime.rs`, replace lines 48-55:

```rust
    pub fn clear_codex_session_if_current(&mut self, epoch: u64) -> Option<String> {
        if self.codex_session_epoch != epoch {
            return None;
        }
        self.codex_inject_tx = None;
        self.clear_provider_connection("codex")
    }
```

- [ ] **Step 5: Change `invalidate_claude_sdk_session_if_current` to return `Option<String>`**

In `src-tauri/src/daemon/state_runtime.rs`, replace lines 131-137:

```rust
    pub fn invalidate_claude_sdk_session_if_current(&mut self, epoch: u64) -> Option<String> {
        if self.claude_sdk_session_epoch != epoch {
            return None;
        }
        self.invalidate_claude_sdk_session()
    }
```

- [ ] **Step 6: Fix the existing `stale_codex_session_cleanup_cannot_clear_new_session` test**

The existing test at `state_tests.rs` uses `assert!(!s.clear_codex_session_if_current(stale_epoch))` and `assert!(s.clear_codex_session_if_current(current_epoch))`. Update to match the new `Option<String>` return type:

```rust
#[test]
fn stale_codex_session_cleanup_cannot_clear_new_session() {
    let mut s = DaemonState::new();
    let stale_epoch = s.begin_codex_launch();
    let current_epoch = s.begin_codex_launch();
    let (current_tx, _current_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);

    assert!(s.attach_codex_session_if_current(current_epoch, current_tx));
    assert!(s.clear_codex_session_if_current(stale_epoch).is_none());
    assert!(s.codex_inject_tx.is_some());
    assert!(s.clear_codex_session_if_current(current_epoch).is_some_or_none_ok());
    assert!(s.codex_inject_tx.is_none());
}
```

Wait — the issue is this test has no provider connection set up, so `clear_provider_connection` will return `None` even for the current epoch. The test is checking the epoch guard, not the task_id. Fix as:

```rust
#[test]
fn stale_codex_session_cleanup_cannot_clear_new_session() {
    let mut s = DaemonState::new();
    let stale_epoch = s.begin_codex_launch();
    let current_epoch = s.begin_codex_launch();
    let (current_tx, _current_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);

    assert!(s.attach_codex_session_if_current(current_epoch, current_tx));
    assert!(s.clear_codex_session_if_current(stale_epoch).is_none());
    assert!(s.codex_inject_tx.is_some());
    // current epoch passes guard; no provider connection → returns None but inject_tx is cleared
    let _ = s.clear_codex_session_if_current(current_epoch);
    assert!(s.codex_inject_tx.is_none());
}
```

- [ ] **Step 7: Fix callers in `claude_sdk/runtime.rs` that use `bool` pattern**

In `src-tauri/src/daemon/claude_sdk/runtime.rs`, the `spawn_runtime` function at lines 83-86 uses:

```rust
state.write().await.invalidate_claude_sdk_session_if_current(epoch);
```

This discards the return value (previously `bool`, now `Option<String>`). This is fine — it's a cleanup path during failed launch, not a disconnect path. No change needed here.

- [ ] **Step 8: Run tests to verify they pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml clear_codex_session_if_current_returns_task_id -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml invalidate_claude_sdk_session_if_current_returns_task_id -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml stale_codex_session_cleanup_cannot_clear_new_session -- --nocapture
```

Expected: PASS.

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/daemon/state_runtime.rs src-tauri/src/daemon/state_tests.rs
git commit -m "fix: return task_id from _if_current disconnect functions"
```

---

### Task 2: Extract `emit_task_context_events` into `gui_task` so submodules can use it

**Files:**
- Modify: `src-tauri/src/daemon/gui_task.rs`
- Modify: `src-tauri/src/daemon/mod.rs:276-296`

- [ ] **Step 1: Add the public async helper to `gui_task.rs`**

Append before the `#[cfg(test)]` block in `src-tauri/src/daemon/gui_task.rs`:

```rust
/// Emit a full task-context sync to the frontend for the given task.
///
/// This is the single entry point for notifying the UI about session/artifact
/// changes. It reads the current task graph state and emits all relevant
/// events (task_updated, active_task_changed, session_tree_changed,
/// artifacts_changed).
pub async fn emit_task_context_events(
    state: &crate::daemon::SharedState,
    app: &AppHandle,
    task_id: &str,
) {
    let s = state.read().await;
    let sess: Vec<_> = s
        .task_graph
        .sessions_for_task(task_id)
        .into_iter()
        .cloned()
        .collect();
    let arts: Vec<_> = s
        .task_graph
        .artifacts_for_task(task_id)
        .into_iter()
        .cloned()
        .collect();
    let events = build_task_context_events(s.task_graph.get_task(task_id), task_id, &sess, &arts);
    drop(s);
    for event in events {
        event.emit(app);
    }
}
```

- [ ] **Step 2: Delegate the private function in `mod.rs` to the new public one**

Replace the private `emit_task_context_events` in `src-tauri/src/daemon/mod.rs` (lines 275-296) with:

```rust
/// Emit a full task context sync for the selected task.
async fn emit_task_context_events(state: &SharedState, app: &AppHandle, task_id: &str) {
    gui_task::emit_task_context_events(state, app, task_id).await;
}
```

- [ ] **Step 3: Verify compilation and all tests pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture 2>&1 | tail -5
```

Expected: all tests pass, no compile errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/daemon/gui_task.rs src-tauri/src/daemon/mod.rs
git commit -m "refactor: extract emit_task_context_events into gui_task for submodule access"
```

---

### Task 3: Emit task context events from Codex natural disconnect paths

**Files:**
- Modify: `src-tauri/src/daemon/codex/session.rs:152-159`
- Modify: `src-tauri/src/daemon/codex/runtime.rs:61-73`

- [ ] **Step 1: Fix `codex/session.rs` — emit events when WS session loop ends**

Replace lines 152-159 in `src-tauri/src/daemon/codex/session.rs`:

```rust
    let task_id = state
        .write()
        .await
        .clear_codex_session_if_current(session_epoch);
    if task_id.is_some() {
        gui::emit_agent_status(app, "codex", false, None, None);
        gui::emit_system_log(app, "info", "[Codex] session ended");
    }
    if let Some(task_id) = task_id {
        crate::daemon::gui_task::emit_task_context_events(state, app, &task_id).await;
    }
```

- [ ] **Step 2: Fix `codex/runtime.rs` — emit events when health monitor detects process exit**

Replace lines 61-72 in `src-tauri/src/daemon/codex/runtime.rs`:

```rust
                    Ok(Some(status)) => {
                        eprintln!("[Codex] health_monitor: process exited with status={status}");
                        cancel.cancel();
                        let task_id = {
                            let mut daemon = state.write().await;
                            daemon.clear_codex_session_if_current(session_epoch)
                        };
                        if task_id.is_some() {
                            gui::emit_agent_status(&app, "codex", false, None, None);
                            gui::emit_system_log(
                                &app,
                                "warn",
                                &format!("[Codex] exited: {status}"),
                            );
                        }
                        if let Some(task_id) = task_id {
                            crate::daemon::gui_task::emit_task_context_events(
                                &state, &app, &task_id,
                            )
                            .await;
                        }
                        return;
                    }
```

- [ ] **Step 3: Verify compilation**

```bash
cargo check --manifest-path src-tauri/Cargo.toml 2>&1
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/daemon/codex/session.rs src-tauri/src/daemon/codex/runtime.rs
git commit -m "fix: emit session_tree_changed on codex natural disconnect"
```

---

### Task 4: Emit task context events from Claude natural disconnect paths

**Files:**
- Modify: `src-tauri/src/daemon/claude_sdk/mod.rs:80-101`
- Modify: `src-tauri/src/daemon/claude_sdk/reconnect.rs:84-92`

- [ ] **Step 1: Fix `claude_sdk/mod.rs` — emit events when process exit monitor fires**

Replace lines 80-101 in `src-tauri/src/daemon/claude_sdk/mod.rs`:

```rust
                _ = poll_child_exit(&current_child, true) => {
                    let task_id = monitor_state
                        .write()
                        .await
                        .invalidate_claude_sdk_session_if_current(current_epoch);
                    if task_id.is_none() {
                        return;
                    }
                    emit_runtime_health(
                        &monitor_state,
                        &monitor_app,
                        crate::daemon::types::RuntimeHealthLevel::Error,
                        format!("Claude runtime exited for role={monitor_role}"),
                    ).await;
                    gui::emit_agent_status(&monitor_app, "claude", false, None, None);
                    gui::emit_claude_stream(&monitor_app, gui::ClaudeStreamPayload::Done);
                    gui::emit_system_log(
                        &monitor_app,
                        "info",
                        &format!("[Claude SDK] process exited, role={monitor_role}"),
                    );
                    if let Some(task_id) = task_id {
                        crate::daemon::gui_task::emit_task_context_events(
                            &monitor_state, &monitor_app, &task_id,
                        ).await;
                    }
                    return;
                }
```

- [ ] **Step 2: Fix `claude_sdk/reconnect.rs` — emit events when all reconnect attempts fail**

Replace lines 84-91 in `src-tauri/src/daemon/claude_sdk/reconnect.rs`:

```rust
    let task_id = state
        .write()
        .await
        .invalidate_claude_sdk_session_if_current(*epoch);
    if task_id.is_some() {
        gui::emit_agent_status(app, "claude", false, None, None);
        gui::emit_claude_stream(app, gui::ClaudeStreamPayload::Reset);
    }
    if let Some(task_id) = task_id {
        crate::daemon::gui_task::emit_task_context_events(state, app, &task_id).await;
    }
```

- [ ] **Step 3: Verify compilation and all Rust tests pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/daemon/claude_sdk/mod.rs src-tauri/src/daemon/claude_sdk/reconnect.rs
git commit -m "fix: emit session_tree_changed on claude natural disconnect"
```

---

### Task 5: Fix frontend `buildSessionTreeRows` to show paused sessions after disconnect

**Files:**
- Modify: `src/components/TaskPanel/view-model.ts:123-128`
- Modify: `tests/task-panel-view-model.test.ts:120-127`

- [ ] **Step 1: Update the existing test to expect paused sessions to be visible**

In `tests/task-panel-view-model.test.ts`, replace the test at lines 120-127:

```typescript
  test("falls back to all top-level sessions when task has no bound session pointers", () => {
    const rows = buildSessionTreeRows(
      [
        makeSession("sess_lead", "lead", { status: "paused", parentSessionId: null }),
        makeSession("sess_coder_b", "coder", { status: "paused", parentSessionId: null }),
      ],
      makeTask({ leadSessionId: null, currentCoderSessionId: null }),
    );

    expect(rows.map((row) => row.sessionId)).toEqual([
      "sess_lead",
      "sess_coder_b",
    ]);
  });
```

- [ ] **Step 2: Run test to verify it fails**

```bash
bun test tests/task-panel-view-model.test.ts 2>&1
```

Expected: FAIL — current code returns `[]`.

- [ ] **Step 3: Fix `buildSessionTreeRows` to fall back when task pointers are empty**

In `src/components/TaskPanel/view-model.ts`, replace lines 123-128:

```typescript
  if (task) {
    for (const rootId of currentRootIds) {
      visit(rootId, 0);
    }
    if (rows.length > 0) {
      return rows;
    }
    // Task exists but lead/coder pointers are cleared (e.g. after disconnect).
    // Fall through to show all top-level sessions so paused sessions remain visible.
  }
```

- [ ] **Step 4: Run test to verify it passes**

```bash
bun test tests/task-panel-view-model.test.ts 2>&1
```

Expected: all tests pass.

- [ ] **Step 5: Run TypeScript check**

```bash
bun x tsc --noEmit -p tsconfig.app.json
```

Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add src/components/TaskPanel/view-model.ts tests/task-panel-view-model.test.ts
git commit -m "fix: show paused sessions in TaskPanel after disconnect"
```

---

### Task 6: Full verification pass

- [ ] **Step 1: Run all Rust tests**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 2: Run all frontend tests**

```bash
bun test 2>&1
```

Expected: all tests pass.

- [ ] **Step 3: Run TypeScript check**

```bash
bun x tsc --noEmit -p tsconfig.app.json
```

Expected: no errors.

## Manual Verification (2026-04-06)

Runtime verification passed:
1. Started `bun run tauri dev`, app launched successfully
2. Connected Codex as lead, Connected Claude as coder
3. Confirmed "Active sessions" shows 2 sessions in TaskPanel
4. Disconnected an agent — verified the bug: session count stays stale, no `session_tree_changed` emitted on natural disconnect

Root cause confirmed at runtime: natural disconnect paths fire `agent_status(false)` but never `session_tree_changed`.

## Implementation (2026-04-06)

Commit trail:

| Commit | Summary |
|--------|---------|
| `2ac2335c` | fix: return task_id from _if_current disconnect functions |
| `1555fdf0` | refactor: extract emit_task_context_events into gui_task |
| `2f2a1af4` | fix: emit session_tree_changed on codex natural disconnect |
| `4d5ccf34` | fix: emit session_tree_changed on claude natural disconnect |
| `961bc640` | fix: show paused sessions in TaskPanel after disconnect |
| `6fafa0cd` | fix: decouple agent_status emission from task_id availability |

Code review finding addressed: `emit_agent_status`/`emit_system_log` guards were narrowed from "epoch matched" to "epoch matched + task binding exists". Fixed by checking epoch via public accessors (`codex_session_epoch()`, `claude_sdk_epoch()`) separately from task_id.

Test results: Rust 283 passed, Frontend 162 passed, 0 failures.

## Final Acceptance Criteria

- [x] `clear_codex_session_if_current` returns `Option<String>` containing the task_id when the epoch matches and a provider session exists.
- [x] `invalidate_claude_sdk_session_if_current` returns `Option<String>` containing the task_id when the epoch matches and a provider session exists.
- [x] Codex WS session loop exit (`codex/session.rs`) emits `session_tree_changed`.
- [x] Codex health monitor process exit (`codex/runtime.rs`) emits `session_tree_changed`.
- [x] Claude process exit monitor (`claude_sdk/mod.rs`) emits `session_tree_changed`.
- [x] Claude reconnect failure (`claude_sdk/reconnect.rs`) emits `session_tree_changed`.
- [x] `buildSessionTreeRows` shows paused sessions when task pointers are cleared.
- [x] All existing tests continue to pass.
- [x] agent_status emission decoupled from task_id — fires on epoch match regardless of task binding.
