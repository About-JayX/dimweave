# Codex MCP 集成

> Historical note: this dated spec describes an intermediate MCP design, not the live architecture. Current source of truth is `CLAUDE.md`, `.claude/rules/**`, `src-tauri/src/daemon/**`, and `bridge/**`.

## Summary

让 Codex 和 Claude 使用完全相同的 MCP tools（`reply`/`check_messages`/`get_status`）进行结构化通信。复用现有 `bridge.ts`，通过环境变量区分 agent 身份。Codex 通过 `CODEX_HOME/mcp.json` 注入，Claude 通过 `--mcp-config` 内联注入。

## Motivation

当前 Codex 只能输出纯文本，无法结构化指定 `to`/`type` 等路由字段。`developerInstructions` 是软引导，不可靠。MCP tools 是唯一能让 agent 主动、结构化通信的方式。

## Design

### MCP Tools（两边完全一致）

```
reply(to, text, type?, replyTo?, priority?)  — 发消息给目标角色
check_messages()                              — 拉取新消息
get_status()                                  — 查看在线角色
```

### bridge.ts 泛化

当前 bridge.ts 硬编码为 Claude。通过 `AGENTBRIDGE_AGENT` 环境变量泛化：

```typescript
const AGENT_ID = process.env.AGENTBRIDGE_AGENT ?? "claude";
// 连接 daemon 时声明身份
daemonClient.attachAgent(AGENT_ID);
// reply tool 的 from 用 daemon 返回的角色名
adapter.setAgentRole(agentRole);
```

同一个 bridge.ts 文件，Claude 和 Codex 各自 spawn 一个独立进程。

### 注入方式

**Claude（不变）**：
```bash
claude --strict-mcp-config --mcp-config '<inline json>'
```

**Codex（新增）**：
```
CODEX_HOME/mcp.json:
{
  "mcpServers": {
    "agentbridge": {
      "command": "bun",
      "args": ["run", "<abs_bridge_path>"],
      "env": {
        "AGENTBRIDGE_CONTROL_PORT": "4502",
        "AGENTBRIDGE_AGENT": "codex"
      }
    }
  }
}
```

`session-manager.ts` 创建临时 `CODEX_HOME` 时写入此文件。

### 控制协议

```typescript
// Before
| { type: "claude_connect" }
| { type: "claude_disconnect" }

// After
| { type: "agent_connect"; agentId: string }
| { type: "agent_disconnect"; agentId: string }
```

### daemon-state

```typescript
// Before
attachedClaude: ServerWebSocket<ControlSocketData> | null = null;

// After
attachedAgents = new Map<string, ServerWebSocket<ControlSocketData>>();
```

### 路由变更

`resolveTarget` 查 `attachedAgents` 而非 `attachedClaude`：

```typescript
function resolveTarget(to: string, deps): RouteTarget[] {
  const targets: RouteTarget[] = [];
  if (state.claudeRole === to) {
    targets.push({
      agent: "claude",
      online: state.attachedAgents.has("claude"),
    });
  }
  if (state.codexRole === to) {
    targets.push({
      agent: "codex",
      online: state.attachedAgents.has("codex"),
    });
  }
  return targets;
}
```

消息投递也改为查 `attachedAgents.get(agent)`：

```typescript
// 发给 Claude
const ws = state.attachedAgents.get("claude");
if (ws) sendProtocolMessage(ws, { type: "routed_message", message: msg });

// 发给 Codex（通过 MCP，不再 injectMessage）
const ws = state.attachedAgents.get("codex");
if (ws) sendProtocolMessage(ws, { type: "routed_message", message: msg });
```

### Codex 消息来源变更

**Before**: Codex 输出通过 app-server WebSocket → `notification-handler` → `codex-events` → daemon inject PTY。

**After**: Codex 通过 MCP `reply` tool 发消息 → bridge → daemon `route_message`。与 Claude 完全对称。

`codex-events.ts` 的 `turnCompleted` 不再负责消息转发。Codex agent 自己决定何时调用 `reply`。

### codex-events.ts 简化

移除 `turnCompleted` 中的消息转发和 PTY inject 逻辑。保留：
- `phaseChanged`（GUI 状态指示）
- `agentMessageStarted`/`agentMessageDelta`/`agentMessage`（GUI 流式显示）
- `ready`/`tuiConnected`/`tuiDisconnected`/`error`/`exit`（生命周期事件）
- `turnCompleted`（仅日志，不转发）

移除：
- `routeMessage` dep
- `sendToClaudePty` dep
- PTY inject 逻辑
- `forwardPrompt` 使用

### adapter 重命名

`ClaudeAdapter`（MCP server）→ `AgentMcpAdapter`：
- `setClaudeRole` → `setAgentRole`
- 类名改为通用

### GUI 影响

前端仍然通过 daemon WS 4503 收 `agent_message` 事件，只渲染 `content`。无变更。

Codex 的流式消息（`agentMessageStarted`/`delta`/`agentMessage`）仍通过 app-server → daemon → GUI 显示。MCP `reply` 是最终的结构化路由消息。

## Files Changed

| 文件 | 变更 |
|------|------|
| `daemon/bridge.ts` | 读 `AGENTBRIDGE_AGENT` env, `agent_connect` 代替 `claude_connect` |
| `daemon/adapters/claude-adapter/claude-adapter.ts` | 重命名为 `agent-mcp-adapter.ts`, `ClaudeAdapter` → `AgentMcpAdapter` |
| `daemon/adapters/claude-adapter/index.ts` | 更新导出 |
| `daemon/control-protocol.ts` | `claude_connect/disconnect` → `agent_connect/disconnect { agentId }` |
| `daemon/daemon-state.ts` | `attachedClaude` → `attachedAgents: Map` |
| `daemon/daemon-client/index.ts` | `attachClaude()` → `attachAgent(agentId)` |
| `daemon/daemon-client/connection.ts` | 无变化（已用 `routed_message`） |
| `daemon/control-server/handler.ts` | `agent_connect`/`agent_disconnect` 处理 |
| `daemon/control-server/claude-session.ts` | 重命名为 `agent-session.ts`, 泛化 attach/detach |
| `daemon/control-server/message-routing.ts` | `resolveTarget` 查 `attachedAgents`, 投递查 Map |
| `daemon/codex-events.ts` | 移除 routeMessage/sendToClaudePty/PTY inject, 简化为 GUI 事件 |
| `daemon/daemon.ts` | 适配新 deps（移除 route/sendToClaudePty from codex-events deps） |
| `daemon/session-manager.ts` | 创建 CODEX_HOME 时写入 `mcp.json` |
| `daemon/gui-server/server.ts` | role_sync 查 `attachedAgents` |

## Not Changed

- MCP tools 接口（reply/check_messages/get_status）
- 前端（GUI 只渲染 content）
- 角色系统（ROLES/defaultTarget/per-agent roles）
- Claude 侧注入方式（--mcp-config 内联）
- Codex app-server WebSocket（仍用于 session 管理/sandbox/配置）
