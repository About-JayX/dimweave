# Feishu Project Bug Inbox Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Dimweave Bug Inbox tool that ingests one Feishu Project workspace, lists all work items, and lets the user start an idempotent lead-driven repair workflow from any row.

**Architecture:** Implement a new Feishu Project integration runtime inside Tauri using polling as the guaranteed baseline and webhook ingestion as the fast path. Surface the integration through a new shell drawer pane, persist one inbox record per work item, and reuse the existing task graph plus normal user-to-lead routing when the user clicks `Start handling`.

**Tech Stack:** React 19, TypeScript, Zustand, Tauri 2, Rust, axum, reqwest, tokio, Bun, Cargo

---

## Baseline Notes

- The desktop daemon’s control server currently binds to `127.0.0.1` in `src-tauri/src/daemon/control/server.rs`. Public Feishu webhook delivery therefore requires a user-supplied tunnel/public forwarder in V1.
- Dimweave already has the correct extension seams for a new tool rail item (`ShellContextBar`) and embedded drawer panel (`TaskContextPopover`).
- Dimweave already has a durable integration pattern in Telegram (`src-tauri/src/telegram/*`) and a reusable task launch path in `daemon_create_task` + `daemon_send_user_input`.

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
- Modify: `src-tauri/src/daemon/gui.rs`
- Modify: `src-tauri/src/daemon/control/server.rs`
- Create: `src-tauri/src/daemon/control/feishu_project_webhook.rs`
- Create: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Create: `src-tauri/src/commands_feishu_project.rs`
- Create: `src-tauri/src/feishu_project/mod.rs`
- Create: `src-tauri/src/feishu_project/config.rs`
- Create: `src-tauri/src/feishu_project/types.rs`
- Create: `src-tauri/src/feishu_project/api.rs`
- Create: `src-tauri/src/feishu_project/runtime.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `feat: add feishu project inbox runtime model` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::`; `git diff --check` | The integration must mirror Telegram’s config/runtime split so secrets stay Rust-side and the frontend only sees masked state. |
| Task 2 | `feat: ingest feishu project work items via poll and webhook` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project_webhook`; `cargo test --manifest-path src-tauri/Cargo.toml` | Desktop localhost cannot receive public webhooks directly, so polling must remain the guaranteed baseline while webhook acts as an optional fast path. |
| Task 3 | `feat: add bug inbox shell panel` | `bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts`; `bun run build`; `git diff --check` | The Bug Inbox must feel like a native shell tool, not a separate modal or hidden settings screen. |
| Task 4 | `feat: launch linked dimweave tasks from bug inbox` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::task_link`; `bun run build`; `cargo test --manifest-path src-tauri/Cargo.toml` | `Start handling` must be idempotent: one Feishu work item maps to one Dimweave task unless the user explicitly builds a different workflow later. |

