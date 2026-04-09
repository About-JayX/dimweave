# Feishu Project Bug Inbox Design

## Summary

Dimweave should gain an embedded Bug Inbox tool for a single Feishu Project workspace. The user opens the tool from a dedicated shell icon, sees a live list of Feishu Project work items, configures connection and sync parameters inside the same panel, and manually starts handling a selected item. Starting handling should create or resume exactly one Dimweave task for that work item, persist a local issue snapshot file, and seed the lead role with the issue context so the normal planning -> execution -> review -> CM workflow begins inside Dimweave.

The agreed V1 product choices are:

- source system: one Feishu Project workspace
- item scope: all work items in that workspace
- list behavior: one row per work item, updated in place
- launch model: half-automatic (`Start handling` is user-triggered)
- Dimweave task reuse: repeated starts reopen the existing linked task instead of creating duplicates
- Feishu write-back: excluded from V1
- ingress model: ~~polling baseline with webhook fast path~~ → **direct HTTP MCP** (see [MCP pivot design](2026-04-09-feishu-project-mcp-pivot-design.md))

## Product Goal

Turn Feishu Project into a first-class Dimweave intake source so the product can operate like this:

1. Feishu Project work item appears or changes
2. Dimweave Bug Inbox updates
3. User clicks `Start handling`
4. Dimweave creates/selects a task and hands the issue to lead
5. lead writes the repair plan
6. coder executes
7. lead reviews and closes out with CM evidence

## Scope

### Included

- A new shell tool entry for `Bug Inbox`
- Embedded configuration UI in the inbox panel
- Feishu Project workspace polling
- Feishu Project webhook ingestion
- Deduplicated work-item list persistence
- Idempotent `Start handling` orchestration that creates or resumes a linked Dimweave task
- Storing the source issue snapshot as a persisted local file for inspection and lead handoff

### Excluded

- Updating Feishu Project issue status/comments from Dimweave
- Multi-workspace support
- Feishu IM bot intake
- Cloud relay infrastructure operated by Dimweave
- Automatic lead launch the instant a work item changes

## Evidence and Constraints

### Feishu Project platform facts

- Feishu Project provides both **standard API** and **Webhook** capabilities; the 2024 version guide lists both as product capabilities and publishes daily quotas. Source: <https://www.feishu.cn/content/epjgdgdd>
- Feishu Project webhook/automation payloads include `header.event_type`, `header.token`, and an idempotency `header.uuid`. The webhook guide enumerates work-item events such as `WorkitemCreateEvent`, `WorkitemStatusEvent`, `WorkitemUpdateEvent`, `WorkitemCommentEvent`, and `WorkitemFinishEvent`, and it states webhook delivery is `POST`, times out after 6 seconds, and retries up to 3 times. Source: <https://www.feishu.cn/content/49fq0rvm>
- Feishu Project OpenAPI access to space data requires a **plugin token** installed into the target space; the FAQ explicitly says space APIs need a plugin token and that the plugin must be installed in the space. Source: <https://www.feishu.cn/content/60bl79n2>
- The same FAQ states `X-USER-KEY` can be obtained via API and that the acting user must have the corresponding space permissions. Source: <https://www.feishu.cn/content/60bl79n2>
- The FAQ states webhook source IPs are **dynamic** and cannot be pre-listed. Source: <https://www.feishu.cn/content/60bl79n2>
- The FAQ also states the “complex parameter” work-item list API cannot return the full space without filters; full-space listing should use the single-space list endpoint with the full set of work-item type keys. Source: <https://www.feishu.cn/content/60bl79n2>

### Current repo constraints

