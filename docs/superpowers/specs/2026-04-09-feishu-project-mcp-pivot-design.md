# Feishu Project MCP Pivot Design

## Summary

Dimweave's current Feishu Project Bug Inbox is built around direct OpenAPI polling plus webhook ingestion. That architecture assumes the user can obtain plugin tokens, user keys, and webhook configuration authority for the target project space. The user has now clarified that this is a company-managed Feishu Project workspace and they do **not** have token access.

Feishu Project's official help center now documents an official **Feishu Project MCP Server** that operates within the user's personal permissions and supports `HTTP OAuth`, `HTTP Header`, and `stdio` connection modes. That makes the current token/webhook-first architecture the wrong primary integration surface for this user.

The new direction is:

- keep the **Bug Inbox UI**
- keep the **Handle -> lead -> coder -> review -> CM** workflow
- replace the **Feishu data access layer** with a **Feishu Project MCP client**
- require the MCP integration to be **app-managed / project-bundled**, not a globally installed prerequisite

## Product Goal

Turn Bug Inbox into an MCP-powered Feishu Project workspace browser and launcher:

1. Dimweave connects to the official Feishu Project MCP Server
2. Dimweave reads the user's permitted Feishu Project data through MCP tools
3. Bug Inbox shows workspace work items
4. The user clicks `Handle` / `Open task`
5. Dimweave creates or reopens the linked task
6. lead receives the Feishu-sourced handoff and starts the normal repair workflow

## Why Pivot

### New evidence from the user-provided Feishu Project help center doc

The user provided the following verified product facts from Feishu Project help center:

- Feishu Project MCP Server is an **official MCP service**
- It works **within the user's personal permission scope**
- It supports `HTTP OAuth`, `HTTP Header`, and `stdio`
- It exposes read/write operations for work items, spaces, flows, comments, views, and related metadata

That changes the architecture tradeoff:

- **OpenAPI token path** requires admin/plugin setup that the user lacks
- **MCP path** is explicitly designed to avoid that token bottleneck

### Current codebase facts

- The existing Bug Inbox UI is already working and should be preserved:
  - `src/components/BugInboxPanel/*`
  - `src/stores/feishu-project-store.ts`
- The current direct Feishu integration is localized to Rust-side files and can be replaced:
  - `src-tauri/src/feishu_project/*`
  - `src-tauri/src/daemon/feishu_project_lifecycle.rs`
  - `src-tauri/src/daemon/control/feishu_project_webhook.rs`
- Dimweave already manages external local processes and connection lifecycles:
  - Claude / Codex runtimes
  - MCP registration helpers in `src-tauri/src/mcp.rs`
  - OAuth-style launch/cancel patterns in `src-tauri/src/codex/oauth.rs`
- Dimweave already knows how to ship and resolve app-managed helper binaries (`resolve_release_bridge_cmd()` in `src-tauri/src/mcp.rs`), so bundling an MCP-side helper is consistent with existing packaging patterns.

## Recommended Connection Strategy

### Option A: HTTP OAuth first

**Pros**
- Matches Feishu's most user-friendly path
- No manual token entry
- Best long-term UX

**Cons**
- Requires implementing an MCP-over-HTTP client plus OAuth handshake
- Harder to ship quickly in the current Rust/Tauri codebase

### Option B: Stdio first, OAuth later (**recommended, pending validation**) 

**Pros**
- Best fit for current Dimweave runtime model
- Reuses existing subprocess lifecycle patterns
- Avoids plugin token dependency immediately
- Can be driven by an app-managed bundled Feishu MCP stdio launch command

**Cons**
- Requires us to prove the Feishu Project MCP stdio server can be bundled or vendored into the app/package
- Less turnkey than OAuth

### Option C: Keep token/webhook as primary and add MCP as optional fallback

**Pros**
- Reuses already implemented code

**Cons**
- Solves the wrong problem for this user
- Keeps the bad dependency on unavailable admin credentials
- Leaves two competing primary code paths

## Recommendation

**Use Stdio-first MCP as the new V1 primary architecture, but only after we verify the real Feishu Project stdio launch contract, bundling model, and live tool catalog.**

Rationale:

- it removes the token blocker immediately
- it fits the current Tauri desktop/process-management model
- it lets us preserve almost all of the existing Bug Inbox UI and task-launch workflow
- it leaves room to add `HTTP OAuth` later as a better UX layer without changing the core inbox model

This recommendation is an engineering inference from the current codebase and the user-provided Feishu Project MCP feature description. It is **not yet enough** to hardcode the stdio launch command or tool names. Those must be captured from a real Feishu Project MCP connection before implementation locks the adapter. It is also **not yet proven** that the stdio server can be shipped as an app-managed dependency instead of requiring global installation; that is now a hard validation gate.

## Scope

### Included

- replacing direct Feishu OpenAPI polling with MCP-based work-item reads
- replacing token-based config UI with MCP connection config UI
- preserving Bug Inbox rows, ignore state, and linked-task workflow
- preserving task launch and lead handoff flow
- adding MCP connection diagnostics and tool discovery
- enforcing an app-managed MCP packaging model instead of a global install prerequisite

### Excluded

- Feishu Project direct OpenAPI polling as the main path
- Feishu Project webhook as the main path
- HTTP OAuth implementation in this first pivot, unless the tool catalog proves stdio is unavailable
- any final design that requires the user to globally install the Feishu Project MCP server by hand
- Feishu write-back redesign beyond what the MCP path naturally enables later

