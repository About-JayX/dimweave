# Feishu Project Bug Inbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Dimweave Bug Inbox tool that ingests one Feishu Project workspace, lists all work items, and lets the user start an idempotent lead-driven repair workflow from any row.

**Architecture:** Implement a new Feishu Project integration runtime inside Tauri using polling as the guaranteed baseline and webhook ingestion as the optional fast path. Surface the integration through a new shell drawer pane, persist one inbox record per work item, and create/reuse linked Dimweave tasks from inbox rows. Seed lead with a system-sourced Feishu handoff message plus an attached issue snapshot file; do not create task artifacts before a real task session exists.

**Tech Stack:** React 19, TypeScript, Zustand, Tauri 2, Rust, axum, reqwest, tokio, Bun, Cargo

---

## Baseline Notes

- The desktop daemon’s control server currently binds to `127.0.0.1` in `src-tauri/src/daemon/control/server.rs`. Public Feishu webhook delivery therefore requires a user-supplied tunnel/public forwarder in V1.
- Feishu Project webhook docs state deliveries are `POST`, include `header.token` + `header.uuid`, time out after 6 seconds, and retry up to 3 times. The webhook handler therefore needs fast acknowledgement and idempotent merge logic. Source: <https://www.feishu.cn/content/49fq0rvm>
- Dimweave already has the correct extension seams for a new tool rail item (`ShellContextBar`) and embedded drawer panel (`TaskContextPopover`).
- Dimweave already has a durable integration pattern in Telegram (`src-tauri/src/telegram/*`) and a reusable task launch path in `daemon_create_task`.
- `TaskContextPopover.tsx` uses a `paneMeta satisfies Record<ShellSidebarPane, ...>` map; adding a `bugs` pane requires both shell-layout-type changes and a new `paneMeta.bugs` entry or TypeScript will fail.
- `ShellContextBar.tsx` currently only supports approval/message counts, so a Bug Inbox badge needs an explicit new prop and render path.
- `DaemonState` currently has Telegram integration fields but no Feishu Project runtime fields, so `src-tauri/src/daemon/state.rs` must be updated in Task 1.

## File Map

### Frontend shell + panel

- Modify: `src/App.tsx`
- Modify: `src/components/shell-layout-state.ts`
- Modify: `src/components/ShellContextBar.tsx`
- Modify: `src/components/TaskContextPopover.tsx`
- Create: `src/components/BugInboxPanel/index.tsx`
- Create: `src/components/BugInboxPanel/ConfigCard.tsx`
- Create: `src/components/BugInboxPanel/IssueList.tsx`
- Create: `src/components/BugInboxPanel/view-model.ts`
- Create: `src/stores/feishu-project-store.ts`

### Frontend tests

- Modify: `src/components/ShellContextBar.test.tsx`
- Modify: `src/components/TaskContextPopover.test.tsx`
- Create: `src/components/BugInboxPanel/index.test.tsx`
- Create: `src/stores/feishu-project-store.test.ts`

### Rust integration runtime

- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/daemon/gui.rs`
- Modify: `src-tauri/src/daemon/control/server.rs`
- Create: `src-tauri/src/daemon/control/feishu_project_webhook.rs`
- Create: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Create: `src-tauri/src/commands_feishu_project.rs`
- Create: `src-tauri/src/feishu_project/mod.rs`
- Create: `src-tauri/src/feishu_project/config.rs`
- Create: `src-tauri/src/feishu_project/types.rs`
- Create: `src-tauri/src/feishu_project/store.rs`
- Create: `src-tauri/src/feishu_project/api.rs`
- Create: `src-tauri/src/feishu_project/runtime.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `feat: add feishu project inbox runtime model` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::`; `git diff --check` | `3876d21a` — The integration mirrors Telegram’s config/runtime split so secrets stay Rust-side and the frontend only sees masked state. |
| Task 2 | `feat: poll feishu project work items into inbox` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::polling`; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::store`; `git diff --check` | `1abe81a3` — Polling is the guaranteed baseline; the API client pages through the workspace, merges rows in place, and surfaces a runtime warning if the filter API truncates results beyond 2000 items. |
| Task 3 | `feat: accept feishu project webhooks into inbox` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project_webhook`; `cargo test --manifest-path src-tauri/Cargo.toml`; `git diff --check` | Webhook is an optional fast path and must validate the configured token, dedupe on `header.uuid`, and return quickly. |
| Task 4 | `feat: add bug inbox shell panel` | `bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts`; `bun run build`; `git diff --check` | The Bug Inbox must feel like a native shell tool, not a separate modal or hidden settings screen. |
| Task 5 | `feat: launch linked dimweave tasks from bug inbox` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::task_link`; `cargo test --manifest-path src-tauri/Cargo.toml`; `bun run build`; `git diff --check` | `Start handling` must be idempotent: one Feishu work item maps to one Dimweave task unless the user explicitly changes that workflow later. |

## Task 1: Add the Feishu Project runtime model, config persistence, state field, and command surface

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/daemon/gui.rs`
- Create: `src-tauri/src/commands_feishu_project.rs`
- Create: `src-tauri/src/feishu_project/mod.rs`
- Create: `src-tauri/src/feishu_project/config.rs`
- Create: `src-tauri/src/feishu_project/types.rs`
- Create: `src-tauri/src/feishu_project/store.rs`
- Create: `src-tauri/src/daemon/feishu_project_lifecycle.rs`

- [ ] **Step 1: Write the failing Rust tests first**

Create the new runtime/config/store tests before implementation:

```rust
// src-tauri/src/feishu_project/types.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_state_masks_plugin_token() {
        let cfg = FeishuProjectConfig {
            enabled: true,
            project_key: "manciyuan".into(),
            plugin_token: "plugin_secret_123".into(),
            user_key: "u_123".into(),
            webhook_token: "hook_456".into(),
            poll_interval_minutes: 10,
            public_webhook_base_url: Some("https://abc.ngrok.app".into()),
            ..Default::default()
        };

        let state =
            FeishuProjectRuntimeState::from_config(&cfg, "/integrations/feishu-project/webhook");
        assert_eq!(state.project_key.as_deref(), Some("manciyuan"));
        assert_eq!(state.token_label.as_deref(), Some("plugi***"));
        assert_eq!(state.local_webhook_path, "/integrations/feishu-project/webhook");
    }
}
```

```rust
// src-tauri/src/feishu_project/config.rs
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dimweave_feishu_project_cfg_{name}_{}_{}.json",
            std::process::id(),
            chrono::Utc::now().timestamp_millis(),
        ))
    }

    #[test]
    fn config_round_trip_preserves_tokens_and_poll_interval() {
        let cfg = FeishuProjectConfig {
            enabled: true,
            project_key: "manciyuan".into(),
            plugin_token: "plugin_123".into(),
            user_key: "u_123".into(),
            webhook_token: "hook_123".into(),
            poll_interval_minutes: 15,
            public_webhook_base_url: Some("https://abc.ngrok.app".into()),
            ..Default::default()
        };

        let path = temp_path("round_trip");
        save_config(&path, &cfg).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.project_key, "manciyuan");
        assert_eq!(loaded.plugin_token, "plugin_123");
        assert_eq!(loaded.poll_interval_minutes, 15);
        let _ = std::fs::remove_file(path);
    }
}
```

```rust
// src-tauri/src/feishu_project/store.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upsert_item_updates_existing_record_instead_of_appending() {
        let mut store = FeishuProjectStore::default();
        store.upsert(FeishuProjectInboxItem {
            work_item_id: "1001".into(),
            title: "Crash on launch".into(),
            status_label: Some("Open".into()),
            updated_at: 10,
            ..sample_item()
        });
        store.upsert(FeishuProjectInboxItem {
            work_item_id: "1001".into(),
            title: "Crash on launch (updated)".into(),
            status_label: Some("In Progress".into()),
            updated_at: 20,
            ..sample_item()
        });

        assert_eq!(store.items.len(), 1);
        assert_eq!(store.items[0].title, "Crash on launch (updated)");
        assert_eq!(store.items[0].status_label.as_deref(), Some("In Progress"));
    }
}
```

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::
```

Expected: FAIL because the new module/types/functions do not exist yet.

- [ ] **Step 3: Implement config, runtime state, store, and daemon commands**

Add a new integration module patterned after Telegram and persist config to the app config directory:

```rust
// src-tauri/src/feishu_project/config.rs
use super::types::FeishuProjectConfig;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base.join("com.dimweave.app").join("feishu_project.json"))
}

