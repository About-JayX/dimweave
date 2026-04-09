# Report Telegram Route Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make prompt-originated `report_telegram` messages follow the same routed delivery path across Codex and Claude bridge flows, so terminal lead reports reliably reach Telegram when the flag is set.

**Architecture:** Keep the prompt contract as the single source of intent (`status` + `report_telegram` come from the model/tool call), preserve that metadata at provider ingress, and route all prompt-originated visible terminal messages through `routing::route_message(...)` so the Telegram fan-out hook in `routing_dispatch.rs` sees them. Do not expand Telegram eligibility to Claude SDK direct fallback, because that path has no explicit `report_telegram` intent.

**Tech Stack:** Rust, Tauri daemon, bridge crate, Tokio async tests, Telegram routing hook

---

## File Map

### Modified files

- `docs/superpowers/plans/2026-04-09-report-telegram-route-unification.md`
- `src-tauri/src/daemon/codex/handler.rs`
- `src-tauri/src/daemon/codex/session_event.rs`

### Verification files

- `bridge/src/tools_tests.rs`
- `src-tauri/src/telegram/report.rs`

## Baseline

- Worktree: `/Users/jason/floder/agent-bridge/.worktrees/report-telegram-route-unification`
- Branch: `fix/report-telegram-route-unification`
- Baseline verification in this worktree on `2026-04-09`:
  - `cargo build --manifest-path bridge/Cargo.toml` ✅
  - `cargo test --manifest-path bridge/Cargo.toml report_telegram -- --nocapture` ✅ 4 passed
  - `cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture` ✅ 33 passed

## File Responsibilities

- `src-tauri/src/daemon/codex/handler.rs`
  - Ingress for Codex dynamic tool calls (`reply`, `check_messages`, `get_status`)
  - Must preserve the prompt contract for `reply`: `to`, `text`, `status`, `report_telegram`
- `src-tauri/src/daemon/codex/session_event.rs`
  - Ingress for Codex structured terminal output
  - Must build one routed `BridgeMessage` shape regardless of whether `send_to` is `user`, `lead`, or `coder`
- `src-tauri/src/daemon/routing_dispatch.rs`
  - Existing Telegram fan-out hook; this plan depends on it unchanged

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `2b255a06` | `manual review` | `cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture` ✅ 6 passed; `git diff --check` ✅ | Codex dynamic `reply` must preserve the same routing metadata that Claude bridge `reply` already preserves. |
| Task 2 | `bdbc9a95` | `manual review` | `cargo test --manifest-path src-tauri/Cargo.toml session_event -- --nocapture` ✅ 9 passed; `cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture` ✅ 34 passed; `git diff --check` ✅ | Prompt-originated user-target terminal messages must still go through `routing::route_message(...)`; direct GUI emission bypasses Telegram fan-out. |
| Final | `bfc16cac` | `final deep review` | `cargo test --manifest-path bridge/Cargo.toml report_telegram -- --nocapture` ✅ 4 passed; `cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture` ✅ 6 passed; `cargo test --manifest-path src-tauri/Cargo.toml session_event -- --nocapture` ✅ 9 passed; `cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture` ✅ 34 passed; `git diff --check` ✅ | The supported Telegram contract is: explicit prompt/tool intent in, unified routing path through daemon, Telegram hook decides fan-out. |
| Ingress gate | `85ea11ad` | `manual review` | `cargo test --manifest-path bridge/Cargo.toml report_telegram -- --nocapture` ✅; `cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture` ✅; `cargo test --manifest-path src-tauri/Cargo.toml session_event -- --nocapture` ✅ | All three ingress paths (bridge reply, codex dynamic reply, codex structured output) now strip `report_telegram` from non-lead senders at source. Defense-in-depth: `should_send_telegram_report()` backstop remains unchanged. |

### Task 1: Preserve full `reply()` routing metadata in Codex dynamic tool handling

**Acceptance criteria**

- Codex dynamic `reply()` preserves `status` instead of hard-coding `done`
- Codex dynamic `reply()` preserves `report_telegram`
- Empty/invalid replies keep the current guard behavior
- Focused `codex::handler` tests pass

**Files:**
- Modify: `src-tauri/src/daemon/codex/handler.rs`

- [x] **Step 1: Write the failing tests**

Add focused tests in `src-tauri/src/daemon/codex/handler.rs` for a pure message-builder helper:

```rust
#[test]
fn reply_builder_preserves_status_and_report_telegram() {
    let args = serde_json::json!({
        "to": "user",
        "text": "final review result",
        "status": "error",
        "report_telegram": true
    });

    let msg = build_reply_message(&args, "lead").expect("message");
    assert_eq!(msg.to, "user");
    assert_eq!(msg.status, Some(MessageStatus::Error));
    assert_eq!(msg.report_telegram, Some(true));
}

#[test]
fn reply_builder_defaults_status_to_done_and_flag_to_none() {
    let args = serde_json::json!({
        "to": "coder",
        "text": "take task 2"
    });

    let msg = build_reply_message(&args, "lead").expect("message");
    assert_eq!(msg.status, Some(MessageStatus::Done));
    assert_eq!(msg.report_telegram, None);
}
```

