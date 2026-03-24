# Bridge 角色路由

## Summary

将 bridge 从"全量转发"改为"结构化路由"。Agent 之间完全解耦，不互相监听，所有消息通过 bridge 按 `from`/`to` 角色分发。目标角色不在线时 bridge 以 system 消息回复。GUI 只渲染 `content`。

## Motivation

当前 Claude 全路监听所有 Codex 输出（turnCompleted → PTY inject），浪费 token 且无法定向通信。改为 bridge 中央路由后：
- Agent 只收到发给自己角色的消息
- 支持角色执行模式（Lead 分任务 → Coder → Reviewer → Tester）
- 减少无关信息注入，节省 token

## Message Format

```typescript
interface BridgeMessage {
  id: string;
  from: string;           // 发送者角色: "lead" | "coder" | "reviewer" | "tester" | "user"
  to: string;             // 目标角色: "lead" | "coder" | ... | "all"
  content: string;        // 消息体（GUI 只渲染这个）
  timestamp: number;
  type?: "task" | "review" | "result" | "question" | "system";
  replyTo?: string;       // 关联的原消息 id
  priority?: "normal" | "urgent";
}
```

**Breaking change**: `source` 字段改名为 `from`。

## Routing Logic

### Bridge 路由表

Bridge (daemon) 维护一个角色→agent 映射表，基于已有的 `claudeRole`/`codexRole`：

```typescript
function resolveTarget(to: string): {
  target: "claude" | "codex" | null;
  online: boolean;
} {
  if (to === "all") return { target: null, online: true }; // 特殊处理
  if (state.claudeRole === to) {
    return { target: "claude", online: state.attachedClaude !== null };
  }
  if (state.codexRole === to) {
    return { target: "codex", online: codex.activeThreadId !== null };
  }
  return { target: null, online: false };
}
```

### 路由规则

1. `to` 匹配 `claudeRole` → 发给 Claude（PTY inject 或 MCP push）
2. `to` 匹配 `codexRole` → 发给 Codex（injectMessage）
3. `to === "all"` → 广播给所有在线 agent
4. 无匹配 → bridge 返回 system 消息: `"{to} 角色不在线"`
5. 匹配但不在线 → 同上

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

### Claude → Codex (via MCP reply tool)

```
Claude 调用 reply(to: "coder", text: "请修复第42行")
  → bridge.ts → daemonClient.routeMessage({ from: "lead", to: "coder", ... })
  → daemon control-server → resolveTarget("coder")
    → codexRole === "coder" && online → codex.injectMessage(content)
    → GUI 广播 agent_message（只渲染 content）
  → 返回给 Claude: "Message routed to coder (codex)."
```

### Codex → Claude (via turnCompleted)

```
Codex turn 完成
  → daemon codex-events → 构建 BridgeMessage { from: codexRole, to: "lead", ... }
  → resolveTarget("lead")
    → claudeRole === "lead" && online → emitToClaude + PTY inject（只注入 content）
    → GUI 广播 agent_message（只渲染 content）
    → 不在线 → system 消息广播到 GUI
```

**关键变更**: Codex turnCompleted 不再无条件注入 Claude PTY，而是按 `to` 路由。默认 `to: "lead"`（Codex 输出默认发给 Lead 审核）。

### GUI 用户 → Codex

```
用户在 GUI 输入消息
  → send_to_codex { content: "..." }
  → daemon 构建 { from: "user", to: codexRole, content, type: "task" }
  → codex.injectMessage(content)
  → GUI 广播（只渲染 content）
```

## MCP Tool Changes

### reply tool

```typescript
// Before
reply({ text: string })

// After
reply({ to: string, text: string, type?: string, replyTo?: string, priority?: string })
```

- `to` — 必填，目标角色
- `text` — 必填，消息内容
- `type` — 可选，消息意图
- `replyTo` — 可选，关联消息 id
- `priority` — 可选，默认 "normal"

### check_messages

无参数变更。返回的消息格式从 `source` 改为 `from`，包含新字段。

### get_status

新增：返回当前角色分配（`claudeRole`/`codexRole`）和在线状态，让 Claude 知道能发给谁。

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

- `SourceBadge` 用 `from` 字段显示来源标签（Claude/Codex/System/User）
- `MessagePanel` 渲染 `content`
- 新字段对 GUI 完全透明

## Files Changed

| File | Change |
|------|--------|
| `daemon/types.ts` | `BridgeMessage`: `source` → `from`, 新增 `to`/`type`/`replyTo`/`priority` |
| `daemon/adapters/claude-adapter/claude-adapter.ts` | reply tool 加 `to`/`type`/`replyTo`/`priority` 参数 |
| `daemon/control-protocol.ts` | `claude_to_codex` → `route_message`, `codex_to_claude` → `routed_message` |
| `daemon/control-server/handler.ts` | `route_message` 处理 + `resolveTarget` |
| `daemon/control-server/message-routing.ts` | `emitToClaude` → `routeToAgent`, 新增路由逻辑 |
| `daemon/codex-events.ts` | turnCompleted: 构建带 `from`/`to` 的消息, 按角色路由而非全量转发 |
| `daemon/gui-server/handlers.ts` | `send_to_codex`: 构建 `from: "user"` 消息 |
| `daemon/daemon-state.ts` | `systemMessage` 方法适配新格式 |
| `daemon/bridge.ts` | replySender 传完整 BridgeMessage（含 to） |
| `daemon/daemon-client/` | 适配新协议名 |
| `src/types.ts` | `BridgeMessage`: `source` → `from`, 新增可选字段 |
| `src/stores/bridge-store/message-handler.ts` | 适配 `from` 字段 |
| `src/components/MessagePanel/SourceBadge.tsx` | `source` → `from` |
| `src/components/MessagePanel/index.tsx` | 消息过滤逻辑适配 |

## Not Changed

- `ROLES` 定义不变（路由用 `claudeRole`/`codexRole` 查找）
- `RoleSelect` 组件不变
- `StatusDot` 不变
- GUI 布局不变（消息面板只渲染 content，无新 UI）
- 硬约束体系不变（sandbox/Starlark/approval 已在角色定义中）

## Migration

`source` → `from` 是 breaking change。需要同步更新：
- daemon 所有构建 BridgeMessage 的地方
- 前端所有读取 `source` 的地方（SourceBadge, message-handler, 条件渲染）
- `MessageSource` type 保持值不变（"claude" | "codex" | "user" | "system"），只改字段名
