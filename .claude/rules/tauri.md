---
paths:
  - "src-tauri/**"
---

# Tauri / Rust 开发规范

- Tauri 2 主进程负责：
  - 内嵌 async daemon
  - Codex 账号 / OAuth / 用量 / 模型
  - 项目 `.mcp.json` 注册
  - Claude channel preview preflight + 启动外部 Claude 终端

## 当前模块职责

- `main.rs` — commands 注册、daemon task 启动
- `mcp.rs` — 项目 `.mcp.json` 注册、检查、Claude CLI 版本校验、preview 启动
- `codex/auth.rs` — 读取 `$HOME/.codex/auth.json`
- `codex/oauth.rs` — OAuth 登录 / 取消 / 登出
- `codex/usage.rs` — 用量查询
- `codex/models.rs` — 模型缓存读取
- `commands.rs` — Tauri command handlers（从 main.rs 拆出）
- `claude_session/` — Claude PTY 会话管理（spawn/stop/prompt 自动确认）
- `claude_launch.rs` — Terminal 启动 helpers（macOS/other）
- `daemon/mod.rs` — command channel + daemon 主循环
- `daemon/state.rs` — 运行时共享状态
- `daemon/routing.rs` — 唯一消息投递入口
- `daemon/control/` — bridge 控制通道 WS server
- `daemon/codex/` — Codex app-server 生命周期、session、动态工具
- `daemon/session_manager.rs` — 临时 `CODEX_HOME` 生命周期
- `daemon/role_config/roles.rs` — 角色约束定义

## 当前 Commands

- `get_codex_account`
- `refresh_usage`
- `list_codex_models`
- `pick_directory`
- `register_mcp`
- `check_mcp_registered`
- `launch_claude_terminal`
- `stop_claude`
- `claude_terminal_input`
- `claude_terminal_resize`
- `codex_login`
- `codex_cancel_login`
- `codex_logout`
- `daemon_send_message`
- `daemon_send_user_input`
- `daemon_launch_codex`
- `daemon_stop_codex`
- `daemon_set_claude_role`
- `daemon_set_codex_role`
- `daemon_respond_permission`
- `daemon_get_status_snapshot`

新增 command 时，必须同步注册到 `main.rs` 的 `invoke_handler`。

## 当前 Events

- `agent_message`
- `system_log`
- `agent_status`
- `permission_prompt`
- `claude_terminal_data`
- `claude_terminal_reset`
- `claude_terminal_status`
- `claude_terminal_attention`
- `codex_stream`

新增事件时：

- Rust payload 用 `#[serde(rename_all = "camelCase")]`
- 前端 store 第一时间接入
- `CLAUDE.md` / rules 同步更新

## 会话与路由要求

- 所有消息路由都必须走 `routing.rs`
- 不要在 `control/handler.rs` 或 `codex/session.rs` 里复制路由规则
- Claude permission request / verdict 必须走 `daemon/state.rs` + `daemon/control/handler.rs` 的显式协议，不要伪装成普通聊天消息
- `session_manager.rs` 当前只负责 `auth.json` symlink 和 `config.toml`
- 如果后续新增 Starlark、AGENTS、MCP 注入到 `CODEX_HOME`，必须先更新文档再实现

## 构建与打包

- bridge sidecar 路径变更时，必须同步更新：
  - `src-tauri/build.rs`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/src/mcp.rs`
- `.mcp.json` 中的 bridge command 当前有意写绝对路径，这是 Tauri 打包形态要求，不要擅自改回文档示例里的相对脚本路径
- `beforeDevCommand` / `beforeBuildCommand` 必须保证 `agent-bridge-bridge` 先被构建，否则 `.mcp.json` 注册出来的命令会指向不存在的二进制
- 当前运行时不依赖 Bun daemon，但前端构建仍由 `bun run dev/build` 驱动

## 校验要求

- Rust 改动后至少执行 `cargo test`
- 结构性改动后建议执行 `cargo check`
- 删除旧架构残留时，要同步删掉陈旧测试、陈旧依赖和陈旧注释
- 每次做完 Tauri / daemon / Rust 链路审查后，必须同步更新 `docs/agentbridge-audit-summary.md`；如果涉及 Claude / Codex / bridge 专项协议，还要同步更新对应 `docs/agents/*.md`