pub fn load_config(path: &Path) -> anyhow::Result<FeishuProjectConfig> {
    if !path.exists() {
        return Ok(FeishuProjectConfig::default());
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

pub fn save_config(path: &Path, cfg: &FeishuProjectConfig) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(cfg)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}
```

```rust
// src-tauri/src/daemon/cmd.rs
GetFeishuProjectState {
    reply: oneshot::Sender<crate::feishu_project::types::FeishuProjectRuntimeState>,
},
SaveFeishuProjectConfig {
    config: crate::feishu_project::types::FeishuProjectConfig,
    reply: oneshot::Sender<Result<crate::feishu_project::types::FeishuProjectRuntimeState, String>>,
},
SyncFeishuProjectNow {
    reply: oneshot::Sender<Result<(), String>>,
},
ListFeishuProjectItems {
    reply: oneshot::Sender<Vec<crate::feishu_project::types::FeishuProjectInboxItem>>,
},
StartFeishuProjectHandling {
    work_item_id: String,
    reply: oneshot::Sender<Result<String, String>>,
},
SetFeishuProjectIgnored {
    work_item_id: String,
    ignored: bool,
    reply: oneshot::Sender<Result<(), String>>,
},
```

```rust
// src-tauri/src/commands_feishu_project.rs
#[tauri::command]
pub async fn feishu_project_get_state(
    sender: State<'_, DaemonSender>,
) -> Result<crate::feishu_project::types::FeishuProjectRuntimeState, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::GetFeishuProjectState { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())
}
```

- [ ] **Step 4: Register commands, add state fields, and add GUI emitters**

Wire the command module into Tauri and add the runtime fields to `DaemonState`:

```rust
// src-tauri/src/main.rs
mod commands_feishu_project;
mod feishu_project;
```

```rust
.invoke_handler(tauri::generate_handler![
    // ...
    commands_feishu_project::feishu_project_get_state,
    commands_feishu_project::feishu_project_save_config,
    commands_feishu_project::feishu_project_sync_now,
    commands_feishu_project::feishu_project_list_items,
    commands_feishu_project::feishu_project_start_handling,
    commands_feishu_project::feishu_project_set_ignored,
])
```

```rust
// src-tauri/src/daemon/state.rs
pub struct DaemonState {
    // ...
    pub feishu_project_runtime: crate::feishu_project::runtime::FeishuProjectRuntime,
}
```

```rust
// src-tauri/src/daemon/gui.rs
pub fn emit_feishu_project_state(
    app: &AppHandle,
    state: &crate::feishu_project::types::FeishuProjectRuntimeState,
) {
    let _ = app.emit("feishu_project_state", state.clone());
}

pub fn emit_feishu_project_items(
    app: &AppHandle,
    items: &[crate::feishu_project::types::FeishuProjectInboxItem],
) {
    let _ = app.emit("feishu_project_items", items.to_vec());
}
```

Integrate a managed `FeishuProjectHandle` in `src-tauri/src/daemon/mod.rs`, mirroring `telegram_handle`, so config saves can start/stop/restart the poller instead of spawning detached loops.

- [ ] **Step 5: Run Rust verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::
git diff --check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/main.rs src-tauri/src/daemon/cmd.rs src-tauri/src/daemon/mod.rs src-tauri/src/daemon/state.rs src-tauri/src/daemon/gui.rs src-tauri/src/commands_feishu_project.rs src-tauri/src/feishu_project src-tauri/src/daemon/feishu_project_lifecycle.rs
git commit -m "feat: add feishu project inbox runtime model"
```

- [ ] **Step 7: Update `## CM Memory` with the real commit SHA after review**

## Task 2: Poll Feishu Project work items into the inbox store

