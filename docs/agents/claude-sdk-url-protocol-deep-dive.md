# Claude Code `--sdk-url` Protocol Deep Dive

Extracted from CLI bundle `@anthropic-ai/claude-code/cli.js`.

---

## 1. All Transport-Level Message Types

Messages are newline-delimited JSON. Each has a top-level `type` field.

### Inbound (host -> CLI, processed by `EK8.processLine`)

| type | description |
|------|-------------|
| `user` | User message. Must have `message.role === "user"`. Fields: `type, session_id, message, parent_tool_use_id` |
| `assistant` | Replay of assistant message (only accepted when `replayUserMessages` enabled) |
| `system` | System message injection |
| `control_response` | Response to a pending `control_request`. Fields: `type, response: {subtype, request_id, ...}` |
| `control_request` | Can arrive inbound when replaying (rare) |
| `keep_alive` | Silently consumed, no action |
| `update_environment_variables` | Sets `process.env` keys. Fields: `type, variables: Record<string, string>` |

Unknown types are logged as warnings and dropped.

### Outbound (CLI -> host, emitted via `write()` / `yield`)

| type | description |
|------|-------------|
| `assistant` | Model response turn. Fields: `type, uuid, session_id, message, parent_tool_use_id, error?` |
| `result` | Conversation turn completion. See subtypes below |
| `system` | System notifications (many subtypes). Fields: `type, subtype, content, ...` |
| `user` | Echo of user messages in some contexts |
| `control_request` | Permission / config request to host. Fields: `type, request_id, request: {subtype, ...}` |
| `control_cancel_request` | Cancel a pending control request. Fields: `type, request_id` |
| `control_response` | Response forwarded back (in replay scenarios) |
| `keep_alive` | Sent periodically to maintain connection |
| `stream_event` | Raw Anthropic API streaming event. Fields: `type, event, ttftMs?` |
| `prompt_suggestion` | Next-prompt suggestion. Fields: `type, suggestion, uuid, session_id` |
| `rate_limit_event` | Rate limit info. Fields: `type, rate_limit_info, uuid, session_id` |
| `auth_status` | OAuth status change. Fields: `type, isAuthenticating, output, error, uuid, session_id` |

### `result` subtypes

| subtype | meaning |
|---------|---------|
| `success` | Normal completion |
| `error_during_execution` | Runtime error |
| `error_max_turns` | Turn limit reached |
| `error_max_budget_usd` | Budget exhausted |
| `error_max_structured_output_retries` | Structured output validation failures exceeded |

Common `result` fields:
```
{
  type: "result",
  subtype: string,
  is_error: boolean,
  duration_ms: number,
  duration_api_ms: number,
  num_turns: number,
  result: string,
  stop_reason: string | null,
  session_id: string,
  total_cost_usd: number,
  usage: { input_tokens, cache_creation_input_tokens, cache_read_input_tokens, output_tokens, server_tool_use: { web_search_requests, web_fetch_requests } },
  modelUsage: Record<string, unknown>,
  permission_denials: array,
  uuid: string,
  errors?: string[],
  structured_output?: unknown,
  deferred_tool_use?: { id, name, input },
  fast_mode_state?: unknown
}
```

### `system` message subtypes

| subtype | description |
|---------|-------------|
| `init` | Session initialized |
| `status` | Status update |
| `informational` | Informational notice |
| `bridge_status` | Remote control active. Extra fields: `url, upgradeNudge` |
| `bridge_state` | Bridge state change |
| `api_error` | API error |
| `api_retry` | API retry |
| `compact_boundary` | Context compaction boundary |
| `elicitation_complete` | Elicitation finished |
| `file_snapshot` | File state snapshot |
| `hook_started` | Hook execution started |
| `hook_progress` | Hook execution progress |
| `hook_response` | Hook execution response |
| `local_command` | Local command execution |
| `memory_saved` | Memory file saved |
| `permission_retry` | Permission request retry |
| `session_state_changed` | Session state transition |
| `stop_hook_summary` | Stop hook summary |
| `turn_duration` | Turn timing info |
| `agents_killed` | Agent workers terminated |
| `task_started` | Background task started |
| `task_progress` | Background task progress |
| `task_notification` | Background task notification |
| `scheduled_task_fire` | Scheduled task fired |

