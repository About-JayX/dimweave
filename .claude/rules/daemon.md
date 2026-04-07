---
paths:
  - "bridge/**"
---

# Bridge / Daemon 协议规范

- `bridge/**` 是 Rust sidecar，不是旧 Bun daemon 的替代目录
- bridge 只负责两件事：
  1. MCP stdio 协议
  2. daemon 控制通道 WS 客户端
- 业务状态、角色状态、消息缓冲、Codex session 生命周期都属于 `src-tauri/src/daemon/**`

## 协议边界

- `bridge/src/types.rs` 和 `src-tauri/src/daemon/types.rs` 的消息字段必须保持兼容
- bridge 发给 daemon 的消息统一走：
  - `agent_connect`
  - `agent_reply`
  - `permission_request`
  - `get_online_agents`
  - `agent_disconnect`
- daemon 发给 bridge 的消息统一走：
  - `routed_message`
  - `permission_verdict`
  - `online_agents_response`

## 工具边界

- bridge 当前暴露 2 个 MCP tool：`reply` 和 `get_online_agents`
- `reply` tool 契约为 `to + text`，Claude 自行决定路由目标，bridge 纯转发
- Claude channel initialize 结果必须带 `instructions`，并显式声明 `experimental['claude/channel']` 与 `experimental['claude/channel/permission']`
- 如果要给 Claude 增加新 tool，必须同时更新：
  - `bridge/src/tools.rs`
  - `bridge/src/mcp.rs`
  - `CLAUDE.md`
  - 相关规则文档
- 不要把 Codex 动态工具实现塞进 bridge；Codex 工具属于 `src-tauri/src/daemon/codex/handler.rs`

## 连接与重连

- `bridge/src/daemon_client.rs` 负责自动重连
- 连接成功后必须立即发 `agent_connect`
- 断线后只能重连 daemon，不要在 bridge 内缓存长期业务状态
- bridge 只允许把 `user`、`system`、`lead`、`coder` 这些内部 sender 转成 Claude channel 事件
- bridge 只保留 pending permission 短期缓存，不能演变成业务状态数据库

## 文件规模

- **每个文件最多 200 行**，超过必须拆分
- 协议类型、MCP 处理、WS 客户端分别放独立文件

## 修改要求

- 改 sidecar 二进制名时，同时更新：
  - `bridge/Cargo.toml`
  - `src-tauri/src/mcp.rs`
  - `src-tauri/build.rs`
  - `src-tauri/tauri.conf.json`
- 改 WS 地址或健康检查路径时，同时更新 `.claude/skills/debug-daemon/SKILL.md`