**Files:**
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/feishu_project/mod.rs`
- Modify: `src-tauri/src/feishu_project/store.rs`
- Create: `src-tauri/src/feishu_project/api.rs`
- Create: `src-tauri/src/feishu_project/runtime.rs`

- [ ] **Step 1: Verify the current Feishu Project OpenAPI endpoint shape before coding**

Read the current official Feishu Project OpenAPI docs/FAQ and record the verified list/search endpoints, pagination fields, and work-item-type discovery path in code comments or test fixtures.

Run:

```bash
printf '%s\n' "Verify current Feishu Project work-item list/search endpoint, pagination fields, and type-key listing before implementing api.rs."
```

Expected: A concrete endpoint plan written into the implementation notes or code comments. Do **not** hardcode an unverified path such as `https://project.feishu.cn/open_api/list`.

- [ ] **Step 2: Write failing Rust tests for polling pagination and store merge**

Create tests like:

```rust
// src-tauri/src/feishu_project/runtime.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn poll_cycle_merges_multiple_pages() {
        let client = fake_client_with_pages(vec![page_one(), page_two()]);
        let cfg = sample_config();
        let mut store = FeishuProjectStore::default();

        run_poll_cycle(&client, &cfg, &mut store).await.unwrap();

        assert_eq!(store.items.len(), 3);
    }
}
```

- [ ] **Step 3: Run targeted Rust tests and confirm red**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::polling
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::store
```

Expected: FAIL because the poller/API/store helpers do not exist yet.

- [ ] **Step 4: Implement paginated polling**

Write the poller so it iterates work-item type keys and pages until exhaustion:

```rust
// src-tauri/src/feishu_project/runtime.rs
pub async fn run_poll_cycle(
    client: &reqwest::Client,
    cfg: &FeishuProjectConfig,
    store: &mut FeishuProjectStore,
) -> anyhow::Result<()> {
    let type_keys = api::list_work_item_type_keys(client, cfg).await?;
    for type_key in type_keys {
        let mut page_token = None;
        loop {
            let page =
                api::list_work_items_page(client, cfg, &type_key, page_token.as_deref()).await?;
            for item in page.items {
                store.upsert(item);
            }
            if !page.has_more {
                break;
            }
            page_token = page.next_page_token;
        }
    }
    Ok(())
}
```

Implement the API layer behind a verified endpoint resolver:

```rust
// src-tauri/src/feishu_project/api.rs
pub async fn list_work_items_page(
    client: &reqwest::Client,
    cfg: &FeishuProjectConfig,
    work_item_type_key: &str,
    page_token: Option<&str>,
) -> anyhow::Result<WorkItemPage> {
    let response = client
        .post(resolve_verified_work_item_endpoint(work_item_type_key))
        .header("X-PLUGIN-TOKEN", &cfg.plugin_token)
        .header("X-USER-KEY", &cfg.user_key)
        .json(&build_search_body(&cfg.project_key, page_token))
        .send()
        .await?;
    parse_work_item_page(response).await
}
```

- [ ] **Step 5: Integrate the poller into daemon lifecycle**

Start/stop/restart the managed `FeishuProjectHandle` from `save_config` and `sync_now`, following the Telegram lifecycle pattern instead of detached ad-hoc tasks.

- [ ] **Step 6: Run backend verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::polling
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::store
git diff --check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/daemon/mod.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project/api.rs src-tauri/src/feishu_project/store.rs src-tauri/src/feishu_project/runtime.rs
git commit -m "feat: poll feishu project work items into inbox"
```

- [ ] **Step 8: Update `## CM Memory` with the real commit SHA after review**

## Task 3: Accept Feishu Project webhooks into the inbox

**Files:**
- Modify: `src-tauri/src/daemon/control/server.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Create: `src-tauri/src/daemon/control/feishu_project_webhook.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/feishu_project/runtime.rs`

- [ ] **Step 1: Write failing Rust tests for webhook token validation and UUID dedupe**

Create tests like:

```rust
#[test]
fn webhook_rejects_invalid_token() {
    let config = FeishuProjectConfig {
        webhook_token: "expected_token".into(),
        ..Default::default()
    };
    let payload = serde_json::json!({
        "header": { "token": "wrong_token", "uuid": "u1", "event_type": "WorkitemUpdateEvent" },
        "payload": {}
    });

    let result = validate_webhook_token(&config, &payload);
    assert!(result.is_err());
}
```

```rust
#[test]
fn webhook_uuid_is_idempotent() {
    let mut runtime = FeishuProjectRuntime::default();
    let payload = sample_webhook_payload("uuid-1");
    runtime.ingest_webhook_payload(&payload).unwrap();
    runtime.ingest_webhook_payload(&payload).unwrap();
    assert_eq!(runtime.items.len(), 1);
}
```

- [ ] **Step 2: Run targeted Rust tests and confirm red**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project_webhook
```