- Dimweave already has a left rail tool launcher in `src/components/ShellContextBar.tsx`, and side panels render inside `src/components/TaskContextPopover.tsx`. This is the correct extension point for a Bug Inbox tool.
- The desktop daemon already owns an axum server in `src-tauri/src/daemon/control/server.rs`, but it binds to `127.0.0.1:{port}`. That means Feishu cloud webhooks cannot reach the desktop app directly without a user-provided tunnel/public forwarder.
- Dimweave already has a durable config/runtime pattern for external integrations via the Telegram integration (`src-tauri/src/telegram/*`, `src/stores/telegram-store.ts`, `src/components/AgentStatus/TelegramPanel.tsx`).
- Dimweave already has a task system with task/session/artifact persistence and GUI snapshots (`src-tauri/src/daemon/task_graph/*`, `src/stores/task-store/*`).
- Existing user input routing stamps active-task context automatically when a task is selected (`src-tauri/src/daemon/routing_user_input.rs`), so the lead handoff can reuse normal user-message routing after task creation.
- `TaskContextPopover.tsx` uses a `paneMeta satisfies Record<ShellSidebarPane, ...>` map, so adding a `bugs` pane requires both shell-layout-type changes and a new `paneMeta.bugs` entry or TypeScript will fail.
- `ShellContextBar.tsx` currently only supports approval/message counts, so a Bug Inbox badge needs an explicit new prop and render path.
- `DaemonState` currently contains Telegram integration fields but no Feishu Project runtime state, so `src-tauri/src/daemon/state.rs` must be part of the change set.

## Design

### 1. Add a dedicated Bug Inbox shell pane

Extend the shell nav model with a new `bugs` pane and place it in the left rail alongside Task context, Agents, Approvals, and Logs.

The panel should live inside the existing `TaskContextPopover`, not as a floating modal, so it behaves like the other shell tools and preserves the current single-drawer interaction pattern.

### 2. Model Feishu Project as an integration runtime, not a one-off fetch

Create a new `feishu_project` Rust module that owns:

- persisted config (`project_key`, plugin token, user key, webhook token, polling interval, optional public webhook base URL, enabled flag)
- masked runtime state for the frontend
- persisted Bug Inbox records
- ingestion helpers for polling and webhook events

This mirrors the Telegram integration pattern and keeps all external-system logic on the Rust side.

### 3. Use polling as the guaranteed baseline and webhook as the fast path

Because the desktop app only listens on localhost today, direct Feishu webhook delivery is not reachable from the public internet. Therefore:

- polling is the guaranteed ingestion path and must always work on its own
- webhook support is still implemented, but requires the user to expose the local daemon path through a tunnel or another public forwarder and paste that public base URL into config

This preserves the chosen `C` architecture in a way that is actually feasible for the current desktop codebase.

### 4. Persist one logical inbox row per work item

Each Feishu work item should map to exactly one Bug Inbox record keyed by a stable work-item identifier. New poll/webhook payloads update that record in place instead of appending new rows.

Each record should store:

- Feishu identifiers (`work_item_id`, `project_key`, `type_key`)
- title/status/assignee/update time
- source URL
- latest payload snapshot path
- last ingress source (`poll` or `webhook`)
- last event UUID
- `ignored` flag
- optional `linked_task_id`

Dimweave-facing workflow state should be derived from `linked_task_id` and the linked task status where possible, so the inbox does not fork a second workflow state machine.

### 5. Keep issue snapshots as persisted files first, not task artifacts first

`TaskGraphStore::add_artifact()` requires a `session_id`, but creating/selecting a task does not create a lead session by itself. Therefore V1 should persist the latest Feishu work-item snapshot as a standalone markdown/JSON file referenced by the inbox record, and attach that file to the seeded lead handoff message.

If later we want the same file mirrored into task artifacts, that should happen only after a real task session exists. That mirror is explicitly out of V1 scope.

### 6. Make `Start handling` idempotent

When the user clicks `Start handling`:

- if the record already has a `linked_task_id` and that task still exists, select that task and do not create another one
- otherwise create a new Dimweave task in the currently selected workspace
- persist the latest work-item snapshot as a local file if it has not already been materialized
- route a structured message to `lead`

The seeded lead message should include:

- Feishu issue title and link
- current status/assignee
- latest known summary/body excerpt
- the snapshot file as an attachment when available
- a short instruction that this is a repair task originating from Feishu Project and should follow Dimweave’s plan -> execute -> review -> CM flow

