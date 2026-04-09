# Feishu Project MCP Server — Investigation Notes

> Task 0 evidence artifact for the MCP pivot plan.
> Date: 2026-04-09

## Package Identity

- npm: `@lark-project/mcp@0.0.1`
- Binary name: `meego-mcp-cli`
- Entry: `dist/index.js` (ESM, `#!/usr/bin/env node`)
- Published: 3 weeks ago by ByteDance engineers
- Unpacked size: 23.5 kB, 107 transitive deps, ~27 MB installed
- Dependencies: `@modelcontextprotocol/sdk`, `axios`, `commander`, `dotenv`, `env-paths`
- No `engines` field — no strict Node version constraint

## Architecture: Stdio-to-HTTP Proxy

**Critical finding:** The npm package is NOT a standalone MCP server. It is a **stdio-to-HTTP proxy/bridge**:

```text
MCP Client (Dimweave)
  ↔ stdio (JSON-RPC 2.0)
  ↔ @lark-project/mcp (local Node.js process)
    ↔ StreamableHTTPClientTransport (@modelcontextprotocol/sdk)
    ↔ Feishu Project remote MCP server (https://project.feishu.cn/mcp_server/v1)
```

The local Node process:
1. Starts an MCP stdio server (via `@modelcontextprotocol/sdk Server`)
2. Connects to the remote Feishu MCP endpoint via `StreamableHTTPClientTransport`
3. Proxies `tools/list` and `tools/call` requests between stdio and HTTP

The actual MCP tools/capabilities are served by Feishu's cloud infrastructure, not the local package.

## Launch Command

```bash
# Via npx (official docs example)
npx -y @lark-project/mcp --domain https://project.feishu.cn --user_token <TOKEN>

# Via local install (verified working)
node <install_path>/node_modules/@lark-project/mcp/dist/index.js \
  --domain https://project.feishu.cn \
  --user_token <TOKEN>

# Via env var
MCP_USER_TOKEN=<TOKEN> node <install_path>/node_modules/@lark-project/mcp/dist/index.js
```

CLI options:
- `--domain <domain>` — defaults to `https://project.feishu.cn`
- `--user_token <user_token>` — **required**, no default
- `--lang <lang>` — `zh` or `en`
- `--debug` — enables verbose logging

## Authentication

### Token Requirement

`MCP_USER_TOKEN` is **mandatory**. The server refuses to start without it:

```js
if (!options.userToken) {
    throw new Error('userToken must be provided');
}
```

The token is sent to the remote server as HTTP header `X-Mcp-Token`.

### No Interactive Login

The npm package has **no OAuth flow, no browser redirect, no interactive login**. The token must be pre-obtained and provided via CLI argument or environment variable.

### How to Obtain `MCP_USER_TOKEN`

**Not yet verified.** Likely candidates:
1. Feishu personal access token from project settings
2. Feishu `user_access_token` from OAuth flow
3. Token from Feishu Project MCP settings page (referenced in help center docs)

**Blocker:** We need the user to provide a real token or document how to obtain one.

## App-Managed Feasibility

### Verdict: **Conditionally Feasible**

**Can Dimweave manage the npm package install?** YES.

Verified approach:
1. `npm install @lark-project/mcp` into a Dimweave-managed directory (e.g., `$APP_DATA/feishu-mcp/`)
2. Launch via `node $APP_DATA/feishu-mcp/node_modules/@lark-project/mcp/dist/index.js --user_token <TOKEN>`
3. No global install required
4. No native dependencies — pure JS, cross-platform

**Remaining dependency: Node.js runtime.** The package requires `node` to execute. Options:
- **Option A:** Require Node.js on PATH (most users in this context already have it)
- **Option B:** Bundle a Node.js runtime (adds ~40-80 MB, complex)
- **Option C:** Bypass the npm package entirely — Dimweave's Rust MCP client connects directly to `https://project.feishu.cn/mcp_server/v1` via StreamableHTTP, eliminating the Node.js proxy entirely

### Option C Analysis (Direct HTTP MCP Client)

Since the npm package is just a proxy, Dimweave could skip it entirely:

```text
Dimweave Rust MCP Client
  ↔ StreamableHTTP (POST with SSE/JSON responses)
  ↔ https://project.feishu.cn/mcp_server/v1
```

Required HTTP headers:
- `X-Meego-MCP-Connection-Type: stdio` (or appropriate value)
- `X-Mcp-Token: <user_token>`
- `Content-Type: application/json`

This would:
- Eliminate Node.js dependency entirely
- Eliminate npm package management
- Use Dimweave's existing Rust HTTP client (reqwest/hyper)
- Match the existing Tauri desktop architecture better
- Reduce complexity from "manage Node subprocess + stdio" to "HTTP POST + response parsing"

The MCP StreamableHTTP protocol is standard: JSON-RPC 2.0 over HTTP POST with optional SSE for streaming.

## Remote MCP Endpoint

- URL: `https://project.feishu.cn/mcp_server/v1`
- Protocol: MCP StreamableHTTP (JSON-RPC 2.0 over HTTP POST)
- Auth: `X-Mcp-Token` header
- Without valid token: returns `{"error":"unauthorized"}`

## Tool Catalog

**BLOCKED — requires valid `MCP_USER_TOKEN`.**

From help center documentation, expected capabilities include:
- Work item listing and detail retrieval
- Space/project metadata
- Process/flow information
- View queries
- Comment read/write

Real `tools/list` response not yet captured.

## Logging

The package writes logs to platform-specific paths via `env-paths`:
- macOS: `~/Library/Logs/lark-project-mcp-nodejs/`
- Log format: `larkproject-mcp-YYYY-MM-DD.log`
- Auto-cleans logs older than 7 days

## Engineering Recommendation

### Primary: Direct HTTP MCP Client (Option C)

Skip the npm stdio proxy. Build a Rust HTTP MCP client that talks directly to `https://project.feishu.cn/mcp_server/v1`. This:

1. Eliminates Node.js runtime dependency
2. Eliminates npm package management complexity
3. Fits the existing Tauri/Rust architecture
4. Reduces moving parts (no subprocess lifecycle)
5. Is simpler than the stdio proxy approach from the original plan

The trade-off is implementing StreamableHTTP MCP in Rust instead of using the SDK's stdio transport, but the protocol is straightforward HTTP POST + JSON-RPC.

### Fallback: App-Managed Node Proxy

If direct HTTP proves problematic (e.g., streaming SSE complications), fall back to:
1. Dimweave manages `npm install @lark-project/mcp` into app data directory
2. Spawns `node .../dist/index.js --user_token <TOKEN>` as subprocess
3. Communicates via stdio JSON-RPC

This still requires Node.js on the system but avoids global package installation.

### Token UX

Regardless of transport choice, the user must provide `MCP_USER_TOKEN`. The config UI should:
1. Accept the token as a one-time setup field
2. Store it securely (Tauri secure storage or OS keychain)
3. Surface clear "unauthorized" errors if the token expires