Expected: FAIL because the webhook helpers do not exist yet.

- [ ] **Step 3: Add the webhook route**

Expose a local webhook route on the existing axum server:

```rust
// src-tauri/src/daemon/control/server.rs
.route(
    "/integrations/feishu-project/webhook",
    post(feishu_project_webhook::handle),
)
```

```rust
// src-tauri/src/daemon/control/feishu_project_webhook.rs
pub async fn handle(
    State((state, app)): State<(SharedState, AppHandle)>,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    match crate::daemon::feishu_project_lifecycle::ingest_webhook(&state, &app, payload).await {
        Ok(()) => StatusCode::OK,
        Err(_) => StatusCode::UNAUTHORIZED,
    }
}
```

This webhook path is based on the documented Feishu Project automation webhook body format (`header.token`, `header.uuid`) and does **not** require a Feishu app `challenge` handshake unless implementation evidence shows a different connector path is actually being used.

- [ ] **Step 4: Merge webhook payloads into the same inbox store**

Use the same `store.upsert()` path as polling and dedupe on `header.uuid`.

- [ ] **Step 5: Run backend verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project_webhook
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/daemon/control/server.rs src-tauri/src/daemon/control/feishu_project_webhook.rs src-tauri/src/daemon/mod.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project/runtime.rs
git commit -m "feat: accept feishu project webhooks into inbox"
```

- [ ] **Step 7: Update `## CM Memory` with the real commit SHA after review**

## Task 4: Add the Bug Inbox shell icon, store, panel UI, and config controls

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/shell-layout-state.ts`
- Modify: `src/components/ShellContextBar.tsx`
- Modify: `src/components/TaskContextPopover.tsx`
- Create: `src/components/BugInboxPanel/index.tsx`
- Create: `src/components/BugInboxPanel/ConfigCard.tsx`
- Create: `src/components/BugInboxPanel/IssueList.tsx`
- Create: `src/components/BugInboxPanel/view-model.ts`
- Create: `src/stores/feishu-project-store.ts`
- Modify: `src/components/ShellContextBar.test.tsx`
- Modify: `src/components/TaskContextPopover.test.tsx`
- Create: `src/components/BugInboxPanel/index.test.tsx`
- Create: `src/stores/feishu-project-store.test.ts`

- [ ] **Step 1: Write failing frontend tests first**

Create the shell/panel assertions before implementation:

```ts
test("renders Bug Inbox in the shell rail", async () => {
  installTauriStub();
  const { ShellContextBar } = await import("./ShellContextBar");
  const html = renderToStaticMarkup(
    <ShellContextBar
      activeItem={null}
      approvalCount={0}
      bugCount={5}
      messageCount={0}
      runtimeHealth={null}
      themeMode="auto"
      radiusMode="rounded"
      onToggle={() => {}}
      onThemeChange={() => {}}
      onRadiusToggle={() => {}}
    />,
  );

  expect(html).toContain("Bug Inbox");
  expect(html).toContain("5");
});
```

```ts
test("renders the bug inbox drawer pane", async () => {
  installTauriStub();
  const { TaskContextPopover } = await import("./TaskContextPopover");
  const html = renderToStaticMarkup(
    <TaskContextPopover activePane="bugs" onClose={() => {}} task={null} />,
  );

  expect(html).toContain("Bug Inbox");
  expect(html).toContain("Connection");
});
```

- [ ] **Step 2: Run the targeted frontend tests to verify they fail**

Run:

```bash
bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx
```

Expected: FAIL because `bugs` pane support does not exist yet.

- [ ] **Step 3: Implement the frontend store and rail badge**

Create a dedicated store that mirrors the Telegram pattern:

```ts
// src/stores/feishu-project-store.ts
export interface FeishuProjectInboxItem {
  recordId: string;
  workItemId: string;
  title: string;
  statusLabel: string | null;
  assigneeLabel: string | null;
  updatedAt: number;
  sourceUrl: string;
  ignored: boolean;
  linkedTaskId: string | null;
}

