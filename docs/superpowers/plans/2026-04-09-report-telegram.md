# Report Telegram Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an explicit `report_telegram` protocol flag so lead can selectively fan out important terminal messages to the globally configured Telegram chat, with polished HTML formatting.

**Architecture:** Extend the message protocol end-to-end with an optional boolean `report_telegram`, then replace the daemon's hard-coded Telegram trigger with a lead-only, terminal-status, `report_telegram`-driven gate. Keep Telegram configuration global and single-chat, and format outbound Telegram content with an HTML card-like template that includes `task_id`.

**Tech Stack:** Rust (Tauri daemon + bridge), TypeScript type surfaces, Telegram Bot API `sendMessage`, Codex structured output schema, Claude MCP reply tool schema, Rust unit tests, Bun build.

---

## File Map

### Core protocol / parsing

- Modify: `src-tauri/src/daemon/types.rs`
- Modify: `bridge/src/types.rs`
- Modify: `src/types.ts`
- Modify: `src-tauri/src/daemon/codex/structured_output.rs`
- Modify: `src-tauri/src/daemon/codex/structured_output_tests.rs`
- Modify: `src-tauri/src/daemon/codex/session_event.rs`
- Modify: `src-tauri/src/daemon/role_config/roles.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`
- Modify: `bridge/src/tools.rs`
- Modify: `bridge/src/tools_tests.rs`
- Modify: `bridge/src/mcp_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt.rs`

### Telegram routing / formatting

- Modify: `src-tauri/src/telegram/api.rs`
- Modify: `src-tauri/src/telegram/report.rs`
- Modify: `src-tauri/src/daemon/routing_dispatch.rs`

### Verification

- Run: `cargo test telegram`
- Run: `cargo test structured_output`
- Run: `cargo test tools`
- Run: `bun run build`
- Run: `git diff --check`

---

### Task 1: Extend the message protocol with `report_telegram`

**Acceptance criteria:**

- `BridgeMessage` carries an optional `report_telegram` boolean end-to-end
- Codex structured output accepts and preserves `report_telegram`
- Claude `reply()` accepts and preserves `report_telegram`
- frontend types stay in sync
- missing `report_telegram` still behaves as `false`

**Files:**

- Modify: `src-tauri/src/daemon/types.rs`
- Modify: `bridge/src/types.rs`
- Modify: `src/types.ts`
- Modify: `src-tauri/src/daemon/codex/structured_output.rs`
- Modify: `src-tauri/src/daemon/codex/structured_output_tests.rs`
- Modify: `src-tauri/src/daemon/codex/session_event.rs`
- Modify: `src-tauri/src/daemon/role_config/roles.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`
- Modify: `bridge/src/tools.rs`
- Modify: `bridge/src/tools_tests.rs`

**CM:** `feat: add report_telegram to message protocol`

- [ ] **Step 1: Add failing structured-output and tool-schema tests**

Add tests that prove:

- Codex output schema exposes optional `report_telegram: boolean`
- structured output parsing preserves `report_telegram: true`
- reply tool schema exposes optional `report_telegram: boolean`
- reply tool parsing preserves `report_telegram: true`

Suggested test additions:

```rust
#[test]
fn output_schema_allows_optional_report_telegram_boolean() {
    let schema = output_schema();
    assert_eq!(schema["properties"]["report_telegram"]["type"], "boolean");
    assert!(!schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v == "report_telegram"));
}
```

```rust
#[test]
fn parses_report_telegram_flag() {
    let parsed = parse_structured_output(
        r#"{"message":"done","send_to":"lead","status":"done","report_telegram":true}"#,
    )
    .unwrap();
    assert!(parsed.report_telegram);
}
```

```rust
#[test]
fn handle_reply_preserves_report_telegram() {
    let params = serde_json::json!({
        "name": "reply",
        "arguments": {
            "to": "lead",
            "text": "hello",
            "status": "done",
            "report_telegram": true
        }
    });
    let msg = handle_tool_call(&params, "coder").unwrap().unwrap();
    assert_eq!(msg.report_telegram, Some(true));
}
```

- [ ] **Step 2: Run the targeted tests and verify they fail for missing field support**

Run:

```bash
cargo test structured_output
cargo test tools
```

