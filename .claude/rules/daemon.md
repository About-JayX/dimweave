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
- 每条消息带 `from`/`to` 字段（角色名），bridge 按 `to` 字段路由到目标 agent，`skipSender` 防循环

## 文件规模
- **每个文件最多 500 行**，超过必须拆分
- daemon 模块结构: `daemon.ts`(入口) / `daemon-state.ts`(共享状态) / `gui-server.ts` / `control-server.ts`
- adapter 模块结构: `codex-adapter.ts`(编排) / `codex-message-handler.ts` / `codex-response-patcher.ts` / `codex-port-utils.ts` / `codex-types.ts`

## 封装与抽离
- 每个 Agent 适配器独立封装，通过 EventEmitter 暴露事件，daemon.ts 不直接访问内部状态
- 公共类型定义放 `daemon/types.ts`、`daemon/control-protocol.ts`、`daemon/adapters/codex-types.ts`
- 工具函数和可复用逻辑抽为独立模块
- 服务器模块通过依赖注入（deps 参数）获取共享依赖，不直接 import daemon.ts 中的变量
- 新 session 的启动副作用（如协议注入、首次状态广播）只能有一个权威入口，禁止在 `ready` 事件和 GUI 成功回调里重复触发

## 性能优化
- 避免高频广播：同类型状态变更做节流（如 rateLimits 更新）
- WebSocket 消息序列化只做一次，多个 client 共享同一 JSON 字符串
- Map/Set 及时清理过期条目，防止内存泄漏

## 代码检查
- 每次修改后必须执行 `npx tsc --noEmit -p tsconfig.daemon.json` 确保零类型错误
- 不允许未处理的 Promise rejection，异步操作必须有 catch 或 try/catch
- GUI/daemon 的 `.then()` 链必须以 `.catch()` 收口，并向 GUI 写入可见错误日志

## 测试与重启
- Daemon 代码（`daemon/**/*.ts`）没有 HMR，修改后**必须重启**：`pkill -f "bun run daemon"; sleep 1; bun run daemon/index.ts &`
- Rust 代码（`src-tauri/**`）修改后需要重新编译，**必须重启 Tauri**：`pkill -f "target/debug/agent-bridge"; bun run tauri dev &`
- 前端代码（`src/**/*.{ts,tsx}`）有 Vite HMR，修改后无需手动重启
- 每次改动后必须验证生效：改了 daemon 就重启 daemon，改了 Rust 就重启 Tauri，改了前端确认 HMR 热更新成功
