# Telegram Remote Control MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Telegram bot bridge that automatically delivers lead completion reports to a paired user chat and routes inbound Telegram text back into the existing lead workflow.

**Architecture:** Keep Tauri/Rust daemon as the only Telegram owner. A daemon-managed long-poll runtime talks to the Telegram Bot API, persists one local bot configuration plus one paired chat, converts inbound Telegram text into standard `user -> lead` messages, and mirrors eligible `lead -> user` terminal replies into Telegram as structured plain-text reports. The bridge, Claude SDK path, Codex app-server path, and existing task/session routing stay intact.

**Tech Stack:** Rust, tokio, reqwest, Tauri commands/events, React 19, Zustand, Bun tests

---

## Execution Rules

- Phase 1 scope is fixed:
  - one bot token
  - one paired private Telegram chat
  - long polling via `getUpdates` (no webhook)
  - inbound text-only messages
  - outbound lead terminal reports only (`status = done | error`)
- Do not change `bridge/**`; Telegram belongs to the daemon/runtime layer, not the Claude MCP sidecar.
- Do not block GUI delivery when Telegram fails. GUI routing stays primary; Telegram is a best-effort secondary transport.
- Reuse existing task graph and artifact timeline. Every Telegram report written to disk should also become a `summary` artifact.
- Keep every new source file under the repo's 200-line limit by splitting helpers aggressively.

## Product Decisions Locked For This MVP

1. **Pairing model:** desktop-generated one-time code; user sends `/pair <code>` to the bot; daemon binds exactly one `chat_id`.
2. **Outbound trigger:** only `lead -> user` messages with terminal status generate Telegram output.
3. **Outbound format:** plain text, Telegram-safe, hard-capped/chunked to the Bot API message limit.
4. **Inbound target:** every accepted Telegram text is routed to `lead`, never directly to `coder` or `reviewer`.
5. **Persistence:** Telegram config/state lives in its own daemon-owned JSON file under the app config dir, not inside provider history or task graph persistence.
6. **Non-goals:** attachments, group chats, multi-user chat allowlists, inline keyboards, webhook hosting, Telegram-originated approvals.

## File Map

### New backend files

- `src-tauri/src/telegram/mod.rs`
  - Module exports and runtime start/stop helpers
- `src-tauri/src/telegram/types.rs`
  - Telegram config DTOs, runtime state DTOs, Bot API payloads, outbound queue item types
- `src-tauri/src/telegram/config.rs`
  - Load/save config JSON, app-config path resolution, token masking helpers
- `src-tauri/src/telegram/api.rs`
  - `getMe`, `getUpdates`, `sendMessage` wrappers over `reqwest`
- `src-tauri/src/telegram/pairing.rs`
  - one-time code generation, expiration, `/pair` parsing, chat binding rules
- `src-tauri/src/telegram/report.rs`
  - terminal lead-report formatting, chunking, local artifact file creation
- `src-tauri/src/telegram/runtime.rs`
  - long-poll loop, outbound send queue, inbound update handling, daemon state sync
- `src-tauri/src/commands_telegram.rs`
  - Tauri commands for Telegram config/state/pairing actions
- `src-tauri/src/daemon/user_input.rs`
  - shared helper for GUI-originated and Telegram-originated user message dispatch
- `src-tauri/src/daemon/telegram_lifecycle.rs`
  - start/restart/stop helpers so `daemon/mod.rs` only wires the lifecycle instead of absorbing more runtime logic
- `src/stores/telegram-store.ts`
  - frontend state/actions for Telegram config and runtime status
- `src/components/AgentStatus/TelegramPanel.tsx`
  - bot token input, enable toggle, pairing controls, runtime status
- `src/components/AgentStatus/TelegramPanel.test.tsx`
  - render/action coverage for the Telegram panel

### Modified backend files

- `src-tauri/src/main.rs`
  - register `commands_telegram` and the `telegram` module