export const useFeishuProjectStore = create<FeishuProjectStore>((set) => ({
  state: null,
  items: [],
  loading: false,
  error: null,
  fetchState: async () => { /* invoke("feishu_project_get_state") */ },
  fetchItems: async () => { /* invoke("feishu_project_list_items") */ },
  saveConfig: async (config) => { /* invoke("feishu_project_save_config") */ },
  syncNow: async () => { /* invoke("feishu_project_sync_now") */ },
  startHandling: async (workItemId) => { /* invoke("feishu_project_start_handling") */ },
  setIgnored: async (workItemId, ignored) => { /* invoke("feishu_project_set_ignored") */ },
}));
```

Add the new nav item and explicit badge prop:

```ts
// src/components/shell-layout-state.ts
export type ShellSidebarPane = "task" | "bugs" | "agents" | "approvals";
export type ShellNavItem = ShellSidebarPane | "logs";
```

```tsx
interface ShellContextBarProps {
  activeItem: ShellNavItem | null;
  approvalCount: number;
  bugCount: number;
  messageCount: number;
  runtimeHealth: RuntimeHealthInfo | null;
  // ...
}
```

- [ ] **Step 4: Render the drawer contents and add `paneMeta.bugs`**

Add the new pane to `TaskContextPopover`:

```tsx
const paneMeta = {
  task: { eyebrow: "Task context", title: task?.title ?? "Task workspace", icon: Workflow },
  bugs: { eyebrow: "Bug Inbox", title: "Feishu Project work items", icon: Bug },
  agents: { eyebrow: "Agents", title: "Runtime control", icon: Bot },
  approvals: { eyebrow: "Approvals", title: "Permission queue", icon: AlertTriangle },
} satisfies Record<ShellSidebarPane, { eyebrow: string; title: string; icon: typeof Workflow }>;
```

```tsx
{mountedPanes.includes("bugs") && (
  <div
    className={cn(
      "h-full overflow-y-auto px-4 py-4 text-[12px] text-muted-foreground/78",
      activePane === "bugs" ? "block" : "hidden",
    )}
  >
    <BugInboxPanel />
  </div>
)}
```

- [ ] **Step 5: Run frontend verification**

Run:

```bash
bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts
bun run build
git diff --check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/components/shell-layout-state.ts src/components/ShellContextBar.tsx src/components/TaskContextPopover.tsx src/components/BugInboxPanel src/stores/feishu-project-store.ts src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts
git commit -m "feat: add bug inbox shell panel"
```

- [ ] **Step 7: Update `## CM Memory` with the real commit SHA after review**

## Task 5: Launch or reopen linked Dimweave tasks from Bug Inbox rows

**Files:**
- Modify: `src-tauri/src/commands_feishu_project.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src/components/BugInboxPanel/IssueList.tsx`
- Modify: `src/stores/feishu-project-store.ts`

- [ ] **Step 1: Write failing task-link tests first**

Create Rust tests like:

```rust
#[test]
fn start_handling_reuses_existing_linked_task() {
    let mut runtime = FeishuProjectRuntime::default();
    runtime.items.push(FeishuProjectInboxItem {
        work_item_id: "1001".into(),
        linked_task_id: Some("task_1".into()),
        ..sample_item()
    });

    let result = runtime.resolve_task_link("1001");
    assert_eq!(result, Some("task_1".into()));
}
```

```rust
#[test]
fn build_lead_handoff_message_mentions_repair_plan() {
    let prompt = build_lead_handoff_message(&sample_item(), "/tmp/issue.md");
    assert!(prompt.contains("Feishu Project"));
    assert!(prompt.contains("Start by writing a repair plan"));
    assert!(prompt.contains("Crash on launch"));
}
```

