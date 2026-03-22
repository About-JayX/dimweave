# 架构概览

```
Claude Code <--MCP--> bridge.ts <--WS--> daemon.ts <--WS Proxy--> Codex TUI
                                             |
                                        GUI WS (4503)
                                             |
                                      Tauri/React 前端
```

## 端口分配

| 端口 | 用途 | 服务 |
|------|------|------|
| 4500 | Codex app-server WebSocket | codex-adapter.ts |
| 4501 | Codex proxy (TUI 连接) | codex-adapter.ts |
| 4502 | daemon 控制端口 (bridge <-> daemon) | daemon.ts |
| 4503 | GUI WebSocket (daemon -> 前端) | daemon.ts |
| 1420 | Vite dev server | vite |

## 消息流

- **Codex -> Claude**: codex-adapter 拦截 agentMessage -> daemon 转发 -> bridge -> MCP notification -> Claude
- **Claude -> Codex**: Claude 调用 reply 工具 -> bridge -> daemon -> codex.injectMessage
- **GUI 实时同步**: daemon 所有消息事件同时广播到 4503 GUI WebSocket
