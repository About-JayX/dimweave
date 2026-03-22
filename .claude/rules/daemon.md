---
paths:
  - "daemon/**/*.ts"
---

# Daemon 开发规范

- 运行时为 Bun，类型用 `bun-types`（tsconfig.daemon.json）
- 所有适配器实现 `adapters/base-adapter.ts` 中的 `AgentAdapter` 接口
- 新增 Agent 适配器放 `daemon/adapters/`
- 日志统一写 `/tmp/agentbridge.log`，格式 `[ISO timestamp] [ModuleName] message`
- GUI 事件通过 `broadcastToGui()` 推送，事件类型: `agent_message` | `agent_status` | `system_log` | `daemon_status`
- 每条消息带 `source` 字段 ("claude" | "codex")，不回传给来源方（防循环）