- `src-tauri/src/daemon/cmd.rs`
  - add daemon command variants for Telegram state/config/pairing lifecycle
- `src-tauri/src/daemon/mod.rs`
  - own `TelegramHandle` lifecycle and daemon command handling
- `src-tauri/src/daemon/state.rs`
  - store current Telegram runtime snapshot and outbound sender handle
- `src-tauri/src/daemon/gui.rs`
  - emit `telegram_state` events to the frontend
- `src-tauri/src/daemon/routing_dispatch.rs`
  - trigger outbound Telegram report delivery after successful `lead -> user` terminal routing
- `src-tauri/src/daemon/routing_user_input.rs`
  - slim down to GUI adapter and delegate shared user-input dispatch to `daemon/user_input.rs`
- `src-tauri/src/commands_artifact.rs`
  - no API changes, but verify Telegram report artifacts remain previewable

### Modified frontend/docs files

- `src/components/AgentStatus/index.tsx`
  - mount `TelegramPanel`
- `src/components/MessagePanel/SourceBadge.tsx`
  - add `telegram` badge label
- `src/components/MessagePanel/surface-styles.ts`
  - add Telegram accent/surface colors
- `src/types.ts`
  - add `TelegramStateInfo` for shared frontend typing
- `CLAUDE.md`
  - document Telegram runtime in current architecture and runtime flow
- `UPDATE.md`
  - record the Telegram MVP addition and any new limits

---

### Task 1: Establish Telegram config, daemon state, and command surface

**Files:**
- Create: `src-tauri/src/telegram/mod.rs`
- Create: `src-tauri/src/telegram/types.rs`
- Create: `src-tauri/src/telegram/config.rs`
- Create: `src-tauri/src/commands_telegram.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing Rust tests for config round-trip and masked runtime state**

```rust
#[test]
fn telegram_config_round_trip_preserves_pairing_and_cursor() {
    let cfg = TelegramConfig {
        enabled: true,
        bot_token: "123:abc".into(),
        notifications_enabled: true,
        paired_chat_id: Some(777001),
        paired_chat_label: Some("jason".into()),
        last_update_id: Some(42),
        pending_pair_code: None,
        pending_pair_expires_at: None,
    };

    let path = temp_telegram_path("round_trip");
    save_config(&path, &cfg).unwrap();
    let loaded = load_config(&path).unwrap();

    assert_eq!(loaded.paired_chat_id, Some(777001));
    assert_eq!(loaded.last_update_id, Some(42));
}

#[test]
fn runtime_snapshot_masks_sensitive_values() {
    let state = TelegramRuntimeState::from_config(&TelegramConfig {
        bot_token: "123:secret".into(),
        paired_chat_label: Some("@jason".into()),
        ..TelegramConfig::default()
    });

    assert_eq!(state.token_label.as_deref(), Some("123:***"));
    assert_eq!(state.paired_chat_label.as_deref(), Some("@jason"));
}
```

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml telegram::config -- --nocapture
```

Expected: FAIL because Telegram config/state types and helpers do not exist yet.

- [ ] **Step 2: Add Telegram config/runtime DTOs and file persistence**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub notifications_enabled: bool,
    pub paired_chat_id: Option<i64>,
    pub paired_chat_label: Option<String>,
    pub last_update_id: Option<i64>,
    pub pending_pair_code: Option<String>,
    pub pending_pair_expires_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramRuntimeState {
    pub enabled: bool,
    pub connected: bool,
    pub notifications_enabled: bool,
    pub token_label: Option<String>,
    pub bot_username: Option<String>,
    pub paired_chat_label: Option<String>,
    pub pending_pair_code: Option<String>,
    pub pending_pair_expires_at: Option<u64>,
    pub last_error: Option<String>,
    pub last_delivery_at: Option<u64>,
    pub last_inbound_at: Option<u64>,
}
```

- [ ] **Step 3: Add daemon commands and Tauri command wrappers**

```rust
pub enum DaemonCmd {
    GetTelegramState {
        reply: oneshot::Sender<TelegramRuntimeState>,
    },
    SaveTelegramConfig {
        bot_token: String,
        enabled: bool,
        notifications_enabled: bool,
        reply: oneshot::Sender<Result<TelegramRuntimeState, String>>,
    },
    GenerateTelegramPairCode {
        reply: oneshot::Sender<Result<TelegramRuntimeState, String>>,
    },
    ClearTelegramPairing {
        reply: oneshot::Sender<Result<TelegramRuntimeState, String>>,
    },
    // existing variants...
}
```

- [ ] **Step 4: Wire the commands into `main.rs` without touching bridge/runtime behavior yet**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml telegram::config daemon::cmd -- --nocapture
```

