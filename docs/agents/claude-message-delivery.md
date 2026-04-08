# Claude 消息投递链路

## 概述

Claude 的消息通过两条独立路径到达 GUI，daemon 用状态机去重保证同一 turn 只显示一次。

## 两条投递路径

### 路径 A：Bridge reply() tool（主路径）

```
Claude → MCP reply(to, text, status) → bridge/tools.rs
  → WS :4502 agent_reply → control/handler.rs
  → claim_claude_bridge_terminal_delivery()
  → routing::route_message() → GUI
```

**触发条件**：Claude 调用 MCP `reply()` tool。
**消息来源**：bridge sidecar 通过 WS control channel 发送 `agent_reply`。
**去重标记**：`ClaudeSdkDirectTextState::CompletedByBridge`。

### 路径 B：SDK result 直投（兜底路径）

```
Claude SDK → POST /claude/events → event_handler.rs
  → handle_result() → extract_assistant_text()
  → claim_sdk_terminal_delivery()
  → build_direct_sdk_gui_message()
  → routing::route_message() → GUI
```

**触发条件**：Claude 结束 turn 时 SDK 自动发送 `result` 事件。
**消息来源**：Claude SDK HTTP POST 的 `type: "result"` 事件。
**去重标记**：`ClaudeSdkDirectTextState::CompletedBySdk`。

## 去重状态机

```
ClaudeSdkDirectTextState:
  Inactive        → 空闲，等待新 turn
  Active          → assistant 文本事件已开始，SDK 直投通道已锁定
  CompletedBySdk  → SDK result 已投递，bridge reply 将被抑制
  CompletedByBridge → bridge reply 已投递，SDK result 将被抑制
```

### 状态转换

```
新 turn 开始 (assistant event):
  Inactive/Completed* → Active（如果 bridge 未 attach）
  Inactive → 保持 Inactive（如果 bridge 已 attach，等 bridge 投递）

SDK result 到达 (handle_result):
  Active/Inactive → CompletedBySdk → 投递到 GUI
  CompletedByBridge → 抑制（bridge 已投递）
  CompletedBySdk → 抑制（已投递过）

Bridge reply 到达 (agent_reply, terminal status):
  Active → CompletedByBridge → 投递到 GUI
  Inactive → 投递到 GUI（bridge 不经过 turn 状态）
  CompletedBySdk → 抑制（SDK 已投递）
  CompletedByBridge → 抑制（已投递过）

Turn 结束:
  任意状态 → Inactive
```

## 关键文件

| 文件 | 职责 |
|------|------|
| `daemon/claude_sdk/event_handler.rs` | 处理 SDK POST 事件，调用 claim_sdk_terminal_delivery |
| `daemon/claude_sdk/event_handler_delivery.rs` | SDK 投递辅助函数 |
| `daemon/control/handler.rs:143-164` | 处理 bridge agent_reply，调用 claim_claude_bridge_terminal_delivery |
| `daemon/state_delivery.rs:30-51` | 去重状态机实现 |
| `daemon/state.rs` | ClaudeSdkDirectTextState 枚举定义 |
| `bridge/src/tools.rs` | reply() MCP tool 定义 |

## reply() tool 的作用

`reply(to, text, status)` 是 MCP tool，用途是**跨 agent 路由**：

- `reply(to="user")` — Claude 向用户回复（经过 bridge → daemon → GUI）
- `reply(to="coder")` — Claude 向 coder agent 发消息（经过 bridge → daemon → Codex）
- `reply(to="lead")` — worker 向 lead 汇报

reply() 和 SDK result 的关系：
- Claude 的 prompt 要求 `reply()` 作为主投递方式（`You MUST call reply() before ending any turn`）
- SDK result 是传输层自动产生的，作为 reply() 未被调用时的兜底
- 两者内容相同时由状态机去重

## 已知行为

1. Claude 有时不调 reply() 直接结束 turn → SDK result 兜底投递
2. Claude 调了 reply(to="user") 且 SDK 也有 result → 先到先赢，另一条去重
3. Claude 调 reply(to="coder") → bridge 路由到 Codex，SDK result 仍会投递给 GUI（因为 to 不同，不冲突）
4. bridge 未连接时 → SDK result 始终直投