---

## 2. All `control_request` Subtypes

The union schema `DnY` defines 21 subtypes. Each is sent via `sendRequest()` and expects a typed response.

### `interrupt`
```
request:  { subtype: "interrupt" }
response: {} (empty success)
```
Interrupts the currently running conversation turn.

### `can_use_tool`
```
request: {
  subtype: "can_use_tool",
  tool_name: string,
  input: Record<string, unknown>,
  permission_suggestions?: PermissionUpdate[],
  blocked_path?: string,
  decision_reason?: string,
  title?: string,
  display_name?: string,
  tool_use_id: string,
  agent_id?: string,
  description?: string
}
response: TnY (allow) | knY (deny)   // see section 3
```

### `initialize`
```
request: {
  subtype: "initialize",
  hooks?: Record<HookEventName, HookConfig[]>,
  sdkMcpServers?: string[],
  jsonSchema?: Record<string, unknown>,
  systemPrompt?: string,
  appendSystemPrompt?: string,
  agents?: Record<string, AgentConfig>,
  promptSuggestions?: boolean,
  agentProgressSummaries?: boolean
}
response: {
  commands: Command[],
  agents: Agent[],
  output_style: string,
  available_output_styles: string[],
  models: Model[],
  account: Account,
  pid?: number,
  fast_mode_state?: unknown
}
```

HookEventName enum:
```
"PreToolUse" | "PostToolUse" | "PostToolUseFailure" | "Notification" |
"UserPromptSubmit" | "SessionStart" | "SessionEnd" | "Stop" | "StopFailure" |
"SubagentStart" | "SubagentStop" | "PreCompact" | "PostCompact" |
"PermissionRequest" | "PermissionDenied" | "Setup" | "TeammateIdle" |
"TaskCreated" | "TaskCompleted" | "Elicitation" | "ElicitationResult" |
"ConfigChange" | "WorktreeCreate" | "WorktreeRemove" | "InstructionsLoaded" | "CwdChange"
```

HookConfig:
```
{ matcher?: string, hookCallbackIds: string[], timeout?: number }
```

AgentConfig:
```
{
  description: string,
  tools?: string[],
  disallowedTools?: string[],
  prompt: string,
  model?: string
}
```

### `set_permission_mode`
```
request: { subtype: "set_permission_mode", mode: PermissionMode, ultraplan?: boolean }
response: {} (empty success)
```
PermissionMode enum: `"acceptEdits" | "bypassPermissions" | "default" | "dontAsk" | "plan"`

### `set_model`
```
request: { subtype: "set_model", model?: string }
response: {} (empty success)
```

### `set_max_thinking_tokens`
```
request: { subtype: "set_max_thinking_tokens", max_thinking_tokens: number | null }
response: {} (empty success)
```

### `mcp_status`
```
request: { subtype: "mcp_status" }
response: { mcpServers: McpServerStatus[] }
```

### `get_context_usage`
```
request: { subtype: "get_context_usage" }
response: {
  categories: [{ name, tokens, color, isDeferred? }],
  totalTokens: number,
  maxTokens: number,
  rawMaxTokens: number,
  percentage: number,
  gridRows: [{ color, isFilled, categoryName, tokens, percentage, squareFullness }][],
  model: string,
  memoryFiles: [{ path, type }]
}
```

### `hook_callback`
```
request: { subtype: "hook_callback", callback_id: string, input: unknown, tool_use_id?: string }
response: { continue?: boolean, suppressOutput?: boolean, stopReason?: string, ... }
       | { async: true, asyncTimeout?: number }
```

### `mcp_message`
```
request: { subtype: "mcp_message", server_name: string, message: JsonRpcMessage }
response: { mcp_response: any }
```

### `mcp_set_servers`
```
request: { subtype: "mcp_set_servers", servers: Record<string, McpServerConfig> }
response: { added: string[], removed: string[], errors: Record<string, string> }
```