Expected: PASS for config/state/command surface tests.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/telegram/mod.rs \
  src-tauri/src/telegram/types.rs \
  src-tauri/src/telegram/config.rs \
  src-tauri/src/commands_telegram.rs \
  src-tauri/src/daemon/cmd.rs \
  src-tauri/src/daemon/state.rs \
  src-tauri/src/main.rs
git commit -m "feat: add telegram config and daemon command surface"
```

### Task 2: Build pairing and inbound Telegram-to-lead routing

**Files:**
- Create: `src-tauri/src/telegram/api.rs`
- Create: `src-tauri/src/telegram/pairing.rs`
- Create: `src-tauri/src/telegram/runtime.rs`
- Create: `src-tauri/src/daemon/user_input.rs`
- Create: `src-tauri/src/daemon/telegram_lifecycle.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/routing_user_input.rs`
- Modify: `src-tauri/src/daemon/gui.rs`

- [ ] **Step 1: Write failing tests for pairing and inbound routing behavior**

```rust
#[tokio::test]
async fn pair_command_binds_first_chat_and_persists_it() {
    let mut cfg = TelegramConfig::default();
    let next = apply_pair_command(&mut cfg, 777001, "alice", "/pair 123456", "123456").unwrap();
    assert_eq!(next.paired_chat_id, Some(777001));
    assert_eq!(next.paired_chat_label.as_deref(), Some("@alice"));
}

#[tokio::test]
async fn inbound_text_from_paired_chat_routes_to_lead_with_telegram_badge() {
    let state = test_state_with_active_lead();
    handle_inbound_text(&state, &test_app(), 777001, "@alice", "继续执行并汇总最新结果").await.unwrap();

    let buffered = state.read().await.buffered_messages.clone();
    assert!(buffered.iter().any(|msg| {
        msg.from == "user"
            && msg.to == "lead"
            && msg.display_source.as_deref() == Some("telegram")
    }));
}
```

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml telegram::pairing telegram::runtime -- --nocapture
```

Expected: FAIL because runtime/pairing/user-input helpers do not exist yet.

- [ ] **Step 2: Implement Telegram Bot API client with minimal endpoints**

```rust
pub async fn get_me(client: &Client, token: &str) -> anyhow::Result<GetMeResponse>;
pub async fn get_updates(
    client: &Client,
    token: &str,
    offset: Option<i64>,
    timeout_secs: u64,
) -> anyhow::Result<Vec<TelegramUpdate>>;
pub async fn send_message(
    client: &Client,
    token: &str,
    chat_id: i64,
    text: &str,
) -> anyhow::Result<()>;
```

- [ ] **Step 3: Implement one-time pairing flow**

```rust
pub fn generate_pair_code(now_ms: u64) -> (String, u64) {
    // six digits, 10-minute expiry
}

pub fn match_pair_command(text: &str) -> Option<&str> {
    text.strip_prefix("/pair ").map(str::trim)
}
```

- [ ] **Step 4: Extract shared user-input dispatch and reuse it for Telegram**

