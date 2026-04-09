# Feishu Project MCP Pivot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current token/webhook-first Feishu Project data-access layer with an MCP-first integration while preserving the existing Bug Inbox UI and linked task-handling workflow.

**Architecture:** Add a Feishu Project MCP client runtime in Tauri, start with `stdio` transport as the primary MVP path after validating the real stdio launch contract and live tool catalog, and map the discovered read capabilities into the existing Bug Inbox domain store. Retain the current Bug Inbox UI shell and task-link workflow, but replace the config model and remote sync path.

**Tech Stack:** React 19, TypeScript, Zustand, Tauri 2, Rust, tokio, subprocess stdio, serde_json, Bun, Cargo

---

## Baseline Notes

- The current token/webhook implementation is already committed and working, but it is not a good fit for this user because they cannot obtain project-space tokens.
- The Bug Inbox UI and task-link path should be preserved as much as possible.
- The new unknowns are:
  - the exact Feishu Project MCP stdio launch command and authentication behavior
  - the exact `tools/list` catalog and JSON schemas
- The implementation must therefore include an explicit discovery/diagnostics stage instead of hardcoding all tool names up front.

## Scope

### Keep

- Bug Inbox shell icon and drawer
- work-item list rendering
- ignore/restore
- `Handle` / `Open task`
- snapshot file persistence
- lead handoff routing

### Replace

- token config UI
- OpenAPI polling runtime
- webhook-first sync model

## File Map

### Frontend

- Modify: `src/components/BugInboxPanel/ConfigCard.tsx`
- Modify: `src/stores/feishu-project-store.ts`
- Modify: `src/components/BugInboxPanel/index.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx`
- Modify: `src/stores/feishu-project-store.test.ts`

### Rust

- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/commands_feishu_project.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/control/server.rs`
- Modify: `src-tauri/src/feishu_project/types.rs`
- Modify: `src-tauri/src/feishu_project/config.rs`
- Replace: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Create: `src-tauri/src/feishu_project/mcp_client.rs`
- Create: `src-tauri/src/feishu_project/mcp_stdio.rs`
- Create: `src-tauri/src/feishu_project/tool_catalog.rs`
- Create: `src-tauri/src/feishu_project/mcp_sync.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 0 | `docs: capture feishu project mcp catalog` | `git diff --check`; saved artifact with real `tools/list` response | The pivot must start from a real catalog and validated stdio launch contract, not guesses. |
| Task 1 | `feat: add feishu project mcp stdio runtime` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp`; `git diff --check` | The pivot must first prove we can connect, initialize, and list tools before we rewrite the inbox sync path. |
| Task 2 | `feat: switch bug inbox config to mcp connection` | `bun test src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts`; `bun run build`; `git diff --check` | The UI must stop asking for inaccessible tokens and instead expose MCP connection status and controls. |
| Task 3 | `feat: source bug inbox from feishu mcp` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync`; `cargo test --manifest-path src-tauri/Cargo.toml`; `git diff --check` | Tool discovery should drive the adapter; do not hardcode final tool IDs before seeing the real catalog. |
| Task 4 | `refactor: retire legacy feishu token sync path` | `cargo test --manifest-path src-tauri/Cargo.toml`; `bun run build`; `git diff --check` | The old path should be clearly demoted or removed so we do not maintain two conflicting primary architectures. |

## Task 0: Capture the real Feishu Project MCP stdio launch contract and tool catalog

**Files:**
- Create: `docs/agents/feishu-project-mcp-tool-catalog.json`
- Create: `docs/agents/feishu-project-mcp-notes.md`

- [ ] **Step 1: Connect manually to the real Feishu Project MCP server**

Use the real Feishu Project MCP stdio launch command and capture:

- how it is launched
- whether it requires an interactive login step
- the raw `initialize` response
- the raw `tools/list` response

- [ ] **Step 2: Save the evidence artifact**

Write the raw catalog JSON to:

```bash
docs/agents/feishu-project-mcp-tool-catalog.json
```

and record the launch/auth notes in:

```bash
docs/agents/feishu-project-mcp-notes.md
```

- [ ] **Step 3: Commit**

```bash
git add docs/agents/feishu-project-mcp-tool-catalog.json docs/agents/feishu-project-mcp-notes.md
git commit -m "docs: capture feishu project mcp catalog"
```

## Task 1: Add Feishu Project MCP stdio runtime and tool discovery

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/feishu_project/types.rs`
- Modify: `src-tauri/src/feishu_project/config.rs`
- Replace: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Create: `src-tauri/src/feishu_project/mcp_client.rs`
- Create: `src-tauri/src/feishu_project/mcp_stdio.rs`
- Create: `src-tauri/src/feishu_project/tool_catalog.rs`

- [ ] **Step 1: Replace the config model with MCP connection config**

Move from REST-token config to MCP connection config:

```rust
pub enum FeishuProjectConnectionMode {
    Stdio,
    HttpOAuth,
}

