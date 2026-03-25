---
paths:
  - "src/**/*.{ts,tsx}"
---

# 前端开发规范

## 技术栈

- React 19 + TypeScript + Vite
- Tailwind CSS v4
- Zustand
- shadcn/ui
- Tauri `invoke` / `listen`

## 当前前端边界

- 前端不再维护 GUI WebSocket 客户端
- 前端不再维护 PTY / xterm 终端链路
- agent 运行状态来自 Rust 事件：
  - `agent_message`
  - `agent_status`
  - `system_log`
- 用户操作通过 Tauri command：
  - `daemon_send_message`
  - `daemon_launch_codex`
  - `daemon_stop_codex`
  - `daemon_set_claude_role`
  - `register_mcp`
  - `launch_claude_terminal`

## 状态管理

- 运行时状态统一收口到 `src/stores/bridge-store/`
- store 负责事件监听和 invoke 调用，组件不重复写订阅逻辑
- selector 返回值必须稳定，禁止在 selector 内直接 `.map()` / `.filter()`

## UI 约束

- 自定义组件放 `src/components/`
- shadcn 组件放 `src/components/ui/`
- 所有消息展示都以 `BridgeMessage` 为准，不再保留旧 `source` 字段兼容层
- “是否连接 Codex” 这类交互态必须用真实 `agent_status` 驱动

## 性能与安全

- 禁止在前端使用 Node.js API
- 需要系统信息或文件选择时，统一走 Tauri command
- 日志视图和消息视图只消费现有事件，不在前端推导不存在的后端状态

## 代码检查

- 每次修改后执行 `bun x tsc --noEmit -p tsconfig.app.json`
- 如果改了前端依赖或删了旧组件，必须同步清理 `package.json`