```rust
pub async fn dispatch_user_input(
    state: &SharedState,
    app: &AppHandle,
    content: String,
    target: String,
    attachments: Option<Vec<Attachment>>,
    display_source: &str,
    echo_to_gui: bool,
)
```

Rules for this helper:
- GUI calls it with `display_source = "user"` and `echo_to_gui = true`
- Telegram runtime calls it with `display_source = "telegram"` and `echo_to_gui = true`
- stamped task/session ownership must stay identical to existing GUI behavior

- [ ] **Step 5: Start a daemon-owned long-poll runtime**

```rust
pub struct TelegramHandle {
    pub outbound_tx: mpsc::Sender<TelegramOutbound>,
    shutdown_tx: oneshot::Sender<()>,
}

pub async fn start_runtime(
    state: SharedState,
    app: AppHandle,
    config: TelegramConfig,
) -> anyhow::Result<TelegramHandle>
```

Runtime requirements:
- call `getMe` on startup to validate the token
- long-poll `getUpdates` with persisted `last_update_id + 1`
- accept `/pair <code>` from any private chat only while a pair code is active
- accept normal text only from the paired chat
- persist `last_update_id` after each processed update
- emit `telegram_state` on connect, error, pairing success, and inbound activity

- [ ] **Step 6: Verify inbound routing and pairing**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml telegram::pairing telegram::runtime daemon::routing_user_input -- --nocapture
```

Expected: PASS for pairing parse/bind rules and Telegram-originated user routing.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/telegram/api.rs \
  src-tauri/src/telegram/pairing.rs \
  src-tauri/src/telegram/runtime.rs \
  src-tauri/src/daemon/user_input.rs \
  src-tauri/src/daemon/telegram_lifecycle.rs \
  src-tauri/src/daemon/mod.rs \
  src-tauri/src/daemon/routing_user_input.rs \
  src-tauri/src/daemon/gui.rs
git commit -m "feat: add telegram pairing and inbound lead routing"
```

### Task 3: Deliver outbound lead reports to Telegram and capture them as artifacts

**Files:**
- Create: `src-tauri/src/telegram/report.rs`
- Modify: `src-tauri/src/daemon/routing_dispatch.rs`
- Modify: `src-tauri/src/daemon/state.rs`

- [ ] **Step 1: Write failing tests for outbound trigger, chunking, and artifact persistence**

```rust
#[test]
fn only_terminal_lead_to_user_messages_trigger_reports() {
    assert!(should_send_lead_report(&BridgeMessage {
        from: "lead".into(),
        to: "user".into(),
        status: Some(MessageStatus::Done),
        ..test_message()
    }));

    assert!(!should_send_lead_report(&BridgeMessage {
        from: "lead".into(),
        to: "user".into(),
        status: Some(MessageStatus::InProgress),
        ..test_message()
    }));
}

#[test]
fn telegram_report_chunks_text_at_platform_limit() {
    let parts = chunk_report(&"x".repeat(5000));
    assert!(parts.len() >= 2);
    assert!(parts.iter().all(|part| part.len() <= 4096));
}
```

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml telegram::report -- --nocapture
```

Expected: FAIL because report helpers and hook do not exist yet.

- [ ] **Step 2: Implement plain-text lead report formatting**

```rust
pub fn build_lead_report(task: Option<&TaskSnapshot>, msg: &BridgeMessage) -> String {
    format!(
        "Dimweave update\nTask: {task_title}\nStatus: {status}\n\n{body}",
        task_title = task.map(|snap| snap.task.title.as_str()).unwrap_or("No active task"),
        status = msg.status.map(|s| s.as_str()).unwrap_or("done"),
        body = msg.content.trim(),
    )
}
```

Report rules:
- include task title when available
- include message status
- append latest session/artifact counts if snapshot exists
- output plain text only
- chunk to <= 4096 characters

- [ ] **Step 3: Persist each outbound report as a `summary` artifact**

```rust
pub fn write_report_artifact(base_dir: &Path, task_id: &str, message_id: &str, text: &str) -> anyhow::Result<PathBuf>;
```

Artifact rules:
- store under an app-owned `telegram-reports/` directory
- add `ArtifactKind::Summary`
- emit updated task context so `TaskPanel` immediately shows the report

- [ ] **Step 4: Hook outbound delivery into routing after successful GUI/user delivery**

```rust
if matches!(outcome.result, RouteResult::Delivered | RouteResult::ToGui)
    && telegram::report::should_send_lead_report(&msg)
{
    telegram::report::queue_lead_report(state, app, &msg).await;
}
```

Behavior rules:
- never send on `Buffered` or `Dropped`
- never block or fail the original GUI route if Telegram send fails
- log delivery failures to `system_log`

- [ ] **Step 5: Verify outbound behavior**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml telegram::report daemon::routing -- --nocapture
```

