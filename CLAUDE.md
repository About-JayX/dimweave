# AgentBridge

通用 AI Agent 桥接 GUI 桌面应用，让多个 AI 编程助手（Claude Code、Codex、未来 Gemini 等）在同一台机器上实时双向通信。

## 技术栈

- **桌面壳**: Tauri 2 (Rust)
- **前端**: React 19 + Vite + TypeScript
- **后端 daemon**: Bun + TypeScript
- **通信协议**: MCP + WebSocket + JSON-RPC 2.0

## 常用命令

```bash
bun run daemon    # 启动 daemon（后端必须先启动）
bun run dev       # 启动前端开发模式（浏览器）
bun run tauri dev # 启动 Tauri 桌面应用（含前端）
bun run build     # 构建前端
bun run bridge    # MCP bridge（由 Claude Code 通过 MCP 配置自动启动）
```

## 开发规范

详细规范按路径自动加载，见 `.claude/rules/`:
- `architecture.md` — 架构、端口、消息流
- `daemon.md` — daemon 端规范（匹配 `daemon/**/*.ts`）
- `frontend.md` — 前端规范（匹配 `src/**/*.{ts,tsx}`）
- `tauri.md` — Tauri 规范（匹配 `src-tauri/**`）

## 自定义技能

- `/add-adapter <agent-name>` — 创建新 Agent 适配器的完整流程
- `/debug-daemon` — 排查 daemon 运行问题

## MCP 注册

```json
{
  "mcpServers": {
    "agentbridge": {
      "command": "bun",
      "args": ["run", "/Users/jason/floder/agent-bridge/daemon/bridge.ts"]
    }
  }
}
```

## 当前状态 (MVP)

已实现: Tauri 壳、daemon 消息路由、Claude MCP 适配器、Codex WS 代理适配器、GUI 面板、一键连接 Codex

待实现: Gemini CLI 适配器、多会话支持、消息搜索、Agent 编排、设置页面