- [ ] **Step 2: Run targeted tests and verify red**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::task_link
```

Expected: FAIL because the task-link helpers do not exist yet.

- [ ] **Step 3: Implement idempotent `start handling` lifecycle**

Add a daemon command that owns task linking:

```rust
// src-tauri/src/daemon/cmd.rs
StartFeishuProjectHandling {
    work_item_id: String,
    reply: oneshot::Sender<Result<String, String>>,
},
```

Implement the lifecycle without holding a mutable borrow to one runtime item while also mutating the daemon:

```rust
pub async fn start_handling(
    state: &SharedState,
    app: &AppHandle,
    work_item_id: &str,
) -> Result<String, String> {
    let (task_id, handoff, attachment_path) = {
        let mut daemon = state.write().await;
        let active_task_id = daemon
            .active_task_id
            .clone()
            .ok_or_else(|| "no active workspace task selected".to_string())?;
        let workspace = daemon
            .task_graph
            .get_task(&active_task_id)
            .map(|task| task.workspace_root.clone())
            .ok_or_else(|| "active workspace task missing".to_string())?;

        if let Some(existing) = daemon.feishu_project_runtime.linked_task_id_for(work_item_id) {
            daemon.select_task(&existing)?;
            return Ok(existing);
        }

        let snapshot = daemon.feishu_project_runtime.snapshot_for(work_item_id)?.clone();
        let task = daemon.create_and_select_task(
            &workspace,
            &format!("[Feishu {}] {}", snapshot.work_item_id, snapshot.title),
        );
        daemon
            .feishu_project_runtime
            .set_linked_task_id(work_item_id, &task.task_id)?;
        let attachment_path = write_issue_snapshot_markdown(&snapshot)?;
        let handoff = build_lead_handoff_message(&snapshot, &attachment_path);
        (task.task_id, handoff, attachment_path)
    };

    crate::daemon::routing::route_message(
        state,
        app,
        crate::daemon::types::BridgeMessage {
            id: uuid::Uuid::new_v4().to_string(),
            from: "system".into(),
            display_source: Some("feishu_project".into()),
            to: "lead".into(),
            content: handoff,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            reply_to: None,
            priority: None,
            status: None,
            task_id: Some(task_id.clone()),
            session_id: None,
            sender_agent_id: None,
            attachments: Some(vec![crate::daemon::types::Attachment {
                file_path: attachment_path,
                file_name: "feishu-issue.md".into(),
                is_image: false,
                media_type: Some("text/markdown".into()),
            }]),
        },
    )
    .await;

    Ok(task_id)
}
```

- [ ] **Step 4: Wire the row action in the UI**

Use the store action from the row button:

```tsx
<button
  className="rounded-md border border-primary/40 px-2 py-1 text-[11px] text-primary hover:bg-primary/10"
  onClick={() => void startHandling(item.workItemId)}
>
  {item.linkedTaskId ? "Open task" : "Start handling"}
</button>
```

- [ ] **Step 5: Run end-to-end verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::task_link
cargo test --manifest-path src-tauri/Cargo.toml
bun run build
git diff --check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands_feishu_project.rs src-tauri/src/daemon/cmd.rs src-tauri/src/daemon/mod.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project/runtime.rs src/components/BugInboxPanel/IssueList.tsx src/stores/feishu-project-store.ts
git commit -m "feat: launch linked dimweave tasks from bug inbox"
```

- [ ] **Step 7: Update `## CM Memory` with the real commit SHA after review**

## Final Verification

- [ ] **Step 1: Run the complete backend/frontend verification bundle**

```bash
cargo test --manifest-path src-tauri/Cargo.toml
bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts
bun run build
git diff --check
```

- [ ] **Step 2: Manual workflow verification**

Verify manually:

```md
1. Save Feishu Project config in Bug Inbox
2. Trigger Sync now and confirm rows appear
3. POST a sample webhook payload to /integrations/feishu-project/webhook through the local tunnel and confirm the same row updates in place
4. Click Start handling on an unlinked row and confirm:
   - a Dimweave task is selected
   - the timeline shows a Feishu/system-sourced handoff to lead
   - the lead handoff includes the issue snapshot attachment
   - the row now shows a linked task status
5. Click Start handling again on the same row and confirm the existing task reopens instead of duplicating
```

- [ ] **Step 3: Final review commit (docs only if needed)**

If implementation notes changed during execution, update this plan/spec and commit the documentation delta before merge.