Expected: PASS for trigger gating, chunking, and summary artifact write flow.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/telegram/report.rs \
  src-tauri/src/daemon/routing_dispatch.rs \
  src-tauri/src/daemon/state.rs
git commit -m "feat: send lead reports to telegram and persist summary artifacts"
```

### Task 4: Add Telegram settings/status UI to the shell

**Files:**
- Create: `src/stores/telegram-store.ts`
- Create: `src/components/AgentStatus/TelegramPanel.tsx`
- Create: `src/components/AgentStatus/TelegramPanel.test.tsx`
- Modify: `src/components/AgentStatus/index.tsx`
- Modify: `src/components/MessagePanel/SourceBadge.tsx`
- Modify: `src/components/MessagePanel/surface-styles.ts`
- Modify: `src/types.ts`

- [ ] **Step 1: Write failing frontend tests for Telegram panel behavior**

```tsx
test("renders pairing code and bound chat state", async () => {
  render(<TelegramPanel />);
  expect(screen.getByText("Telegram")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: /generate pairing code/i })).toBeInTheDocument();
});

test("shows telegram source badge for inbound remote messages", () => {
  expect(getSourceBadgePresentation("telegram").label).toBe("Telegram");
});
```

Run:

```bash
bun test src/components/AgentStatus/TelegramPanel.test.tsx
```

Expected: FAIL because Telegram panel/store/types do not exist yet.

- [ ] **Step 2: Add frontend Telegram state typing and store actions**

```ts
export interface TelegramStateInfo {
  enabled: boolean;
  connected: boolean;
  notificationsEnabled: boolean;
  tokenLabel?: string;
  botUsername?: string;
  pairedChatLabel?: string;
  pendingPairCode?: string;
  pendingPairExpiresAt?: number;
  lastError?: string;
  lastDeliveryAt?: number;
  lastInboundAt?: number;
}
```

Store actions:
- `fetchState()`
- `saveConfig(botToken, enabled, notificationsEnabled)`
- `generatePairCode()`
- `clearPairing()`
- live sync via `listen("telegram_state", ...)`

- [ ] **Step 3: Build `TelegramPanel`**

Required UI:
- masked bot-token status + editable token input
- enabled / notifications toggle
- “Generate pairing code” action
- paired chat label or “Not paired”
- runtime status (`connected` / `error` / `disabled`)
- last error / last inbound / last delivery timestamps

- [ ] **Step 4: Integrate the panel into Agent Status and message presentation**

Rules:
- mount `TelegramPanel` under the existing Claude/Codex panels
- keep remote user messages visually distinct with `telegram` badge + accent color
- do not change current Claude/Codex panels

- [ ] **Step 5: Verify frontend behavior**

Run:

```bash
bun test src/components/AgentStatus/TelegramPanel.test.tsx
bun run build
```

Expected: PASS; production build succeeds with new Telegram types/panel.

- [ ] **Step 6: Commit**

```bash
git add src/stores/telegram-store.ts \
  src/components/AgentStatus/TelegramPanel.tsx \
  src/components/AgentStatus/TelegramPanel.test.tsx \
  src/components/AgentStatus/index.tsx \
  src/components/MessagePanel/SourceBadge.tsx \
  src/components/MessagePanel/surface-styles.ts \
  src/types.ts
