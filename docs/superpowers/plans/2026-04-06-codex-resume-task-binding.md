# Codex Resume Task Binding Fix

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the bug where selecting an existing Codex session (resume) does not bind it into the active task graph, while Claude resume correctly does.

**Architecture:** Add `register_on_launch` call to the Codex resume success path in `daemon/mod.rs`, mirroring the pattern already used by Claude's `sync_claude_launch_into_active_task`. The fix is a single insertion point with a regression test.

**Tech Stack:** Rust, tokio, Tauri, existing daemon task graph modules, cargo test

---

## Root Cause

`DaemonCmd::LaunchCodex` with `Some(thread_id)` (resume path, `mod.rs:326-366`) only calls `find_session_by_external_id` after successful resume. If the thread was never previously bound to the task graph (first-time attach from provider history), the lookup returns `None` and no session is registered.

Contrast with Claude's `LaunchClaudeSdk` path (`mod.rs:436-445`) which unconditionally calls `sync_claude_launch_into_active_task` → `register_on_launch` on every successful launch, whether new or resumed.

Evidence:
- `mod.rs:341-354` — Codex resume success: only `find_session_by_external_id`, no `register_on_launch`
- `mod.rs:436-445` — Claude launch success: always `sync_claude_launch_into_active_task`
- `provider/codex.rs:46` — `register_on_launch` exists and works, just not called from resume path

## File Map

### Modified files

- `src-tauri/src/daemon/mod.rs:325-355` — add `register_on_launch` to Codex resume success path
- `src-tauri/src/daemon/state_tests.rs` — regression test

---

### Task 1: Add regression test and fix Codex resume task binding

**Files:**
- Modify: `src-tauri/src/daemon/state_tests.rs`
- Modify: `src-tauri/src/daemon/mod.rs:325-355`

- [ ] **Step 1: Write failing test**

Add to `src-tauri/src/daemon/state_tests.rs`:

```rust
#[test]
fn codex_register_on_launch_binds_resumed_thread_to_active_task() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.active_task_id = Some(task.task_id.clone());

    // Simulate a Codex resume launch — register_on_launch should bind it
    crate::daemon::provider::codex::register_on_launch(&mut s, "coder", "/ws", "thread_resumed_1");

    let session = s
        .task_graph
        .find_session_by_external_id(
            crate::daemon::task_graph::types::Provider::Codex,
            "thread_resumed_1",
        )
        .expect("resumed thread should be registered in task graph");
    assert_eq!(session.task_id, task.task_id);
    let updated_task = s.task_graph.get_task(&task.task_id).unwrap();
    assert_eq!(
        updated_task.current_coder_session_id.as_deref(),
        Some(session.session_id.as_str())
    );
}
```

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml codex_register_on_launch_binds_resumed_thread -- --nocapture
```

Expected: PASS (this tests `register_on_launch` itself, which works — the bug is that mod.rs doesn't call it).

- [ ] **Step 2: Fix the Codex resume path in `mod.rs`**

Replace lines 325-355 in `src-tauri/src/daemon/mod.rs`. The key change: after `codex::resume` succeeds, call `register_on_launch` if `find_session_by_external_id` returns `None`, then always emit task context events.

Before the `codex::resume` call, clone `role_id` and `cwd` since they are moved into `ResumeOpts`:

```rust
                let launch_result = match resume_thread_id {
                    Some(thread_id) => {
                        let resumed_thread_id = thread_id.clone();
                        let resume_role = role_id.clone();
                        let resume_cwd = cwd.clone();
                        match codex::resume(
                            codex::ResumeOpts {
                                role_id,
                                cwd,
                                thread_id,
                                launch_epoch,
                                codex_port: 4500,
                            },
                            state.clone(),
                            app.clone(),
                        )
                        .await
                        {
                            Ok(h) => {
                                codex_handle = Some(h);
                                let task_id = {
                                    let mut daemon = state.write().await;
                                    // If not already in task graph, register now
                                    if daemon
                                        .task_graph
                                        .find_session_by_external_id(
                                            crate::daemon::task_graph::types::Provider::Codex,
                                            &resumed_thread_id,
                                        )
                                        .is_none()
                                    {
                                        crate::daemon::provider::codex::register_on_launch(
                                            &mut daemon,
                                            &resume_role,
                                            &resume_cwd,
                                            &resumed_thread_id,
                                        );
                                    }
                                    daemon.active_task_id.clone()
                                };
                                if let Some(task_id) = task_id {
                                    emit_task_context_events(&state, &app, &task_id).await;
                                }
                                Ok(())
                            }
                            Err(e) => {
                                gui::emit_agent_status(&app, "codex", false, None, None);
                                gui::emit_system_log(
                                    &app,
                                    "error",
                                    &format!("[Daemon] Codex start failed: {e}"),
                                );
                                Err(e.to_string())
                            }
                        }
                    }
```

- [ ] **Step 3: Verify all tests pass**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture 2>&1 | tail -5
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/daemon/mod.rs src-tauri/src/daemon/state_tests.rs
git commit -m "fix: bind codex resumed sessions into active task graph"
```

---

### Task 2: Verify and record

- [ ] **Step 1: Run full test suite**

```bash
cargo test --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
bun test 2>&1 | tail -3
```

Expected: all pass.

- [ ] **Step 2: Commit plan with verification record**

```bash
git add docs/superpowers/plans/2026-04-06-codex-resume-task-binding.md
git commit -m "docs: codex resume task binding fix plan with verification"
```

## Final Acceptance Criteria

- Selecting a Codex session from provider history registers it in the active task graph.
- `TaskPanel` shows the Codex session in the session tree after resume.
- Claude resume behavior unchanged.
- All existing tests pass.