McpServerConfig variants:
```
{ type?: "stdio", command: string, args?: string[], env?: Record<string, string> }
{ type: "sse", url: string, headers?: Record<string, string> }
{ type: "http", url: string, headers?: Record<string, string> }
{ type: "sdk", name: string }
```

### `mcp_reconnect`
```
request: { subtype: "mcp_reconnect", serverName: string }
response: {} (empty success)
```

### `mcp_toggle`
```
request: { subtype: "mcp_toggle", serverName: string, enabled: boolean }
response: {} (empty success)
```

### `rewind_files`
```
request: { subtype: "rewind_files", user_message_id: string, dry_run?: boolean }
response: { canRewind: boolean, error?: string, filesChanged?: string[], insertions?: number, deletions?: number }
```

### `cancel_async_message`
```
request: { subtype: "cancel_async_message", message_uuid: string }
response: { cancelled: boolean }
```

### `seed_read_state`
```
request: { subtype: "seed_read_state", path: string, mtime: number }
response: {} (empty success)
```
Seeds readFileState cache so Edit validation works after context compaction.

### `reload_plugins`
```
request: { subtype: "reload_plugins" }
response: { commands: Command[], agents: Agent[], plugins: [{ name, path, source? }], mcpServers: McpServerStatus[], error_count: number }
```

### `stop_task`
```
request: { subtype: "stop_task", task_id: string }
response: {} (empty success)
```

### `apply_flag_settings`
```
request: { subtype: "apply_flag_settings", settings: Record<string, unknown> }
response: {} (empty success)
```

### `elicitation`
```
request: {
  subtype: "elicitation",
  mcp_server_name: string,
  message: string,
  mode?: "form" | "url",
  url?: string,
  elicitation_id?: string,
  requested_schema?: Record<string, unknown>
}
response: { action: "accept" | "decline" | "cancel", content?: Record<string, unknown> }
```

### `get_settings`
```
request: { subtype: "get_settings" }
response: {
  effective: Record<string, unknown>,
  sources: [{ source: "userSettings"|"projectSettings"|"localSettings"|"flagSettings"|"policySettings", settings: Record<string,unknown> }]
}
```

---

## 3. `control_response` Format for `can_use_tool`

### Envelope

```json
{
  "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": "<matching request_id>",
    "response": { /* TnY or knY */ }
  }
}
```

Or error:
```json
{
  "type": "control_response",
  "response": {
    "subtype": "error",
    "request_id": "<matching request_id>",
    "error": "<error message>",
    "pending_permission_requests?": [ /* control_request objects */ ]
  }
}
```

### `TnY` (allow schema)

```
{
  behavior: "allow",                              // literal
  updatedInput: Record<string, unknown>,           // required (can echo original)
  updatedPermissions?: PermissionUpdate[],         // optional, parsed with catch
  toolUseID?: string,                              // optional
  decisionClassification?: "user_temporary" | "user_permanent" | "user_reject"  // optional
}
```

### `knY` (deny schema)

```
{
  behavior: "deny",                                // literal
  message: string,                                 // required
  interrupt?: boolean,                             // optional
  toolUseID?: string,                              // optional
  decisionClassification?: "user_temporary" | "user_permanent" | "user_reject"  // optional
}
```

The combined schema is `yK8 = z.union([TnY, knY])`.

### `PermissionUpdate` (CE6) discriminated union by `type`

```
{ type: "addRules",       rules: [{ toolName, ruleContent? }], behavior: "allow"|"deny"|"ask", destination: Destination }
{ type: "replaceRules",   rules: [{ toolName, ruleContent? }], behavior: "allow"|"deny"|"ask", destination: Destination }
{ type: "removeRules",    rules: [{ toolName, ruleContent? }], behavior: "allow"|"deny"|"ask", destination: Destination }
{ type: "setMode",        mode: PermissionMode, destination: Destination }
{ type: "addDirectories", directories: string[], destination: Destination }
{ type: "removeDirectories", directories: string[], destination: Destination }
```

Destination enum: `"userSettings" | "projectSettings" | "localSettings" | "session" | "cliArg"`

---

## 4. `h48` HybridTransport Details

Class hierarchy: `L48` (WebSocket base) -> `h48` (adds HTTP POST batching).