Expected:

- new tests fail because `report_telegram` does not exist yet

- [ ] **Step 3: Implement the protocol field end-to-end**

Make these exact data-shape changes:

```rust
// src-tauri/src/daemon/types.rs and bridge/src/types.rs
#[serde(skip_serializing_if = "Option::is_none")]
pub report_telegram: Option<bool>,
```

```rust
// src-tauri/src/daemon/codex/structured_output.rs
pub(super) struct ParsedOutput {
    pub(super) message: String,
    pub(super) send_to: Option<String>,
    pub(super) status: MessageStatus,
    pub(super) report_telegram: bool,
}
```

```rust
// parse_structured_output()
report_telegram: v
    .get("report_telegram")
    .and_then(|value| value.as_bool())
    .unwrap_or(false),
```

```rust
// src-tauri/src/daemon/role_config/roles.rs output_schema()
"report_telegram": {
    "type": "boolean",
    "description": "When true, fan out this terminal lead message to Telegram"
}
```

```rust
// bridge/src/tools.rs reply_tool_schema()
"report_telegram": {
    "type": "boolean",
    "description": "When true, fan out this terminal lead message to Telegram"
}
```

```rust
// bridge/src/tools.rs handle_tool_call()
let report_telegram = args
    .get("report_telegram")
    .and_then(|value| value.as_bool());
```

```rust
// src-tauri/src/daemon/codex/session_event.rs
msg.report_telegram = parsed.report_telegram.then_some(true);
```

```ts
// src/types.ts
reportTelegram?: boolean;
```

Use camelCase on the TypeScript side and serde `rename_all = "camelCase"` compatibility on the Rust structs already in place.

- [ ] **Step 4: Re-run protocol tests and confirm they pass**

Run:

```bash
cargo test structured_output
cargo test tools
```

Expected:

- all updated protocol tests pass

- [ ] **Step 5: Commit Task 1**

Run:

```bash
git add \
  src-tauri/src/daemon/types.rs \
  bridge/src/types.rs \
  src/types.ts \
  src-tauri/src/daemon/codex/structured_output.rs \
  src-tauri/src/daemon/codex/structured_output_tests.rs \
  src-tauri/src/daemon/codex/session_event.rs \
  src-tauri/src/daemon/role_config/roles.rs \
  src-tauri/src/daemon/role_config/roles_tests.rs \
  bridge/src/tools.rs \
  bridge/src/tools_tests.rs
git commit -m "feat: add report_telegram to message protocol"
```

---

### Task 2: Teach agent prompts and reply schemas when to use `report_telegram`

**Acceptance criteria:**

- lead-facing prompt guidance explicitly documents when to set `report_telegram=true`
- guidance includes: plan drafted, plan confirmed, task review result, final review result, blocking error
- guidance explicitly says only blocking errors should use it
- reply tool docs and examples stay aligned with protocol

**Files:**

- Modify: `bridge/src/mcp_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt.rs`
- Modify: `src-tauri/src/daemon/role_config/roles.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`

**CM:** `docs: define report_telegram prompt contract`

- [ ] **Step 1: Add failing prompt tests for the new guidance**

Add assertions that the lead prompt mentions:

- `report_telegram`
- plan drafted
- plan confirmed
- task review result
- final review result
- blocking error

Suggested test shape:

```rust
#[test]
fn lead_prompt_documents_report_telegram_usage() {
    let prompt = get_role("lead").unwrap().base_instructions;
    assert!(prompt.contains("report_telegram"));
    assert!(prompt.contains("plan drafted"));
    assert!(prompt.contains("plan confirmed"));
    assert!(prompt.contains("task review result"));
    assert!(prompt.contains("final review result"));
    assert!(prompt.contains("blocking error"));
}
```

- [ ] **Step 2: Run the prompt tests and verify they fail**

Run:

```bash
cargo test roles_tests
```

Expected:

- new prompt assertions fail before the guidance is added

- [ ] **Step 3: Add the prompt and schema guidance**

Update the lead protocol text so it says, in plain language:

- `report_telegram` is an optional boolean
- only lead should use it
- set it to true for plan drafted / plan confirmed / task review result / final review result / blocking error
- do not use it for non-blocking errors
- for `report_telegram=true` messages, keep the message concise and structured for Telegram formatting