## Design

### 1. Separate the integration into three layers

```md
Feishu Project MCP transport
    -> MCP client session
    -> Feishu capability adapter
    -> Bug Inbox domain store
    -> existing UI + task-launch flow
```

This keeps protocol concerns away from the inbox store and preserves the ability to swap `stdio` for `HTTP OAuth` later.

### 2. Add a general-purpose Feishu MCP client runtime

New runtime responsibilities:

- spawn/connect to the app-managed Feishu MCP transport
- perform `initialize`
- fetch and cache `tools/list`
- expose a small internal API like:
  - `connect()`
  - `disconnect()`
  - `list_tools()`
  - `call_tool(name, input)`

This is the real architectural replacement for the old `api.rs + runtime.rs` token path.

This client is materially more complex than the old REST poller. It must own:

- a bidirectional stdio pump loop
- JSON-RPC request/response correlation by `id`
- serialized writes to child stdin
- notification handling
- child-process health monitoring
- reconnect / shutdown cleanup behavior

It should be treated as a first-class runtime subsystem, closer to the existing Codex WS client lifecycle than to a simple helper module.

### 3. Add a capability adapter on top of tool discovery

Because the user-provided help center excerpt describes Feishu Project MCP capabilities semantically rather than giving us exact tool IDs and JSON schemas, the implementation should not hardcode all final tool names before connection.

Instead:

- first connect and read `tools/list`
- resolve required inbox capabilities from the discovered catalog
- then bind the inbox sync path to the actual tool names/schemas present

The adapter must not guess tool names purely from optimism. Before implementation, we need a captured real tool catalog artifact and an explicit matching strategy. The matching strategy should be one of:

- exact known tool IDs from the captured catalog
- schema-backed matching with tool name fallback
- user-assisted mapping if the catalog is unexpectedly ambiguous

Required capability classes for Bug Inbox:

- workspace/work-item listing
- work-item metadata retrieval
- work-item detail retrieval sufficient to build the inbox row + snapshot

### 4. Preserve the existing Bug Inbox UI shell

The current left-rail icon and drawer pane should remain. Only the config and backend synchronization logic change.

The new configuration card should ask for:

- connection mode (`stdio` initially, but app-managed)
- target Feishu Project workspace selection or identifier
- refresh interval
- connection status
- discovered tool status
- auth / authorization status

The old fields should be retired from the primary UI:

- plugin token
- user key
- webhook token
- public webhook base URL
- raw stdio command / path / args fields exposed to the user

If the stdio transport requires internal launch metadata, it should live in app code or packaging config, not in user-facing settings.

### 5. Keep linked-task and handoff logic

The existing idempotent `Handle` behavior remains valid:

- if `linked_task_id` exists and is valid, reopen it
- if stale, create a new task and relink
- write a snapshot file
- route a Feishu-sourced handoff to lead

This workflow is independent of whether the source data came from OpenAPI or MCP.

### 6. Preserve local inbox persistence

The inbox should still persist locally so:

- ignore state survives restart
- linked-task relationships survive restart
- snapshots survive restart

Only the **source of remote truth** changes from OpenAPI/webhook to MCP.

## Data Model Impact

### Keep

- `FeishuProjectInboxItem`
- `linkedTaskId`
- `ignored`
- snapshot file paths

### Change

- `FeishuProjectConfig` should become MCP connection config, not REST token config
- runtime state should expose connection/authorization/tool-discovery fields instead of token/webhook fields

## File Direction

### Keep / adapt

- `src/components/BugInboxPanel/*`
- `src/stores/feishu-project-store.ts`
- `src-tauri/src/daemon/feishu_project_task_link.rs`
- `src-tauri/src/feishu_project/store.rs`

### Replace or heavily rewrite

- `src-tauri/src/feishu_project/config.rs`
- `src-tauri/src/feishu_project/types.rs`
- `src-tauri/src/feishu_project/runtime.rs`
- `src-tauri/src/daemon/feishu_project_lifecycle.rs`

### Add

- `src-tauri/src/feishu_project/mcp_client.rs`
- `src-tauri/src/feishu_project/mcp_stdio.rs`
- `src-tauri/src/feishu_project/tool_catalog.rs`
- `src-tauri/src/feishu_project/mcp_sync.rs`
- `src-tauri/src/commands_feishu_project.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/main.rs`
- any app-bundled MCP binary wrapper or packaged helper needed to avoid global installation

### Remove from primary path

- `src-tauri/src/feishu_project/api.rs`
- `src-tauri/src/daemon/control/feishu_project_webhook.rs`
- `src-tauri/src/daemon/control/server.rs` route registration for Feishu webhook

## Acceptance Criteria

- Bug Inbox no longer depends on plugin token/user key as its primary access path
- The user can configure an Feishu Project MCP connection in Dimweave
- Dimweave can discover the available Feishu Project MCP tools
- Bug Inbox can populate from MCP-fetched work items
- Existing `Handle` / `Open task` behavior still works
- lead still receives a Feishu-sourced handoff with snapshot attachment
- If the MCP server is missing, disconnected, or lacks required tools, Dimweave surfaces a clear runtime error state in the Bug Inbox UI
- The runtime can shut down cleanly without leaving orphan MCP subprocesses behind
- The final shipped solution does not require the user to globally install the Feishu Project MCP server outside Dimweave