- [x] **Step 2: Run the focused tests to confirm RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture
```

Expected: FAIL because `handle_reply()` still hard-codes `done` and drops `report_telegram`.

- [x] **Step 3: Implement the minimal builder**

Extract a pure helper in `src-tauri/src/daemon/codex/handler.rs`:

```rust
fn build_reply_message(args: &Value, from: &str) -> Option<BridgeMessage> {
    let to = args["to"].as_str().unwrap_or("user");
    let text = args["text"].as_str().unwrap_or("");
    if text.trim().is_empty() {
        return None;
    }

    let status = args["status"]
        .as_str()
        .and_then(MessageStatus::parse)
        .unwrap_or(MessageStatus::Done);
    let report_telegram = args
        .get("report_telegram")
        .and_then(|value| value.as_bool());

    Some(BridgeMessage {
        id: format!("codex_{}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        display_source: Some("codex".into()),
        to: to.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(status),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("codex".into()),
        attachments: None,
        report_telegram,
    })
}
```

Then make `handle_reply()` call this helper, stamp context, and route the built message.

- [x] **Step 4: Run the focused tests to confirm GREEN**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Verify diff hygiene**

Run:

```bash
git diff --check
```

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/daemon/codex/handler.rs
git commit -m "fix: preserve codex reply routing metadata"
```

- [x] **Step 7: Update `## CM Memory`**

Replace the Task 1 placeholder row with the real commit and verification evidence.

### Task 2: Route Codex structured-output user messages through the daemon router

**Acceptance criteria**

- Codex structured terminal output builds one routed `BridgeMessage` shape for `user`, `lead`, and `coder`
- `report_telegram` is preserved for `send_to="user"`
- `handle_completed_agent_message()` no longer bypasses `routing::route_message(...)` for user-target terminal messages
- Existing Telegram gate logic remains unchanged
- Focused `codex::session_event` and `telegram` tests pass

**Files:**
- Modify: `src-tauri/src/daemon/codex/session_event.rs`

- [x] **Step 1: Write the failing tests**

Add pure-helper tests in `src-tauri/src/daemon/codex/session_event.rs`:

```rust
#[test]
fn completed_output_builder_preserves_user_target_and_report_flag() {
    let parsed = ParsedOutput {
        message: "final review result".into(),
        send_to: Some("user".into()),
        status: MessageStatus::Done,
        report_telegram: true,
    };

    let msg = build_completed_output_message("lead", &parsed, true).expect("message");
    assert_eq!(msg.to, "user");
    assert_eq!(msg.status, Some(MessageStatus::Done));
    assert_eq!(msg.report_telegram, Some(true));
}

#[test]
fn completed_output_builder_restricts_schema_routes_to_known_roles() {
    let parsed = ParsedOutput {
        message: "final review result".into(),
        send_to: Some("reviewer".into()),
        status: MessageStatus::Done,
        report_telegram: true,
    };

    let msg = build_completed_output_message("lead", &parsed, true).expect("message");
    assert_eq!(msg.to, "user");
    assert_eq!(msg.report_telegram, Some(true));
}
```

- [x] **Step 2: Run the focused tests to confirm RED**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml codex::session_event -- --nocapture
```

Expected: FAIL because the helper does not exist and the `user` path still emits directly to GUI.

- [x] **Step 3: Implement the routed builder**

Extract a helper in `src-tauri/src/daemon/codex/session_event.rs`:

```rust
fn build_completed_output_message(
    role_id: &str,
    parsed: &ParsedOutput,
    schema_route_enabled: bool,
) -> Option<BridgeMessage> {
    if !should_emit_final_message(&parsed.message) {
        return None;
    }

    let target = if schema_route_enabled {
        parsed
            .send_to
            .as_deref()
            .filter(|target| matches!(*target, "user" | "lead" | "coder"))
            .unwrap_or("user")
    } else {
        "user"
    };

    let mut msg = build_msg_with_status(role_id, target, &parsed.message, parsed.status);
    msg.report_telegram = parsed.report_telegram.then_some(true);
    Some(msg)
}
```

Then simplify `handle_completed_agent_message()` to:

```rust
let Some(mut msg) = build_completed_output_message(role_id, &parsed, schema_route_enabled) else {
    return;
};
state.read().await.stamp_message_context(role_id, &mut msg);
routing::route_message(state, app, msg).await;
```

This intentionally removes the `gui::emit_agent_message(...)` branch for structured terminal output.

- [x] **Step 4: Run the focused tests to confirm GREEN**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml codex::session_event -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture
```

Expected: PASS.

- [x] **Step 5: Verify diff hygiene**

Run:

```bash
git diff --check
```

Expected: PASS.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/daemon/codex/session_event.rs
git commit -m "fix: route codex report_telegram messages through router"
```

- [x] **Step 7: Update `## CM Memory`**

Replace the Task 2 placeholder row with the real commit and verification evidence.

## Final Review Checklist

- [x] Re-read this plan and verify Task 1 + Task 2 cover every confirmed bug:
  - Codex dynamic `reply()` dropping `status`
  - Codex dynamic `reply()` dropping `report_telegram`
  - Codex structured-output `send_to="user"` bypassing `routing::route_message(...)`
- [x] Confirm no task tries to make Claude SDK direct fallback eligible for Telegram; that backup path lacks explicit prompt intent and is intentionally outside this fix
- [x] Run final verification:

```bash
cargo test --manifest-path bridge/Cargo.toml report_telegram -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml codex::session_event -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture
git diff --check
```

Expected: all PASS.