The delivery path should use a normal routed `BridgeMessage` with `from: "system"` and `display_source: Some("feishu_project")` so the timeline correctly shows the source instead of making the handoff look like user-typed text.

### 7. Integrate polling into the daemon lifecycle, not as an ad-hoc helper

The polling loop should follow the Telegram pattern:

- a dedicated runtime handle owned in `src-tauri/src/daemon/mod.rs`
- start/stop/restart logic inside `daemon/feishu_project_lifecycle.rs`
- persisted runtime state updates emitted to the frontend

This avoids orphan background tasks and keeps integration lifecycle control inside the daemon instead of scattering it across command handlers.

### 8. Keep Feishu read-only in V1

V1 should not update Feishu Project status or comments. The inbox is a source-of-truth intake feed plus Dimweave task launcher. This reduces permission surface, avoids bidirectional state drift, and matches the explicit product decision.

## UI Structure

```md
Bug Inbox
├─ Connection card
│  ├─ Enabled toggle
│  ├─ Project key
│  ├─ Plugin token (masked)
│  ├─ User key
│  ├─ Webhook token
│  ├─ Poll interval
│  ├─ Public webhook base URL (optional tunnel URL)
│  └─ Sync now
├─ Runtime status strip
│  ├─ Connected / error
│  ├─ Last poll time
│  ├─ Last webhook time
│  └─ Last error
└─ Work item list
   ├─ Search box
   ├─ Filters: All / Unlinked / In progress / Ignored
   └─ Rows
      ├─ title
      ├─ type + status
      ├─ assignee
      ├─ updated time
      ├─ Dimweave status badge
      ├─ Open in Feishu
      ├─ Start handling
      └─ Ignore / Unignore
```

## Data Model

### Frontend runtime state

```ts
type FeishuProjectRuntimeState = {
  enabled: boolean;
  projectKey: string | null;
  tokenLabel: string | null;
  userKey: string | null;
  pollIntervalMinutes: number;
  publicWebhookBaseUrl: string | null;
  localWebhookPath: string;
  lastPollAt: number | null;
  lastWebhookAt: number | null;
  lastSyncAt: number | null;
  lastError: string | null;
  webhookEnabled: boolean;
};
```

### Inbox record

```ts
type FeishuProjectInboxItem = {
  recordId: string;
  projectKey: string;
  workItemId: string;
  workItemTypeKey: string;
  title: string;
  statusLabel: string | null;
  assigneeLabel: string | null;
  updatedAt: number;
  sourceUrl: string;
  rawSnapshotRef: string;
  ignored: boolean;
  linkedTaskId: string | null;
  lastIngress: "poll" | "webhook";
  lastEventUuid: string | null;
};
```

## File Map

### Frontend

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

### Rust / Tauri

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

## Testing Strategy

- Rust unit tests for config round trips, record upsert/idempotency, webhook token validation, polling pagination merge behavior, and task-link deduplication
- Rust integration tests for webhook route payload ingestion and polling merge behavior
- Frontend tests for shell-rail rendering, panel rendering, config-state display, and `Start handling` button behavior
- Targeted Bun tests for the new store and panel view model
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts`
- `bun run build`

## Acceptance Criteria

- A new Bug Inbox icon appears in the left shell rail and opens an embedded side panel.
- The panel lets the user configure one Feishu Project workspace and sync parameters inside Dimweave.
- Polling alone can fetch and display the workspace’s work items.
- Webhook events update the same list records in place when the user has exposed the local webhook path through a public forwarder.
- The list shows all workspace work items, not just issues/bugs.
- Clicking `Start handling` creates exactly one linked Dimweave task for that work item and hands the context to lead.
- Clicking `Start handling` again on the same row reopens the existing linked task instead of creating a duplicate.
- Feishu issue handoff appears in the timeline as a Feishu/system-sourced message, not as if the user typed it.
- V1 does not write changes back to Feishu Project.
