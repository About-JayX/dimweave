# Feishu Project MCP Pivot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the current token/webhook-first Feishu Project data-access layer with an MCP-first integration while preserving the existing Bug Inbox UI and linked task-handling workflow.

**Architecture:** Add a direct HTTP Feishu Project MCP client runtime in Tauri that talks to `https://project.feishu.cn/mcp_server/v1`, authenticates with `MCP_USER_TOKEN`, discovers the real tool catalog, and maps the discovered read capabilities into the existing Bug Inbox domain store. Retain the current Bug Inbox UI shell and task-link workflow, but replace the config model and remote sync path.

**Tech Stack:** React 19, TypeScript, Zustand, Tauri 2, Rust, tokio, subprocess stdio, serde_json, Bun, Cargo

---

## Baseline Notes

- The current token/webhook implementation is already committed and working, but it is not a good fit for this user because they cannot obtain project-space tokens.
- The Bug Inbox UI and task-link path should be preserved as much as possible.
- The new unknowns are:
  - how `domain` and `MCP_USER_TOKEN` are obtained from the Feishu Project MCP settings UI in practice
  - the exact `tools/list` catalog and JSON schemas
- The implementation must therefore include an explicit discovery/diagnostics stage instead of hardcoding all tool names up front.
- New hard requirement from the user: **MCP must be project-bundled / app-managed, not globally installed by the user.**

## Concrete Feishu evidence already captured

From the Feishu Project help-center page “在 AI 工具中连接 MCP”:

- Claude Code `stdio` example uses:

```json
{
  "mcpServers": {
    "FeishuProjectMcp": {
      "command": "npx",
      "args": ["-y", "@lark-project/mcp", "--domain", "{domain}"],
      "env": { "MCP_USER_TOKEN": "" }
    }
  }
}
```

- Codex CLI supports project-scoped MCP config via:

```toml
.codex/config.toml
```

Task 0 investigation also proved the npm package is only a stdio-to-HTTP proxy to:

```text
https://project.feishu.cn/mcp_server/v1
```

Implication: the most realistic non-global-install path is now **direct HTTP MCP**, not a bundled native binary and not a managed npm proxy.

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
- Create: `src-tauri/src/feishu_project/mcp_http.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 0 | `docs: capture feishu project mcp catalog` | `git diff --check`; saved artifact with launch notes and placeholder catalog | Task 0 proved the stdio package is only a proxy and that direct HTTP is the better app-managed path. |
| Task 1 | `feat: add feishu project mcp http runtime` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp`; `git diff --check` | The pivot must first prove we can connect, initialize, and list tools over direct HTTP before we rewrite the inbox sync path. |
| Task 2 | `feat: switch bug inbox config to mcp connection` | `bun test src/components/BugInboxPanel/index.test.tsx src/stores/feishu-project-store.test.ts`; `bun run build`; `git diff --check` | The UI must stop asking for inaccessible tokens and instead expose MCP connection status and controls. |
| Task 3 | `feat: source bug inbox from feishu mcp` | `cargo test --manifest-path src-tauri/Cargo.toml feishu_project::mcp_sync`; `cargo test --manifest-path src-tauri/Cargo.toml`; `git diff --check` | Tool discovery should drive the adapter; do not hardcode final tool IDs before seeing the real catalog. |
| Task 4 | `refactor: retire legacy feishu token sync path` | `cargo test --manifest-path src-tauri/Cargo.toml`; `bun run build`; `git diff --check` | The old path should be clearly demoted or removed so we do not maintain two conflicting primary architectures. |

## Task 0: Capture the real Feishu Project MCP transport shape and tool-catalog blocker

**Files:**
- Create: `docs/agents/feishu-project-mcp-tool-catalog.json`
- Create: `docs/agents/feishu-project-mcp-notes.md`

- [ ] **Step 1: Connect manually to the real Feishu Project MCP server**

Use the real Feishu Project MCP stdio launch command and capture:

- how `@lark-project/mcp` is launched in practice
- whether it requires an interactive login step
- where `domain` is obtained
- where `MCP_USER_TOKEN` is obtained
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

The notes must explicitly answer:

- Is the npm package only a proxy?
- Can Dimweave bypass it and speak direct HTTP MCP?
- Is `MCP_USER_TOKEN` still mandatory?

- [ ] **Step 3: Commit**

```bash
git add docs/agents/feishu-project-mcp-tool-catalog.json docs/agents/feishu-project-mcp-notes.md
git commit -m "docs: capture feishu project mcp catalog"
```

## Task 1: Add Feishu Project MCP HTTP runtime and tool discovery

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/feishu_project/types.rs`
- Modify: `src-tauri/src/feishu_project/config.rs`
- Replace: `src-tauri/src/feishu_project/runtime.rs`
- Modify: `src-tauri/src/daemon/feishu_project_lifecycle.rs`
- Modify: `src-tauri/src/feishu_project/mod.rs`
- Create: `src-tauri/src/feishu_project/mcp_http.rs`
- Create: `src-tauri/src/feishu_project/mcp_client.rs`
- Create: `src-tauri/src/feishu_project/tool_catalog.rs`

- [ ] **Step 1: Replace the config model with direct MCP endpoint config**

Move from REST-token config to MCP endpoint config:

```rust
pub struct FeishuProjectConfig {
    pub enabled: bool,
    pub domain: String,
    pub mcp_user_token: String,
    pub workspace_hint: Option<String>,
    pub refresh_interval_minutes: u64,
    pub last_error: Option<String>,
}
```

- [ ] **Step 2: Write failing tests for HTTP MCP transport and client**

Tests should prove:

- initialize request formatting
- parsing HTTP JSON-RPC responses
- unauthorized response handling
- missing required tools handling

- [ ] **Step 3: Implement HTTP MCP transport**

The transport needs:

- JSON-RPC 2.0 request building
- HTTP POST to `{domain}/mcp_server/v1`
- required headers:
  - `X-Mcp-Token`
  - `X-Meego-MCP-Connection-Type`
- timeout + response parsing
- initialize + tools/list + tools/call support

- [ ] **Step 4: Add diagnostics surface in runtime state**

Expose:

- connected / disconnected
- unauthorized / invalid token
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
git add src-tauri/src/main.rs src-tauri/src/daemon/cmd.rs src-tauri/src/daemon/mod.rs src-tauri/src/feishu_project/types.rs src-tauri/src/feishu_project/config.rs src-tauri/src/feishu_project/runtime.rs src-tauri/src/daemon/feishu_project_lifecycle.rs src-tauri/src/feishu_project/mod.rs src-tauri/src/feishu_project/mcp_http.rs src-tauri/src/feishu_project/mcp_client.rs src-tauri/src/feishu_project/tool_catalog.rs
git commit -m "feat: add feishu project mcp http runtime"
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
- domain
- `MCP_USER_TOKEN`
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

Also explicitly remove the Feishu webhook route from `control/server.rs` if it is no longer part of the primary product path, and remove the legacy token/webhook fields from the primary config model.

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