### Constants

| name | value | meaning |
|------|-------|---------|
| `TmY` | `100` ms | Stream event buffer flush interval |
| `kmY` | `15000` ms (15s) | POST request timeout (axios) |
| `VmY` | `3000` ms (3s) | Close flush timeout (race with `flush()`) |
| `WmY` | `1000` | WebSocket message ring buffer capacity |
| `DmY` | `1000` ms | Reconnect base delay |
| `SdK` | `30000` ms (30s) | Reconnect max delay |
| `fmY` | `600000` ms (10min) | Total reconnection time budget |
| `ZmY` | `10000` ms (10s) | Ping interval |
| `GmY` | `300000` ms (5min) | Keep-alive interval |
| `RdK` | `60000` ms (2 * SdK) | Sleep detection threshold |
| `vmY` | `Set([1002, 4001, 4003])` | Permanent WS close codes (no reconnect) |
| `hnY` | `1000` | Max resolved tool use ID cache size |

### POST URL derivation (`NmY`)

```js
function NmY(url) {
  let protocol = url.protocol === "wss:" ? "https:" : "http:";
  let pathname = url.pathname.replace("/ws/", "/session/");
  if (!pathname.endsWith("/events"))
    pathname = pathname.endsWith("/") ? pathname + "events" : pathname + "/events";
  return `${protocol}//${url.host}${pathname}${url.search}`;
}
```

Example: `wss://host/ws/abc` -> `https://host/session/abc/events`

### `postOnce` method

```js
async postOnce(events) {
  let token = getSessionToken();  // ED()
  if (!token) return;             // logged + dropped

  let headers = {
    "Authorization": `Bearer ${token}`,
    "Content-Type": "application/json"
  };

  let response = await axios.post(postUrl, { events }, {
    headers,
    validateStatus: () => true,
    timeout: 15000  // kmY
  });

  if (200 <= status < 300) return;              // success
  if (400 <= status < 500 && status !== 429) {  // permanent error, drop
    return;
  }
  throw Error(`POST failed with ${status}`);    // retryable -> cJ6 retries
}
```

### `cJ6` Batch Uploader (used by `h48`)

```
maxBatchSize: 500
maxQueueSize: 100000
baseDelayMs: 500
maxDelayMs: 8000
jitterMs: 1000
```

Retry with exponential backoff. Drops batch after `maxConsecutiveFailures`.

### `write` method flow

```js
async write(msg) {
  if (msg.type === "stream_event") {
    this.streamEventBuffer.push(msg);
    if (!this.streamEventTimer)
      this.streamEventTimer = setTimeout(() => this.flushStreamEvents(), 100); // TmY
    return;
  }
  // Non-stream events: immediate flush via POST
  await this.uploader.enqueue([...this.takeStreamEvents(), msg]);
  this.uploader.flush();
}
```

Stream events are batched for 100ms before POST. All other events trigger immediate flush.

### `close` method

```js
close() {
  // Clear stream buffer
  clearTimeout(this.streamEventTimer);
  this.streamEventBuffer = [];
  // Race uploader flush against VmY (3s) timeout
  Promise.race([uploader.flush(), timeout(3000)]).finally(() => uploader.close());
  super.close();  // L48 WebSocket close
}
```

### L48 WebSocket Reconnection

- Exponential backoff: `min(1000 * 2^(attempt-1), 30000)` with +/-25% jitter
- Total budget: 10 minutes (`fmY = 600000`)
- Sleep detection: if gap between ticks > 60s, reset reconnection budget
- Permanent close codes (no reconnect): `1002`, `4001`, `4003`
  - Special case: `4003` will reconnect if headers were refreshed
- On reconnect: replays buffered messages (ring buffer of 1000)
- `X-Last-Request-Id` header used to deduplicate on reconnect
- Ping/pong every 10s; if pong not received, force reconnect
- Keep-alive frame sent every 5min (disabled in `CLAUDE_CODE_REMOTE` mode)

### Transport selection (`Qq5`)

