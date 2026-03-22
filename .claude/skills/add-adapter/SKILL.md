---
name: add-adapter
description: 创建新的 Agent 适配器。当需要添加新 AI agent（如 Gemini、Cursor 等）的桥接支持时使用。
disable-model-invocation: true
argument-hint: <agent-name>
---

为 `$ARGUMENTS` 创建一个新的 Agent 适配器，按以下步骤执行：

1. **创建适配器文件** `daemon/adapters/$0-adapter.ts`
   - 继承 EventEmitter，实现 `AgentAdapter` 接口（见 `daemon/adapters/base-adapter.ts`）
   - 必须实现: `name`, `displayName`, `status`, `start()`, `stop()`, `sendMessage()`
   - 事件: `message` (BridgeMessage), `statusChange` (AgentStatus), `error` (Error)

2. **参考已有适配器**
   - 阅读 `daemon/adapters/claude-adapter.ts`（MCP 模式）
   - 阅读 `daemon/adapters/codex-adapter.ts`（WebSocket 代理模式）
   - 选择合适的通信模式

3. **在 daemon 中注册**
   - 在 `daemon/daemon.ts` 中导入并实例化适配器
   - 连接消息事件到 `emitToClaude()` 和 `broadcastToGui()`
   - 更新 `currentStatus()` 加入新 agent 状态

4. **更新前端**
   - 在 `src/hooks/useWebSocket.ts` 的 agents 初始状态中添加新 agent
   - `AgentStatus.tsx` 会自动渲染新 agent 卡片

5. **更新类型**
   - 在 `daemon/types.ts` 的 `MessageSource` 中添加新来源
   - 在 `src/types.ts` 的 `MessageSource` 中同步更新

6. **日志规范**
   - 使用 `[${AgentName}Adapter]` 前缀
   - 写入 `/tmp/agentbridge.log`