Also update Claude bridge prompt text to mention the optional reply arg:

```text
Use reply(to, text, status, report_telegram?) tool to send messages.
Set report_telegram=true only on important terminal lead messages that should also be sent to Telegram.
```

- [ ] **Step 4: Re-run the prompt tests and confirm they pass**

Run:

```bash
cargo test roles_tests
```

Expected:

- updated prompt tests pass

- [ ] **Step 5: Commit Task 2**

Run:

```bash
git add \
  bridge/src/mcp_protocol.rs \
  src-tauri/src/daemon/role_config/claude_prompt.rs \
  src-tauri/src/daemon/role_config/roles.rs \
  src-tauri/src/daemon/role_config/roles_tests.rs
git commit -m "docs: define report_telegram prompt contract"
```

---

### Task 3: Replace Telegram hard-coded routing with `report_telegram` and HTML formatting

**Acceptance criteria:**

- Telegram fan-out no longer depends on `to == "user"`
- only terminal lead messages with `report_telegram=true` are eligible
- global single-chat config is still respected
- `notifications_enabled` remains a hard gate
- Telegram output uses HTML formatting and escapes dynamic text safely
- message body includes `task_id` when present

**Files:**

- Modify: `src-tauri/src/telegram/api.rs`
- Modify: `src-tauri/src/telegram/report.rs`
- Modify: `src-tauri/src/daemon/routing_dispatch.rs`

**CM:** `feat: route report_telegram messages to telegram`

- [ ] **Step 1: Add failing Telegram routing/formatting tests**

Add tests that prove:

- `report_telegram=true` + lead + terminal `done` triggers
- `report_telegram=true` + lead + terminal `error` triggers
- missing/false `report_telegram` does not trigger
- non-lead does not trigger
- formatter output contains HTML-safe escaped text and `task_id`

Suggested test shapes:

```rust
#[test]
fn report_telegram_requires_lead_terminal_and_flag() {
    let mut msg = test_message("lead", "coder", Some(MessageStatus::Done));
    msg.report_telegram = Some(true);
    assert!(should_send_telegram_report(&msg));

    msg.report_telegram = None;
    assert!(!should_send_telegram_report(&msg));
}
```

```rust
#[test]
fn html_formatter_escapes_dynamic_text() {
    let formatted = escape_html(r#"<tag> & "quote""#);
    assert_eq!(formatted, "&lt;tag&gt; &amp; &quot;quote&quot;");
}
```

- [ ] **Step 2: Run the Telegram tests and verify they fail**

Run:

```bash
cargo test telegram
```

Expected:

- new trigger/formatter tests fail before implementation

- [ ] **Step 3: Implement the new Telegram gate and formatter**

Refactor `src-tauri/src/telegram/report.rs` roughly like this:

```rust
pub fn should_send_telegram_report(msg: &BridgeMessage) -> bool {
    msg.from == "lead"
        && msg.report_telegram == Some(true)
        && matches!(msg.status, Some(MessageStatus::Done) | Some(MessageStatus::Error))
}
```

Add small helpers:

```rust
fn escape_html(text: &str) -> String { /* replace &, <, >, " */ }
fn event_emoji(status: MessageStatus) -> &'static str { /* ✅ / 🚨 */ }
```

Build an HTML message body with:

- bold title
- task id
- task title
- status
- optional worktree/workspace
- summary body

Update `src-tauri/src/telegram/api.rs` to send HTML parse mode:

```rust
let body = serde_json::json!({
    "chat_id": chat_id,
    "text": text,
    "parse_mode": "HTML",
});
```

Update `src-tauri/src/daemon/routing_dispatch.rs` to:

- call the renamed trigger helper
- respect existing global Telegram runtime/chat gating
- keep failures non-fatal

- [ ] **Step 4: Re-run Telegram tests and confirm they pass**

Run:

```bash
cargo test telegram
```

Expected:

- Telegram routing/formatting tests pass

- [ ] **Step 5: Commit Task 3**

Run:

```bash
git add \
  src-tauri/src/telegram/api.rs \
  src-tauri/src/telegram/report.rs \
  src-tauri/src/daemon/routing_dispatch.rs
git commit -m "feat: route report_telegram messages to telegram"
```

