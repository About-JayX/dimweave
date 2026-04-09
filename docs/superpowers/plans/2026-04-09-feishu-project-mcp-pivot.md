# Feishu Project MCP Pivot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current token/webhook-first Feishu Project data-access layer with an MCP-first integration while preserving the existing Bug Inbox UI and linked task-handling workflow.

**Architecture:** Add a Feishu Project MCP client runtime in Tauri, start with `stdio` transport as the primary MVP path, discover the actual tool catalog at runtime, and map the discovered read capabilities into the existing Bug Inbox domain store. Retain the current Bug Inbox UI shell and task-link workflow, but replace the config model and remote sync path.

**Tech Stack:** React 19, TypeScript, Zustand, Tauri 2, Rust, tokio, subprocess stdio, serde_json, Bun, Cargo

---

## Baseline Notes

- The current token/webhook implementation is already committed and working, but it is not a good fit for this user because they cannot obtain project-space tokens.
- The Bug Inbox UI and task-link path should be preserved as much as possible.
- The new unknown is the exact Feishu Project MCP tool schema; the implementation must therefore include an explicit discovery/diagnostics stage instead of hardcoding all tool names up front.

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

- Modify: `src-tauri/src/commands_feishu_project.rs`
- Modify: `src-tauri/src/feishu_project/types.rs`
- Modify: `src-tauri/src/feishu_project/config.rs`
- Modify: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Create: `src-tauri/src/feishu_project/mcp_client.rs`
- Create: `src-tauri/src/feishu_project/mcp_stdio.rs`
- Create: `src-tauri/src/feishu_project/tool_catalog.rs`
- Create: `src-tauri/src/feishu_project/mcp_sync.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `feat: add feishu project mcp stdio runtime` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp`; `git diff --check` | The pivot must first prove we can connect, initialize, and list tools before we rewrite the inbox sync path. |
| Task 2 | `feat: source bug inbox from feishu mcp` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync`; `cargo test --manifest-path src-tauri/Cargo.toml`; `git diff --check` | Tool discovery should drive the adapter; do not hardcode final tool IDs before seeing the real catalog. |
| Task 3 | `feat: switch bug inbox config to mcp connection` | `bun test src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts`; `bun run build`; `git diff --check` | The UI must stop asking for inaccessible tokens and instead expose MCP connection status and controls. |
| Task 4 | `refactor: retire legacy feishu token sync path` | `cargo test --manifest-path src-tauri/Cargo.toml`; `bun run build`; `git diff --check` | The old path should be clearly demoted or removed so we do not maintain two conflicting primary architectures. |

## Task 1: Add Feishu Project MCP stdio runtime and tool discovery

**Files:**
- Modify: `src-tauri/src/feishu_project/types.rs`
- Modify: `src-tauri/src/feishu_project/config.rs`
- Modify: `src-tauri/src/feishu_project/runtime.rs`
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

- [ ] **Step 2: Write failing tests for stdio client handshake**

Add tests that prove:

- initialize request formatting
- parsing `tools/list` results
- runtime state reflects connection failure

- [ ] **Step 3: Implement minimal MCP stdio client**

The client only needs this subset first:

- spawn configured command
- send `initialize`
- send `tools/list`
- collect the tool catalog

Do not implement all downstream inbox sync yet.

- [ ] **Step 4: Add a diagnostics surface in runtime state**

Expose:

- connected / disconnected
- last error
- discovered tool count
- maybe tool names preview

- [ ] **Step 5: Verify**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp
git diff --check
```

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/feishu_project/types.rs src-tauri/src/feishu_project/config.rs src-tauri/src/feishu_project/runtime.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project/mcp_client.rs src-tauri/src/feishu_project/mcp_stdio.rs src-tauri/src/feishu_project/tool_catalog.rs
git commit -m "feat: add feishu project mcp stdio runtime"
```

## Task 2: Source Bug Inbox from Feishu MCP

**Files:**
- Modify: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Create: `src-tauri/src/feishu_project/mcp_sync.rs`

- [ ] **Step 1: Discover the actual required tools**

Use the live tool catalog from Task 1 and record which tool(s) provide:

- workspace metadata
- work-item listing
- work-item detail/meta required for snapshots

- [ ] **Step 2: Write failing tests for adapter mapping**

Tests should prove:

- MCP result rows map into `FeishuProjectInboxItem`
- repeated sync still preserves `ignored` and `linkedTaskId`
- missing required tools becomes a surfaced runtime error

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

## Task 3: Switch Bug Inbox config UI to MCP connection

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

## Task 4: Retire the legacy Feishu token sync path

**Files:**
- Modify: `docs/superpowers/specs/2026-04-09-feishu-project-bug-inbox-design.md`
- Modify: `docs/superpowers/plans/2026-04-09-feishu-project-bug-inbox.md`
- Modify: any obsolete runtime files as needed

- [ ] **Step 1: Remove or clearly demote the old REST/webhook path**

Do one of:

- remove the old path entirely if the MCP path is proven stable, or
- clearly mark it as legacy/fallback and keep it out of the main UI

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