## Task 1: Add the Feishu Project runtime model, config persistence, and command surface

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/gui.rs`
- Create: `src-tauri/src/commands_feishu_project.rs`
- Create: `src-tauri/src/feishu_project/mod.rs`
- Create: `src-tauri/src/feishu_project/config.rs`
- Create: `src-tauri/src/feishu_project/types.rs`
- Create: `src-tauri/src/daemon/feishu_project_lifecycle.rs`

- [ ] **Step 1: Write the failing Rust tests first**

Create the new `src-tauri/src/feishu_project/types.rs` and `config.rs` tests before implementation:

```rust
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

        let state = FeishuProjectRuntimeState::from_config(&cfg, "/integrations/feishu-project/webhook");
        assert_eq!(state.project_key.as_deref(), Some("manciyuan"));
        assert_eq!(state.token_label.as_deref(), Some("plugi***"));
        assert_eq!(state.local_webhook_path, "/integrations/feishu-project/webhook");
    }
}
```

```rust
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

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::
```

Expected: FAIL because the new module/types/functions do not exist yet.

- [ ] **Step 3: Implement config, runtime state, and daemon commands**

Add a new integration module patterned after Telegram:

```rust
// src-tauri/src/feishu_project/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FeishuProjectConfig {
    pub enabled: bool,
    pub project_key: String,
    pub plugin_token: String,
    pub user_key: String,
    pub webhook_token: String,
    pub poll_interval_minutes: u64,
    pub public_webhook_base_url: Option<String>,
    pub last_poll_at: Option<u64>,
    pub last_webhook_at: Option<u64>,
    pub last_sync_at: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeishuProjectRuntimeState {
    pub enabled: bool,
    pub project_key: Option<String>,
    pub token_label: Option<String>,
    pub user_key: Option<String>,
    pub poll_interval_minutes: u64,
    pub public_webhook_base_url: Option<String>,
    pub local_webhook_path: String,
    pub last_poll_at: Option<u64>,
    pub last_webhook_at: Option<u64>,
    pub last_sync_at: Option<u64>,
    pub last_error: Option<String>,
    pub webhook_enabled: bool,
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

- [ ] **Step 4: Register commands and emit a frontend event channel**

Wire the command module into Tauri and add a GUI emitter:

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
// src-tauri/src/daemon/gui.rs
pub fn emit_feishu_project_state(
    app: &AppHandle,
    state: &crate::feishu_project::types::FeishuProjectRuntimeState,
) {
    let _ = app.emit("feishu_project_state", state.clone());
}
```

- [ ] **Step 5: Run Rust verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::
git diff --check
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/main.rs src-tauri/src/daemon/cmd.rs src-tauri/src/daemon/mod.rs src-tauri/src/daemon/gui.rs src-tauri/src/commands_feishu_project.rs src-tauri/src/feishu_project src-tauri/src/daemon/feishu_project_lifecycle.rs
git commit -m "feat: add feishu project inbox runtime model"
```

- [ ] **Step 7: Update `## CM Memory` with the real commit SHA after review**

## Task 2: Ingest Feishu Project work items through polling and webhook upserts

**Files:**
- Modify: `src-tauri/src/daemon/control/server.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Create: `src-tauri/src/daemon/control/feishu_project_webhook.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/feishu_project/mod.rs`
- Create: `src-tauri/src/feishu_project/api.rs`
- Create: `src-tauri/src/feishu_project/runtime.rs`

- [ ] **Step 1: Write failing Rust tests for record upsert and webhook token validation**

Create tests like:

```rust
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
```

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

- [ ] **Step 2: Run targeted Rust tests and confirm red**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project_webhook
```

Expected: FAIL because the runtime/store/webhook helpers do not exist yet.

- [ ] **Step 3: Implement persisted inbox records and poller**

Implement polling around the Feishu Project API constraints:

```rust
// src-tauri/src/feishu_project/runtime.rs
pub async fn run_poll_cycle(
    client: &reqwest::Client,
    cfg: &FeishuProjectConfig,
    store: &mut FeishuProjectStore,
) -> anyhow::Result<()> {
    let type_keys = api::list_work_item_type_keys(client, cfg).await?;
    let items = api::list_work_items(client, cfg, &type_keys).await?;
    for item in items {
        store.upsert(item);
    }
    Ok(())
}
```

Use the FAQ-backed full-space strategy:

```rust
// src-tauri/src/feishu_project/api.rs
pub async fn list_work_items(
    client: &reqwest::Client,
    cfg: &FeishuProjectConfig,
    work_item_type_keys: &[String],
) -> anyhow::Result<Vec<FeishuProjectInboxItem>> {
    let body = serde_json::json!({
        "project_key": cfg.project_key,
        "work_item_type_keys": work_item_type_keys,
        "page_size": 200
    });

    let response = client
        .post("https://project.feishu.cn/open_api/list")
        .header("X-PLUGIN-TOKEN", &cfg.plugin_token)
        .header("X-USER-KEY", &cfg.user_key)
        .json(&body)
        .send()
        .await?;

    parse_work_item_page(response).await
}
```

- [ ] **Step 4: Add webhook route and merge path**

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

- [ ] **Step 5: Emit runtime/list updates to the frontend**

When poll or webhook updates records, emit both runtime state and item list:

```rust
// src-tauri/src/daemon/gui.rs
pub fn emit_feishu_project_items(
    app: &AppHandle,
    items: &[crate::feishu_project::types::FeishuProjectInboxItem],
) {
    let _ = app.emit("feishu_project_items", items.to_vec());
}
```

- [ ] **Step 6: Run backend verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project_webhook
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/daemon/control/server.rs src-tauri/src/daemon/control/feishu_project_webhook.rs src-tauri/src/daemon/mod.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project
git commit -m "feat: ingest feishu project work items via poll and webhook"
```

- [ ] **Step 8: Update `## CM Memory` with the real commit SHA after review**

## Task 3: Add the Bug Inbox shell icon, store, panel UI, and config controls

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

- [ ] **Step 3: Implement the frontend store and panel**

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

Wire a new nav item:

```ts
// src/components/shell-layout-state.ts
export type ShellSidebarPane = "task" | "bugs" | "agents" | "approvals";
export type ShellNavItem = ShellSidebarPane | "logs";
```

```tsx
// src/components/ShellContextBar.tsx
const NAV_ITEMS = [
  { id: "task", label: "Task context", icon: Workflow },
  { id: "bugs", label: "Bug Inbox", icon: Bug },
  { id: "agents", label: "Agents", icon: Bot },
  { id: "approvals", label: "Approvals", icon: AlertTriangle },
  { id: "logs", label: "Logs", icon: TerminalSquare },
];
```

- [ ] **Step 4: Render the drawer contents**

Add the new pane to `TaskContextPopover`:

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

And implement the panel composition:

```tsx
// src/components/BugInboxPanel/index.tsx
export function BugInboxPanel() {
  const state = useFeishuProjectStore((s) => s.state);
  const items = useFeishuProjectStore((s) => s.items);
  const fetchState = useFeishuProjectStore((s) => s.fetchState);
  const fetchItems = useFeishuProjectStore((s) => s.fetchItems);

  useEffect(() => {
    void fetchState();
    void fetchItems();
  }, [fetchState, fetchItems]);

  return (
    <div className="space-y-3">
      <ConfigCard state={state} />
      <IssueList items={items} />
    </div>
  );
}
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

## Task 4: Launch or reopen linked Dimweave tasks from Bug Inbox rows

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
fn start_handling_seeds_lead_prompt_for_new_task() {
    let prompt = build_lead_handoff_prompt(&sample_item());
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

Implement the lifecycle:

```rust
pub async fn start_handling(
    state: &SharedState,
    app: &AppHandle,
    work_item_id: &str,
) -> Result<String, String> {
    let (task_id, prompt) = {
        let mut daemon = state.write().await;
        let item = daemon.feishu_project_runtime.require_item_mut(work_item_id)?;
        if let Some(existing) = item.linked_task_id.clone() {
            daemon.select_task(&existing)?;
            return Ok(existing);
        }

        let task = daemon.create_and_select_task(
            &daemon.active_workspace_root()?,
            &format!("[Feishu {}] {}", item.work_item_id, item.title),
        );
        item.linked_task_id = Some(task.task_id.clone());
        let prompt = build_lead_handoff_prompt(item);
        (task.task_id, prompt)
    };

    crate::daemon::routing_user_input::route_user_input(
        state,
        app,
        prompt,
        "lead".into(),
        None,
    )
    .await;

    Ok(task_id)
}
```

- [ ] **Step 4: Persist the source issue as a task artifact**

When creating a new link, also write a markdown snapshot:

```rust
let snapshot_path = write_issue_snapshot_markdown(item)?;
daemon.task_graph.add_artifact(CreateArtifactParams {
    task_id: &task.task_id,
    session_id: lead_session_id,
    kind: ArtifactKind::Research,
    title: "Feishu Project issue snapshot",
    content_ref: &snapshot_path,
});
```

- [ ] **Step 5: Wire the row action in the UI**

Use the store action from the row button:

```tsx
<button
  className="rounded-md border border-primary/40 px-2 py-1 text-[11px] text-primary hover:bg-primary/10"
  onClick={() => void startHandling(item.workItemId)}
>
  {item.linkedTaskId ? "Open task" : "Start handling"}
</button>
```

- [ ] **Step 6: Run end-to-end verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::task_link
cargo test --manifest-path src-tauri/Cargo.toml
bun run build
git diff --check
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands_feishu_project.rs src-tauri/src/daemon/cmd.rs src-tauri/src/daemon/mod.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project/runtime.rs src/components/BugInboxPanel/IssueList.tsx src/stores/feishu-project-store.ts
git commit -m "feat: launch linked dimweave tasks from bug inbox"
```

- [ ] **Step 8: Update `## CM Memory` with the real commit SHA after review**

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
   - lead receives the seeded issue context
   - the row now shows a linked task status
5. Click Start handling again on the same row and confirm the existing task reopens instead of duplicating
```

- [ ] **Step 3: Final review commit (docs only if needed)**

If implementation notes changed during execution, update this plan/spec and commit the documentation delta before merge.
