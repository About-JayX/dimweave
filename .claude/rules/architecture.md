# 架构概览

## 当前运行拓扑

```text
React UI
  ↕ Tauri invoke / listen
Tauri main.rs
  ├── mcp.rs                  # .mcp.json 注册 + 启动 Claude 终端
  ├── codex/*                 # 账号 / OAuth / 用量 / 模型
  └── daemon/*
      ├── control server      # WS :4502，bridge 连入
      ├── routing             # Claude / Codex / GUI 路由
      ├── codex session       # WS :4500，连 codex app-server
      └── session manager     # 临时 CODEX_HOME

Claude Code
  ↕ MCP stdio
bridge/agent-bridge-bridge
  ↕ WS :4502
Rust daemon

Codex app-server
  ↕ WS :4500
Rust daemon
```

## 端口分配

| 端口 | 用途 | 所属模块 |
|------|------|----------|
| `4500` | Codex app-server WebSocket | `src-tauri/src/daemon/codex/` |
| `4502` | bridge ↔ daemon 控制通道 | `src-tauri/src/daemon/control/` |
| `1420` | Vite dev server | 前端开发环境 |

当前没有 GUI WebSocket `4503`。

## 核心数据流

### Claude 方向

- 前端调用 `register_mcp`
- Tauri 在项目根写 `.mcp.json`
- 外部终端运行 `claude`
- Claude 通过 MCP 启动 `agent-bridge-bridge`
- bridge 用 WS 连内嵌 daemon

### Codex 方向

- 前端调用 `daemon_launch_codex`
- daemon 创建临时 `CODEX_HOME`
- 启动 `codex app-server`
- `session.rs` 完成 `initialize` / `thread/start`
- daemon 通过注入 channel 给 Codex 发送输入

### 路由方向

- `routing.rs` 是消息投递权威入口
- `to = "user"` 只显示到 GUI
- `to = claude_role` 走 bridge WS
- `to = codex_role` 走 `codex_inject_tx`
- 离线消息进入 `buffered_messages`

## 模块边界

### `bridge/**`

- 只负责协议转换，不承载业务状态
- `bridge/src/types.rs` 必须和 `src-tauri/src/daemon/types.rs` 保持字段兼容
- 新增或修改 bridge tool 时，同时检查 daemon 路由和前端文档

### `src-tauri/src/daemon/**`

- 只在这里维护运行时状态、角色状态、缓冲队列、Codex session
- 所有消息路由统一走 `routing.rs`
- bridge control server 只做接入和转发，不写业务分支

### `src/**`

- 前端只做 UI、状态呈现、用户触发
- 不在前端复制 daemon 业务逻辑
- agent 状态来源于 Rust 事件，不手写“猜测状态”

## 架构变更要求

- 改 `bridge` 接入方式时，同时更新 `src-tauri/src/mcp.rs`、`bridge/**`、`CLAUDE.md`
- 改消息协议时，同时更新 Rust daemon types、bridge types、前端 `BridgeMessage`
- 改端口时，同时更新规则、文档、桥接代码和健康检查路径
- 删除旧模块后，要同步删除对应测试脚本、tsconfig、依赖和说明文档