git commit -m "feat: add telegram settings and status panel"
```

### Task 5: Document the architecture change and verify the end-to-end flow

**Files:**
- Modify: `CLAUDE.md`
- Modify: `UPDATE.md`

- [ ] **Step 1: Update architecture docs**

Required documentation changes:
- add Telegram runtime to the current architecture diagram in `CLAUDE.md`
- describe long-poll pairing flow and outbound lead-report flow
- record phase-1 limitations in `CLAUDE.md`
- append implementation notes to `UPDATE.md`

- [ ] **Step 2: Run focused Rust verification**

```bash
cargo test --manifest-path src-tauri/Cargo.toml telegram:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml daemon::routing -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run frontend verification**

```bash
bun test src/components/AgentStatus/TelegramPanel.test.tsx
bun run build
```

Expected: PASS.

- [ ] **Step 4: Manual QA in a real Telegram bot sandbox**

Checklist:
- save a valid bot token
- generate pairing code in the panel
- send `/pair <code>` from the target Telegram DM
- verify the UI shows the bound chat
- send a plain Telegram text and confirm it appears in the GUI as `telegram` -> `lead`
- make lead send a terminal `to = "user"` result and confirm Telegram receives the report
- verify a summary artifact appears in `TaskPanel`
- stop the lead session, send another Telegram instruction, and confirm it buffers until lead is back

- [ ] **Step 5: Commit**

```bash
git add CLAUDE.md UPDATE.md
git commit -m "docs: document telegram remote control mvp"
```

## Implementation (2026-04-06)

Commit trail:

| Commit | Summary |
|--------|---------|
| `edd1770d` | feat: add telegram bot backend — config, pairing, inbound routing, outbound reports |
| `c6e576ed` | feat: add telegram settings panel and message badge |
| `66abc94c` | fix: commit telegram daemon integration wiring |
| `2738bce8` | fix: remove HTML parse_mode and fix unicode panic in truncate |
| `77726596` | refactor: extract telegram lifecycle helpers from daemon/mod.rs |
| `3183045c` | fix: require explicit token input in TelegramPanel save |

Code review findings addressed:
- CRITICAL: Missing daemon integration files committed
- CRITICAL: HTML injection removed (plain text sendMessage)
- CRITICAL: Unicode panic fixed (char-boundary-safe truncation)
- IMPORTANT: telegram_lifecycle.rs extracted (mod.rs -103 lines)
- IMPORTANT: Broken token save fallback removed

Test results: Rust 296 passed (13 new), Frontend 162 passed, 0 failures.

Known deviations from plan:
- `daemon/user_input.rs` shared helper not extracted — Telegram runtime directly constructs BridgeMessage and calls routing::route_message (functionally equivalent, simpler)
- `TelegramPanel.test.tsx` not created — runtime behavior tested via Rust unit tests; frontend panel is render-only
- Summary artifact creation not implemented — outbound reports are delivered but not persisted as task graph artifacts (Phase 2)
- CLAUDE.md / UPDATE.md updates deferred to separate doc commit

## Final Acceptance Criteria

- [x] A user can save a Telegram bot token in-app and see Telegram runtime status.
- [x] The app can pair exactly one private Telegram chat through a one-time code.
- [x] A plain text Telegram message from the paired chat is routed as `user -> lead` and shows in the GUI with a Telegram badge.
- [x] A terminal `lead -> user` reply automatically sends a Telegram report to the paired chat.
- [ ] Summary artifact creation (deferred to Phase 2).
- [x] Telegram delivery failures do not block the GUI route or corrupt the task graph.
- [ ] `CLAUDE.md` and `UPDATE.md` reflect the new runtime (deferred to doc commit).