pub struct FeishuProjectConfig {
    pub enabled: bool,
    pub connection_mode: FeishuProjectConnectionMode,
    pub workspace_hint: Option<String>,
    pub stdio_command: String,
    pub stdio_args: Vec<String>,
    pub refresh_interval_minutes: u64,
    pub last_error: Option<String>,
}
```

- [ ] **Step 2: Write failing tests for stdio client handshake and runtime state**

Add tests that prove:

- initialize request formatting
- parsing `tools/list` results
- request/response correlation by `id`
- EOF / child exit surfaces a disconnected state
- runtime state reflects connection failure

- [ ] **Step 3: Implement MCP stdio transport**

The transport needs these pieces first:

- spawn configured command
- continuously read child stdout in a pump loop
- continuously serialize writes to child stdin
- match JSON-RPC responses by `id`
- surface child exit / EOF

Do not collapse this into a single helper function.

- [ ] **Step 4: Implement MCP client request lifecycle**

Layer a client API on top of the transport:

- `connect()`
- `disconnect()`
- `initialize()`
- `list_tools()`
- `call_tool()`

with:

- timeouts
- inflight request map
- clean shutdown

- [ ] **Step 5: Add diagnostics surface in runtime state**

Expose:

- connected / disconnected
- last error
- discovered tool count
- maybe tool names preview

- [ ] **Step 6: Verify**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp
git diff --check
```

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/feishu_project/types.rs src-tauri/src/feishu_project/config.rs src-tauri/src/feishu_project/runtime.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project/mcp_client.rs src-tauri/src/feishu_project/mcp_stdio.rs src-tauri/src/feishu_project/tool_catalog.rs
git commit -m "feat: add feishu project mcp stdio runtime"
```

## Task 2: Switch Bug Inbox config UI to MCP connection

**Files:**
- Modify: `src/components/BugInboxPanel/ConfigCard.tsx`
- Modify: `src/stores/feishu-project-store.ts`
- Modify: `src/components/BugInboxPanel/index.tsx`
- Modify: `src/components/BugInboxPanel/index.test.tsx`
- Modify: `src/stores/feishu-project-store.test.ts`

- [ ] **Step 1: Replace token fields with MCP connection fields**

For the first MCP pivot pass, support:

- enabled
- connection mode (default `stdio`)
- stdio command
- stdio args
- workspace hint
- refresh interval

- [ ] **Step 2: Show connection and catalog status**

The card should show:

- connected/disconnected
- discovered tools count
- last error
- last sync
- clear error when the connection recovers

- [ ] **Step 3: Verify**

Run:

```bash
bun test src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts
bun run build
git diff --check
```

- [ ] **Step 4: Commit**

```bash
git add src/components/BugInboxPanel/ConfigCard.tsx src/stores/feishu-project-store.ts src/components/BugInboxPanel/index.tsx src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts
git commit -m "feat: switch bug inbox config to mcp connection"
```

## Task 3: Source Bug Inbox from Feishu MCP

**Files:**
- Modify: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Create: `src-tauri/src/feishu_project/mcp_sync.rs`

- [ ] **Step 1: Bind the adapter to the captured real tool catalog**

Use the saved catalog from Task 0 and record which tool(s) provide:

- workspace metadata
- work-item listing
- work-item detail/meta required for snapshots

- [ ] **Step 2: Write failing tests for adapter mapping**

Tests should prove:

- MCP result rows map into `FeishuProjectInboxItem`
- repeated sync still preserves `ignored` and `linkedTaskId`
- missing required tools becomes a surfaced runtime error
- malformed or partial MCP tool output produces a surfaced runtime error

- [ ] **Step 3: Implement MCP-based sync**

Replace the REST polling path with:

- `connect if needed`
- `call required MCP read tools`
- map results into inbox items
- upsert into local store

- [ ] **Step 4: Keep existing task-link workflow unchanged**

Do not rewrite `Handle` / `Open task` logic here; it should continue consuming the same inbox item model.

- [ ] **Step 5: Verify**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync
cargo test --manifest-path src-tauri/Cargo.toml
git diff --check
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/feishu_project/runtime.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project/mcp_sync.rs
git commit -m "feat: source bug inbox from feishu mcp"
```

## Task 4: Retire the legacy Feishu token sync path

**Files:**
- Modify: `src-tauri/src/feishu_project/api.rs`
- Modify: `src-tauri/src/daemon/control/server.rs`
- Modify: `src-tauri/src/daemon/control/feishu_project_webhook.rs`
- Modify: `docs/superpowers/specs/2026-04-09-feishu-project-bug-inbox-design.md`
- Modify: `docs/superpowers/plans/2026-04-09-feishu-project-bug-inbox.md`
- Modify: any obsolete runtime files as needed

- [ ] **Step 1: Remove or clearly demote the old REST/webhook path**

Do one of:

- remove the old path entirely if the MCP path is proven stable, or
- clearly mark it as legacy/fallback and keep it out of the main UI

Also explicitly remove the Feishu webhook route from `control/server.rs` if it is no longer part of the primary product path.

- [ ] **Step 2: Verify**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
bun run build
git diff --check
```

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-04-09-feishu-project-mcp-pivot-design.md docs/superpowers/plans/2026-04-09-feishu-project-mcp-pivot.md
git commit -m "refactor: retire legacy feishu token sync path"
```