```js
function getTransportForUrl(url, headers, sessionId, refreshHeaders) {
  if (CLAUDE_CODE_USE_CCR_V2) {
    // SSE transport -> /worker/events/stream
    return new SSETransport(url, headers, sessionId, refreshHeaders);
  }
  if (url.protocol === "ws:" || url.protocol === "wss:") {
    if (CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2)
      return new h48(url, headers, sessionId, refreshHeaders);  // HybridTransport
    return new L48(url, headers, sessionId, refreshHeaders);     // plain WebSocket
  }
  throw Error(`Unsupported protocol: ${url.protocol}`);
}
```

---

## 5. Bridge Session Spawn Args

From `Vz7` factory (`[bridge:session]` spawning code):

### CLI Arguments

```js
let args = [
  ...scriptArgs,                              // base CLI script args
  "--print",                                  // print mode (non-interactive)
  "--sdk-url", sessionConfig.sdkUrl,          // WebSocket URL back to host
  "--session-id", sessionConfig.sessionId,
  "--input-format", "stream-json",
  "--output-format", "stream-json",
  "--replay-user-messages",
  ...(verbose ? ["--verbose"] : []),
  ...(debugFile ? ["--debug-file", debugFile] : []),
  ...(permissionMode ? ["--permission-mode", permissionMode] : [])
];
```

### Environment Variables

```js
let env = {
  ...parentEnv,
  CLAUDE_CODE_OAUTH_TOKEN: undefined,                 // explicitly cleared
  CLAUDE_CODE_ENVIRONMENT_KIND: "bridge",
  ...(sandbox && { CLAUDE_CODE_FORCE_SANDBOX: "1" }),
  CLAUDE_CODE_SESSION_ACCESS_TOKEN: sessionConfig.accessToken,
  CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2: "1",       // forces HybridTransport
  ...(useCcrV2 && {
    CLAUDE_CODE_USE_CCR_V2: "1",
    CLAUDE_CODE_WORKER_EPOCH: String(sessionConfig.workerEpoch)
  })
};
```

### Child Process Config

```js
spawn(execPath, args, {
  cwd: workingDirectory,
  stdio: ["pipe", "pipe", "pipe"],
  env: env,
  windowsHide: true
});
```

### Runtime Communication

- **stdin**: Host writes newline-delimited JSON (user messages, control_responses, update_environment_variables)
- **stdout**: Worker emits newline-delimited JSON (assistant, result, control_request, stream_event, etc.)
- **stderr**: Logged and buffered (last 10 lines kept in `lastStderr`)
- **Token refresh**: `update_environment_variables` message with `CLAUDE_CODE_SESSION_ACCESS_TOKEN`

### Activity Buffer Limits

```
rxY = 10   // max activity entries kept per session
oxY = 10   // max stderr lines kept per session
```

### Worker Lifecycle States

Returned by `done` promise: `"completed"` (exit 0), `"failed"` (non-zero exit), `"interrupted"` (SIGTERM/SIGINT).

### Gn8 (RemoteIO) additional details

When `CLAUDE_CODE_ENVIRONMENT_KIND === "bridge"`:
- Control requests are echoed to stdout
- Keep-alive sent at `session_keepalive_interval_v2_ms` interval
- Additional headers: `Authorization: Bearer <sessionToken>`, `x-environment-runner-version` (if set)

---

## Summary Table: Message Flow

```
Host (SDK consumer)                CLI Worker (--sdk-url)
        |                                |
        |-- user ----------------------->|  user prompt
        |-- control_response ----------->|  permission verdict / config ack
        |-- update_environment_variables->|  env var update (e.g. token refresh)
        |-- keep_alive ----------------->|  heartbeat
        |                                |
        |<---------- assistant ----------|  model response
        |<---------- result --------------|  turn completion
        |<---------- system --------------|  notifications (many subtypes)
        |<---------- control_request -----|  permission/config request
        |<---------- control_cancel_req --|  cancel pending request
        |<---------- stream_event --------|  raw API stream chunk
        |<---------- prompt_suggestion ---|  next prompt suggestion
        |<---------- rate_limit_event ----|  rate limit info
        |<---------- auth_status ---------|  auth state change
        |<---------- keep_alive ----------|  heartbeat echo
```
