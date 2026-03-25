# AgentBridge Rust 架构整理记录

> 更新日期: 2026-03-25
> 目的: 把仓库中的“当前实现”与“迁移期历史文档”彻底分开，清除 Bun daemon / PTY / GUI WS 时代的残留描述。

## 当前 Source of Truth

- [`CLAUDE.md`](/Users/jason/floder/agent-bridge/CLAUDE.md)
- [`.claude/rules/architecture.md`](/Users/jason/floder/agent-bridge/.claude/rules/architecture.md)
- [`.claude/rules/frontend.md`](/Users/jason/floder/agent-bridge/.claude/rules/frontend.md)
- [`.claude/rules/tauri.md`](/Users/jason/floder/agent-bridge/.claude/rules/tauri.md)
- [`.claude/rules/daemon.md`](/Users/jason/floder/agent-bridge/.claude/rules/daemon.md)
- [`src-tauri/src/daemon/`](/Users/jason/floder/agent-bridge/src-tauri/src/daemon)
- [`bridge/`](/Users/jason/floder/agent-bridge/bridge)

## 这次统一后的真实架构

| 维度 | 当前实现 |
|------|----------|
| 常驻后端 | Tauri 主进程内嵌 Rust daemon |
| Claude 接入 | 项目 `.mcp.json` + 外部终端运行 `claude` |
| bridge | Rust sidecar 二进制 `agent-bridge-bridge` |
| Codex 接入 | Rust daemon 启动 `codex app-server` |
| GUI 通信 | Tauri `invoke` / `listen` |
| daemon 控制通道 | WS `127.0.0.1:4502/ws` |
| Codex 会话通道 | WS `127.0.0.1:4500` |

## 已移除或清理的旧体系

- Bun daemon 目录与其规则假设
- GUI WebSocket `:4503`
- PTY 注入链路与对应测试脚本
- `node-pty` / `portable-pty` 时代的前端残留文件
- `tsconfig.daemon.json`
- 不再使用的前端 `agent-roles.ts`

## 当前实现与旧文档最大的差异

| 旧说法 | 当前真实情况 |
|--------|--------------|
| Claude 通过 `--strict-mcp-config --mcp-config <json>` 注入 | 当前是写项目 `.mcp.json`，再打开终端运行 `claude` |
| Bun daemon 负责路由与会话管理 | 当前全部迁入 `src-tauri/src/daemon/` |
| GUI 通过 `:4503` WebSocket 收事件 | 当前前端直接监听 Tauri 事件 |
| Claude 用 `--agent --agents` 强制角色 | 当前 Claude 角色只是路由层状态，不是 CLI 注入 |
| Starlark rules / AGENTS 合并已上线 | 当前代码里没有这条链路 |

## 当前消息链路

### Claude

1. `register_mcp` 写项目 `.mcp.json`
2. `launch_claude_terminal` 打开外部终端运行 `claude`
3. Claude 启动 `agent-bridge-bridge`
4. bridge 连 `ws://127.0.0.1:4502/ws`
5. daemon 通过 `agent_message` / `agent_status` / `system_log` 同步到前端

### Codex

1. `daemon_launch_codex`
2. 创建临时 `CODEX_HOME`
3. 启动 `codex app-server --listen ws://127.0.0.1:4500`
4. `session.rs` 建立 session 并注册动态工具
5. 用户/Claude 发往 Codex 的消息通过 `codex_inject_tx` 直送 session

## 历史文档处理原则

- `docs/superpowers/plans/**`、`docs/superpowers/specs/**` 保留为 **历史迁移记录**
- 历史文档顶部必须明确标注 archival，避免被误读为当前架构
- 新的架构调整优先更新 `CLAUDE.md` 和 `.claude/rules/**`
