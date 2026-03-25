# Bridge 角色路由

> Historical note: this dated spec was written for the Bun-daemon migration period. Current source of truth is `CLAUDE.md`, `.claude/rules/**`, `src-tauri/src/daemon/**`, and `bridge/**`.

## Summary

将 bridge 从"全量转发"改为"结构化路由"。Agent 之间完全解耦，不互相监听，所有消息通过 bridge 按 `from`/`to` 角色分发。目标角色不在线时 bridge 以 system 消息回复。GUI 只渲染 `content`。

## Motivation

当前 Claude 全路监听所有 Codex 输出（turnCompleted → PTY inject），浪费 token 且无法定向通信。改为 bridge 中央路由后：
- Agent 只收到发给自己角色的消息
- 支持角色执行模式（Lead 分任务 → Coder → Reviewer → Tester）
- 减少无关信息注入，节省 token

## Message Format

```typescript
// daemon/types.ts
type MessageSource = "claude" | "codex" | "user" | "system";

interface BridgeMessage {
  id: string;
  from: string;           // 发送者角色: "lead" | "coder" | "reviewer" | "tester" | "user" | "system"
  to: string;             // 目标角色: "lead" | "coder" | ... | "user"
  content: string;        // 消息体（GUI 只渲染这个）
  timestamp: number;
  type?: "task" | "review" | "result" | "question" | "system";
  replyTo?: string;       // 关联的原消息 id
  priority?: "normal" | "urgent";
}
```

**Breaking changes**:
- `source` 字段改名为 `from`
- `daemon/types.ts` `MessageSource` 从 `"claude" | "codex"` 扩展为 `"claude" | "codex" | "user" | "system"`

## Routing Logic

### Bridge 路由表

Bridge (daemon) 维护一个角色→agent 映射表，基于已有的 `claudeRole`/`codexRole`：

```typescript
function resolveTarget(to: string): {
  targets: Array<{ agent: "claude" | "codex"; online: boolean }>;
} {
  if (to === "user") return { targets: [] }; // GUI only, no agent forwarding
  const matches: Array<{ agent: "claude" | "codex"; online: boolean }> = [];
  if (state.claudeRole === to) {
    matches.push({ agent: "claude", online: state.attachedClaude !== null });
  }
  if (state.codexRole === to) {
    matches.push({ agent: "codex", online: codex.activeThreadId !== null });
  }
  return { targets: matches };
}
```

### 路由规则

1. `to` 匹配 `claudeRole` → 发给 Claude（PTY inject + MCP push）
2. `to` 匹配 `codexRole` → 发给 Codex（injectMessage）
3. `claudeRole === codexRole` → 两个都匹配 → 发给两个（广播）
4. `to === "user"` → 只发给 GUI（不转发给任何 agent）
5. 无匹配 → bridge 返回 system 消息: `"{to} 角色不在线"`
6. 匹配但不在线 → 同上

**注意**: 不支持 `to: "all"`。初始版本不需要广播，角色执行模式通过显式 `to` 定向通信。如果 `claudeRole === codexRole` 时两个 agent 同角色是合法的（并行思考模式），消息会发给两个。

### Sender Validation

`route_message` 处理时验证 `message.from` 必须等于 `state.claudeRole`（因为只有 Claude 通过 control protocol 发消息）。不匹配则拒绝。

### System 消息格式

```typescript
{
  id: "system_...",
  from: "system",
  to: msg.from,         // 回复给发送者
  content: "reviewer 角色不在线",
  timestamp: Date.now(),
  type: "system",
  replyTo: msg.id,
}
```

## Data Flow

### Claude → Target (via MCP reply tool)

```
Claude (lead) 调用 reply(to: "coder", text: "请修复第42行")
  → bridge.ts → daemonClient.routeMessage({ from: claudeRole, to: "coder", ... })
  → daemon control-server → 验证 from === claudeRole → resolveTarget("coder")
    → codexRole === "coder" && online → codex.injectMessage(content)
    → GUI 广播 agent_message（只渲染 content）
  → 返回给 Claude: "Message routed to coder."
  → 不在线 → 返回 system 消息 "coder 角色不在线"
```

### Codex → Target (via turnCompleted)

```
Codex turn 完成
  → notification-handler.ts 构建 BridgeMessage { from: "codex", ... }
  → codex-events.ts 收到 agentMessage → 设置 from: state.codexRole, to: "lead"
  → resolveTarget("lead")
    → claudeRole === "lead" && online → emitToClaude + PTY inject（只注入 content）
    → GUI 广播 agent_message（只渲染 content）
    → 不在线 → system 消息广播到 GUI
```

**Codex `from`/`to` 字段设置位置**: `notification-handler.ts` 构建原始消息时设置 `from: "codex"`（占位），`codex-events.ts` 在 `agentMessage` 事件中覆盖为 `from: state.codexRole, to: "lead"`（因为只有这里能访问 `state.codexRole`）。

### GUI 用户 → Codex

```
用户在 GUI 输入消息
  → send_to_codex { content: "..." }
  → daemon 构建 { from: "user", to: state.codexRole, content, type: "task" }
  → codex.injectMessage(content)
  → GUI 广播（只渲染 content）
```

## MCP Tool Changes

### reply tool

```typescript
// Before
reply({ text: string })
// Description: "Send a message to Codex."

// After
reply({ to: string, text: string, type?: string, replyTo?: string, priority?: string })
// Description: "Send a message to a target agent role. Use get_status to see available roles."
```