---

### Task 4: Full verification and integration review

**Acceptance criteria:**

- Rust tests for changed areas pass
- frontend build passes
- whitespace check is clean
- no unintended protocol regressions are found in review

**Files:**

- Review only: changed files from Tasks 1-3

**CM:** `chore: verify report_telegram integration`

- [ ] **Step 1: Run the full targeted verification suite**

Run:

```bash
cargo test telegram
cargo test structured_output
cargo test tools
bun run build
git diff --check
```

Expected:

- all tests pass
- build succeeds
- no whitespace or conflict markers

- [ ] **Step 2: Review for transport/prompt consistency**

Check manually that:

- `report_telegram` naming is consistent everywhere
- prompt text and schema both document the same field name
- Telegram gate no longer depends on `to == "user"`
- `report_telegram` is optional everywhere

- [ ] **Step 3: Commit Task 4**

Run:

```bash
git add -A
git commit -m "chore: verify report_telegram integration"
```

---

## Self-Review

### Spec coverage

- protocol field addition -> Task 1
- prompt rules for when to use the field -> Task 2
- daemon-side Telegram gate replacement -> Task 3
- polished Telegram HTML formatting -> Task 3
- verification/build/tests -> Task 4

No spec gaps remain.

### Placeholder scan

- no `TODO` / `TBD`
- each task names exact files
- each task includes concrete commands
- code changes are spelled out for the non-obvious parts

### Type consistency

The plan consistently uses one field name everywhere:

- Rust / JSON wire: `report_telegram`
- TypeScript surface: `reportTelegram`

No alternate spellings are introduced.

## Execution Record

### Task 1

- CM: `feat: add report_telegram to message protocol`
- Implementation commit: `3e2c95a7`
- Lead verification:
  - `cargo test structured_output` -> 16 passed, 0 failed
  - `cargo test -p dimweave-bridge tools` -> 17 passed, 0 failed
  - `cargo test telegram` -> passed
  - `bun run build` -> success
  - `git diff --check` -> clean

### Task 2

- CM: `docs: define report_telegram prompt contract`
- Implementation commit: `6a6ad203`
- Lead verification:
  - `cargo test claude_prompt` -> 15 passed, 0 failed
  - `cargo test -p dimweave-bridge mcp_protocol` -> 9 passed, 0 failed
  - `cargo test lead_prompt` -> 12 passed, 0 failed
  - `git diff --check` -> clean

### Task 3

- CM: `feat: route report_telegram messages to telegram`
- Implementation commit: `fa5f16e4`
- Follow-up fix commit: `d5e76ef5` (`fix: add notifications_enabled hard gate to report_telegram routing`)
- Lead verification:
  - `cargo test telegram` -> 37 passed, 0 failed
  - `cargo test telegram_notifications_disabled` -> 1 passed, 0 failed
  - `cargo test` -> 442 passed, 0 failed
  - `bun run build` -> success
  - `git diff --check` -> clean

### Task 4

- CM: `chore: verify report_telegram integration`
- Lead verification:
  - `cargo test telegram` -> 37 passed, 0 failed
  - `cargo test structured_output` -> 16 passed, 0 failed
  - `cargo test -p dimweave-bridge tools` -> 17 passed, 0 failed
  - `bun run build` -> success
  - `git diff --check` -> clean
- Manual review:
  - `report_telegram` naming is consistent across Rust, JSON, prompt, and TypeScript surfaces
  - Telegram routing no longer depends on `to == "user"`
  - `report_telegram` remains optional everywhere
  - `notifications_enabled` remains a hard gate

## Post-Release Addendum

- 2026-04-09 发生一轮 Codex 发送链路事故：连接显示 `active`，但首条消息在 `turn/start` 阶段失败。
- 根因不是连接链路，而是 Task 1 引入的 Codex `outputSchema` 变更只新增了 `report_telegram` property，没有同步更新 `required`。
- 该问题由提交 `3e2c95a7`（`feat: add report_telegram to message protocol`）引入。
- 独立修复记录见：
  - [2026-04-09-codex-output-schema-hotfix.md](/Users/jason/floder/agent-bridge/docs/superpowers/plans/2026-04-09-codex-output-schema-hotfix.md)
