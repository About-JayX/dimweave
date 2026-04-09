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
- The “connect MCP in AI tools” page includes a concrete **Claude Code stdio example**:
  - `command: "npx"`
  - `args: ["-y", "@lark-project/mcp", "--domain", "{domain}"]`
  - `env: { "MCP_USER_TOKEN": "" }`
- The same page shows Codex CLI can scope MCP config at the **project level** via `.codex/config.toml`
- Task 0 investigation proved `@lark-project/mcp` is a **stdio-to-HTTP proxy**, not the actual MCP server. The actual MCP tools are served remotely from `https://project.feishu.cn/mcp_server/v1`.
- Task 0 also proved the npm package requires `MCP_USER_TOKEN` and does **not** implement an interactive OAuth/browser login flow.

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
- Dimweave also already has PATH enrichment and local process launch patterns (`claude_cli.rs`, Codex runtime lifecycle), which makes **app-managed npm package execution** more realistic than a user-managed global install.

## Recommended Connection Strategy

### Option A: Direct HTTP MCP client (**recommended**)

**Pros**
- Eliminates Node.js runtime dependency entirely
- Eliminates npm package management and subprocess lifecycle
- Talks directly to the real MCP endpoint the npm package proxies to
- Best fit for the user's “app-managed / no global install” requirement
- Best fit for the current Rust/Tauri architecture

**Cons**
- Requires implementing MCP over HTTP / StreamableHTTP in Rust
- Still requires `MCP_USER_TOKEN`

### Option B: App-managed npm proxy + stdio (**fallback only**) 

**Pros**
- We know the concrete stdio shape Feishu documents for Claude Code:
  - `npx -y @lark-project/mcp --domain {domain}`
  - `MCP_USER_TOKEN` in env
- Can still satisfy the “no global install” requirement if Dimweave manages the package itself

**Cons**
- Adds Node.js dependency
- Adds npm package management
- Adds subprocess lifecycle complexity even though the package is only a proxy
- Strictly worse than direct HTTP if the direct endpoint is stable

### Option C: HTTP OAuth-first

**Pros**
- Friendly authorization UX if Feishu documents a stable OAuth path for the remote MCP endpoint

**Cons**
- More moving parts than direct HTTP + token
- Not yet evidenced by Task 0
- Still requires implementing a remote MCP HTTP client anyway

## Recommendation

**Use a direct HTTP MCP client as the new V1 primary architecture.**

Rationale:

- Task 0 proved the documented stdio package is only a proxy to the remote HTTP MCP endpoint
- direct HTTP removes the Node/npm/global-install question entirely
- it best satisfies the user's hard requirement that the MCP integration be internal to the project/app
- it fits the current Rust/Tauri desktop architecture
- it lets us preserve almost all of the existing Bug Inbox UI and task-launch workflow
- it leaves room to add `HTTP OAuth` later as a better auth UX without changing the core inbox model

This recommendation is now grounded in Task 0 evidence, but we still cannot complete the final inbox adapter until we have a real `MCP_USER_TOKEN` and real `tools/list` output. The remaining unknowns are:

- how `domain` and `MCP_USER_TOKEN` are obtained from the Feishu Project MCP settings UI
- the real live `tools/list` catalog

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
- HTTP OAuth implementation in this first pivot
- any final design that requires the user to globally install the Feishu Project MCP server by hand
- Feishu write-back redesign beyond what the MCP path naturally enables later

## Design

### 1. Separate the integration into three layers

```md
Feishu Project remote MCP endpoint
    -> HTTP MCP client session
    -> Feishu capability adapter
    -> Bug Inbox domain store
    -> existing UI + task-launch flow
```

This keeps protocol concerns away from the inbox store and preserves the ability to swap `stdio` for `HTTP OAuth` later.

### 2. Add a general-purpose Feishu MCP HTTP client runtime

New runtime responsibilities:

- connect to the remote Feishu MCP endpoint
- perform `initialize`
- fetch and cache `tools/list`
- expose a small internal API like:
  - `connect()`
  - `disconnect()`
  - `list_tools()`
  - `call_tool(name, input)`

This is the real architectural replacement for the old `api.rs + runtime.rs` token path.

This client is materially more complex than the old REST poller. It must own:

- JSON-RPC request/response correlation by `id`
- HTTP request construction against the MCP endpoint
- response parsing
- connection/auth state handling
- request timeout / retry boundaries
- clean shutdown of inflight state

It should be treated as a first-class runtime subsystem, closer to a remote transport client than to the previous REST poll helper.

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

- endpoint domain (default `https://project.feishu.cn`)
- `MCP_USER_TOKEN`
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

The token is still required, but it is now a **user-level MCP token**, not the old space-plugin token path.

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

- `src-tauri/src/feishu_project/mcp_http.rs`
- `src-tauri/src/feishu_project/mcp_client.rs`
- `src-tauri/src/feishu_project/tool_catalog.rs`
- `src-tauri/src/feishu_project/mcp_sync.rs`
- `src-tauri/src/commands_feishu_project.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/main.rs`

### Removed from primary path (completed Task 4)

- `src-tauri/src/feishu_project/api.rs` — deprecated in-place; kept for serde compat and test coverage of pagination logic
- `src-tauri/src/daemon/control/feishu_project_webhook.rs` — **deleted**; webhook route removed from `control/server.rs`
- `src-tauri/src/daemon/control/server.rs` — webhook route registration removed

## Acceptance Criteria

- Bug Inbox no longer depends on plugin token/user key as its primary access path
- The user can configure an Feishu Project MCP connection in Dimweave
- Dimweave can discover the available Feishu Project MCP tools
- Bug Inbox can populate from MCP-fetched work items
- Existing `Handle` / `Open task` behavior still works
- lead still receives a Feishu-sourced handoff with snapshot attachment
- If the MCP endpoint is unauthorized, unavailable, or lacks required tools, Dimweave surfaces a clear runtime error state in the Bug Inbox UI
- The final shipped solution does not require the user to globally install the Feishu Project MCP server outside Dimweave