- `to` — 必填，目标角色（"lead" | "coder" | "reviewer" | "tester" | "user"）
- `text` — 必填，消息内容
- `type` — 可选，消息意图
- `replyTo` — 可选，关联消息 id
- `priority` — 可选，默认 "normal"

### check_messages

无参数变更。返回的消息格式从 `source` 改为 `from`，包含新字段。
格式化输出: `[time] ${m.from}: ${m.content}`（`from` 现在是角色名，更有意义）。

### get_status

新增返回：当前角色分配（`claudeRole`/`codexRole`）和在线状态，让 Claude 知道能发给谁。

```
Available roles:
  lead (claude) - online
  coder (codex) - online
```

## Control Protocol Changes

```typescript
// Before
type ControlClientMessage =
  | { type: "claude_to_codex"; requestId: string; message: BridgeMessage }

// After
type ControlClientMessage =
  | { type: "route_message"; requestId: string; message: BridgeMessage }

// Before
type ControlServerMessage =
  | { type: "claude_to_codex_result"; requestId: string; success: boolean; error?: string }
  | { type: "codex_to_claude"; message: BridgeMessage }

// After
type ControlServerMessage =
  | { type: "route_result"; requestId: string; success: boolean; error?: string }
  | { type: "routed_message"; message: BridgeMessage }
```

## GUI Rendering

**只渲染 `content`**。`from`/`to`/`type`/`replyTo`/`priority` 不显示在消息气泡中。

- `SourceBadge` 用 `from` 字段显示来源标签（角色名）
- `MessagePanel` 渲染 `content`
- 新字段对 GUI 完全透明

## Files Changed

| File | Change |
|------|--------|
| **daemon/types.ts** | `MessageSource` 扩展为 4 值; `BridgeMessage`: `source` → `from`, 新增 `to`/`type`/`replyTo`/`priority` |
| **daemon/daemon-state.ts** | `systemMessage()`: `source: "codex"` → `from: "system"`, 加 `to` 参数 |
| **daemon/adapters/claude-adapter/claude-adapter.ts** | reply tool: 加 `to` 参数, 更新 description; check_messages: `m.source` → `m.from`; get_status: 加角色信息 |
| **daemon/adapters/codex-message-handler/notification-handler.ts** | `source: "codex"` → `from: "codex"` (占位), 加 `to: ""` 占位 |
| **daemon/adapters/codex-message-handler/types.ts** | `emitAgentMessage` 签名跟随 BridgeMessage 变更 |
| **daemon/adapters/claude-adapter/base-adapter.ts** | BridgeMessage 接口引用跟随变更 |
| **daemon/control-protocol.ts** | `claude_to_codex` → `route_message`, `codex_to_claude` → `routed_message`, `claude_to_codex_result` → `route_result` |
| **daemon/control-server/handler.ts** | `route_message` 处理 + `resolveTarget` + sender validation |
| **daemon/control-server/message-routing.ts** | `emitToClaude` → `routeToAgent`, `sendBridgeMessage` 适配; `codex_to_claude` → `routed_message` |
| **daemon/control-server/claude-session.ts** | `state.systemMessage()` 调用加 `to` 参数 |
| **daemon/codex-events.ts** | `agentMessage`: 覆盖 `from: state.codexRole, to: "lead"`; `agentMessageStarted` payload: `source` → `from`; turnCompleted: 按角色路由 |
| **daemon/gui-server/handlers.ts** | `send_to_codex`: `source: "user"` → `from: "user"`, 加 `to: state.codexRole` |
| **daemon/bridge.ts** | `replySender` 传完整 BridgeMessage（含 `to`） |
| **daemon/daemon-client/index.ts** | `sendReply`: `claude_to_codex` → `route_message` |
| **daemon/daemon-client/connection.ts** | `codex_to_claude` → `routed_message`, `claude_to_codex_result` → `route_result` |
| **daemon/daemon-client/types.ts** | `codexMessage` 事件考虑改名 `routedMessage` |
| **src/types.ts** | `MessageSource` 扩展; `BridgeMessage`: `source` → `from`, 新增可选字段 |
| **src/stores/bridge-store/message-handler.ts** | `source` → `from` (agent_message_started, agent_message, system_log) |
| **src/components/MessagePanel/SourceBadge.tsx** | `source` → `from` |
| **src/components/MessagePanel/index.tsx** | 消息过滤逻辑: `source` → `from` |
| **.claude/rules/daemon.md** | 更新反循环规则: "不回传给 source" → "按 to 字段路由" |

## Not Changed

- `ROLES` 定义不变（路由用 `claudeRole`/`codexRole` 查找）
- `RoleSelect` 组件不变
- `StatusDot` 不变
- GUI 布局不变（消息面板只渲染 content，无新 UI）
- 硬约束体系不变（sandbox/Starlark/approval 已在角色定义中）

## Migration

`source` → `from` 是 breaking change。需要同步更新：
- daemon 所有构建 BridgeMessage 的地方（types.ts, notification-handler.ts, codex-events.ts, gui-server/handlers.ts, daemon-state.ts, claude-session.ts）
- 前端所有读取 `source` 的地方（SourceBadge, message-handler, MessagePanel）
- `MessageSource` type 值保持不变（"claude" | "codex" | "user" | "system"），只改字段名
- daemon 侧 `MessageSource` 从 2 值扩展到 4 值
