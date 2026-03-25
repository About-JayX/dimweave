# AgentBridge

通用 AI Agent 桥接桌面应用。当前实现把 **Tauri/Rust 主进程** 作为唯一常驻后端，把 **Claude Code** 和 **Codex app-server** 接到同一个消息路由层里，让用户在一个桌面界面里协调两个 agent。

### 当前产品形态

| 组件 | 当前实现 |
|------|----------|
| 桌面壳 | Tauri 2 |
| 主后端 | Rust 内嵌 async daemon（`src-tauri/src/daemon/`） |
| Claude 接入 | 外部终端启动 `claude` + 项目 `.mcp.json` 注册 Rust bridge sidecar |
| Codex 接入 | Rust daemon 启动 `codex app-server` 并通过 WS 建立 session |
| 桥接 sidecar | Rust 二进制 `agent-bridge-bridge`（`bridge/` crate） |
| 前端 | React 19 + Vite + TypeScript + Tailwind CSS v4 |

### 硬性约束

| 约束 | 说明 |
|------|------|
| 不写 `~/` | 文档和规则统一写仓库相对路径或 `$HOME/...` |
| 当前运行时无 Bun daemon | Bun 只保留为前端包管理/脚本运行器，不再承载后端常驻进程 |
| Source of Truth 只认当前代码 | 以 `src-tauri/src/daemon/`、`bridge/`、`src/`、`.claude/rules/` 为准 |
| 历史设计文档不当现状 | `docs/superpowers/**` 主要是迁移记录，不代表当前实现 |
| 每个源码文件最多 200 行 | 超过必须拆分模块（此限制不适用于 CLAUDE.md 等文档文件） |

## 技术栈

- **桌面应用**: Tauri 2 + Rust
- **内嵌 daemon**: tokio + axum + tokio-tungstenite
- **Claude bridge sidecar**: Rust MCP stdio server + daemon WS client
- **前端**: React 19 + Vite + TypeScript + Tailwind CSS v4 + Zustand + shadcn/ui
- **外部工具**: Claude Code CLI、Codex CLI
- **通信协议**: MCP stdio、WebSocket、Tauri `invoke` / `listen`、Codex JSON-RPC 2.0

## 当前架构

```text
┌─ Claude Code（外部终端） ───────────────────────────────────────┐
│  读取项目 .mcp.json                                            │
│  spawn: agent-bridge-bridge                                    │
└───────────────┬─────────────────────────────────────────────────┘
                │ MCP stdio
                ▼
┌─ bridge/agent-bridge-bridge ────────────────────────────────────┐
│ tools.rs         → reply tool                                   │
│ mcp.rs           → Claude Channel notification / tools/list     │
│ channel_state.rs → reply target tracking / permission cache     │
│ mcp_protocol.rs  → RPC parsing / initialize result              │
│ daemon_client.rs → WS client → 127.0.0.1:4502                   │
└───────────────┬─────────────────────────────────────────────────┘
                │ WS :4502
                ▼
┌─ Tauri 主进程 / Rust daemon ────────────────────────────────────┐
│ main.rs                   → commands 注册 + daemon task 启动     │
│ mcp.rs                    → .mcp.json 注册 + 启动 Claude 终端    │
│ claude_cli.rs             → Claude CLI 版本校验 + channel 启动    │
│ codex/auth|oauth|usage    → 账号/OAuth/用量/模型                 │
│ daemon/control/           → bridge 接入与消息投递                │
│ daemon/routing.rs         → Claude / Codex / GUI 路由            │
│ daemon/codex/             → app-server 生命周期 + session        │
│ daemon/session_manager.rs → 临时 CODEX_HOME 生命周期             │
└───────────────┬─────────────────────────────────────────────────┘
                │ invoke / listen
                ▼
┌─ React 前端 ────────────────────────────────────────────────────┐
│ bridge-store      → 监听 agent_message / system_log / status     │
│ ClaudePanel       → register_mcp + launch_claude_terminal        │
│ AgentStatus/      → CodexPanel / RoleSelect / StatusDot          │
│ MessagePanel      → 消息与日志与 Permission 审批                  │
└──────────────────────────────────────────────────────────────────┘

Codex app-server ← WS :4500 → Rust daemon/codex/session.rs
```

## 运行链路

### Claude 链路

1. 前端选择项目目录后调用 `register_mcp`。
2. Tauri 在项目根写入 **`.mcp.json`**，注册 `agent-bridge-bridge`。
3. 前端再调用 `launch_claude_terminal`，打开外部终端并在该目录运行 `claude`。
4. Claude Code 读取项目 `.mcp.json`，以 MCP stdio 方式启动 bridge sidecar。
5. bridge 通过 `ws://127.0.0.1:4502/ws` 连入内嵌 daemon。
6. Permission 链路: bridge `permission_request` → daemon → GUI → `permission_verdict` → bridge

### Codex 链路

1. 前端调用 `daemon_launch_codex`。
2. `session_manager.rs` 创建 `/tmp/agentbridge-<pid>-<sessionId>/` 临时 `CODEX_HOME`。
3. 当前实现会写入：
   - `auth.json` symlink → `$HOME/.codex/auth.json`
   - `config.toml` → `sandbox_mode` / `approval_policy` / `apply_patch_freeform=false`
4. daemon 启动 `codex app-server --listen ws://127.0.0.1:4500`。
5. `session.rs` 通过 WS 完成 `initialize` + `thread/start`，并注册动态工具：
   - `reply`
   - `check_messages`
   - `get_status`

### 消息路由

- **用户 → Codex**: 前端 `daemon_send_message` → `routing.rs` → `codex_inject_tx`
- **Claude → Codex**: bridge `reply` tool → WS :4502 → `routing.rs` → `codex_inject_tx`
- **Codex → Claude**: Codex 动态 `reply` → `routing.rs` → bridge WS channel → Claude Channel notification
- **任一 agent → user**: `to = "user"` 时只发到 GUI
- **离线缓冲**: 目标离线时写入 `buffered_messages`，Claude bridge 重连或 Codex session 启动后回放
- **Sender gating**: Claude 只接受 `user`/`system`/当前 `codex_role`；control WS 只接受 `claude`/`codex`

## 端口分配

| 端口 | 用途 | 当前实现 |
|------|------|----------|
| `4500` | Codex app-server WebSocket | `src-tauri/src/daemon/codex/` |
| `4502` | bridge ↔ daemon 控制通道 | `src-tauri/src/daemon/control/` |
| `1420` | Vite dev server | `bun run dev` |

当前 **没有** GUI WebSocket `4503`，也没有 PTY 通道端口。

## 角色系统

角色定义以 `src-tauri/src/daemon/role_config/roles.rs` 为准：

| 角色 | 当前作用 | Codex 约束 |
|------|----------|------------|
| `user` | 管理员直控 | `workspace-write` / `never` |
| `lead` | 决策与汇总 | `workspace-write` / `never` |
| `coder` | 代码实现 | `workspace-write` / `never` |
| `reviewer` | 审查 | `read-only` / `never` |
| `tester` | 测试 | `read-only` / `never` |

注意当前实现的真实边界：

- Codex 角色约束已经接到启动链路中。
- Claude 角色目前主要用于 **路由标签和 UI 状态**。
- 当前仓库 **没有** 把 Claude 角色通过 `--agent --agents` 注入到 CLI。
- 当前仓库 **没有** 实现 Starlark rules、AGENTS 合并、编排器三模式。

## 会话管理

`src-tauri/src/daemon/session_manager.rs` 负责临时 `CODEX_HOME` 生命周期：

```text
创建会话
  → /tmp/agentbridge-<pid>-<sessionId>/
  → auth.json symlink（如存在）
  → config.toml
  → 启动 codex app-server

结束会话
  → stop child
  → cleanup_session(sessionId)
  → Drop 时 cleanup_all()
```

当前实现 **不会** 写：

- `rules/role.rules`
- `AGENTS.md`
- `mcp.json` 到 `CODEX_HOME`

## MCP 注册

当前 MCP 注册是 **项目级**、**显式用户触发** 的：

- Tauri command: `src-tauri/src/mcp.rs`
- 配置文件位置: 项目根 **`.mcp.json`**
- sidecar 命令: `agent-bridge-bridge`

当前仓库没有单独的 `agentbridge mcp register` CLI。

## 常用命令

```bash
bun run dev         # Vite dev server
bun run build       # 前端构建
bun run tauri dev   # Tauri 桌面应用开发模式（会先构建 bridge sidecar）
bun run bridge      # 单独构建 Rust bridge sidecar
cargo test          # 运行 Rust 测试
```

## 开发规范

详细规范按路径自动加载，见 `.claude/rules/`:

- `architecture.md` — 全局架构、数据流、模块边界
- `frontend.md` — 前端规范（匹配 `src/**/*.{ts,tsx}`）
- `tauri.md` — Tauri / Rust / 内嵌 daemon 规范（匹配 `src-tauri/**`）
- `daemon.md` — bridge sidecar 与 daemon 协议规范（匹配 `bridge/**`）

**迁移约定:**

- Bun 只作为前端包管理和脚本工具使用，不再承担后端 daemon 职责
- 任何架构描述都必须优先对照当前代码树，不要照搬迁移期 spec
- 修改 `bridge/**`、`src-tauri/src/daemon/**`、`.claude/rules/**` 时，必须同步更新本文件或 `UPDATE.md`
- 历史迁移文档可以保留，但必须明确标注为 archival，不可当作 Source of Truth

**闭环要求:**

- 遇到 bug 或设计问题，修复后必须把根因和解法写入对应 rules 文件和踩坑记录
- 每次架构变更必须同步更新本文件的架构图和 `UPDATE.md`
- **每个源码文件最多 200 行**，超过必须拆分模块（文档文件不受此限制）
- 完成功能后必须补充对应 UI 或可见状态
- 执行完任务后必须调用 superpowers 代码审计（`superpowers:requesting-code-review`）
- Rust 改动后必须重新运行 Tauri / Cargo 校验，前端改动后至少跑一次 TS / build 校验

**Agent 链路修复文档要求（强制）:**

- **每次**修复或发现 agent 链路 bug，必须立即记录到 `docs/agents/<agent>-chain.md`
- 记录内容必须包含：问题描述、根因分析、修复方案、运行时验证结果
- 错误的修复尝试也必须记录（包括失败原因），防止重复犯错
- 未修复的已知问题必须记录，标注 `[未修复]` 和原因
- 每个 agent 一个文档：`claude-chain.md`、`codex-chain.md`
- 修复内容必须对照官方文档，标注官方文档 URL
- **官方文档与实际实现可能不一致，以运行时测试为准**
- 运行时验证结果（成功/失败）必须回填到文档

## 技能系统

项目技能按三层组织：

- `.claude/skills/` — Claude Code 自动发现入口
- `.agents/skills/` — 仓库内托管的共享 skill 实体目录
- `skills-lock.json` — skill 来源与 hash 锁定文件

当前仓库内置技能：

- 项目自定义：`add-adapter`、`debug-daemon`
- 共享镜像：`aceternity-ui`、`rust-async-patterns`、`rust-pro`、`shadcn`、`superpowers`、`tailwind-css-patterns`、`vercel-react-best-practices`

**维护约定:**

- `.claude/skills/<name>` 如果是 symlink，真实内容以 `.agents/skills/<name>/` 为准
- 新增共享 skill 时，同时更新 `.agents/skills/`、`.claude/skills/` 和 `skills-lock.json`
- 已明确 vendored 到仓库的共享 skill 必须在 `.agents/skills/<name>/` 落地并可提交；不要让 `.gitignore` 把项目级 skill 屏蔽掉
- 新增项目私有 skill 时，直接创建 `.claude/skills/<name>/SKILL.md`
- 用户全局 skills 目录只作为个人环境兜底，不属于仓库协作约定

## 模块结构

### Cargo Workspace

```text
Cargo.toml                 # workspace root
bridge/                    # Rust bridge sidecar crate
src-tauri/                 # Tauri app crate
src/                       # React frontend
```

### bridge

```text
bridge/src/
├── main.rs                # sidecar entry
├── daemon_client.rs       # WS client → 4502
├── mcp.rs                 # MCP stdio server / channel notification
├── mcp_protocol.rs        # RPC message parsing + initialize result
├── channel_state.rs       # Claude channel state + reply target tracking
├── tools.rs               # reply tool schema + parsing
└── types.rs               # bridge ↔ daemon protocol mirror
```

### Tauri / Rust

```text
src-tauri/src/
├── main.rs
├── mcp.rs
├── claude_cli.rs           # Claude CLI version check + channel preview launch
├── codex/
│   ├── auth.rs
│   ├── models.rs
│   ├── oauth.rs
│   ├── oauth_helpers.rs
│   └── usage.rs
└── daemon/
    ├── mod.rs
    ├── gui.rs
    ├── routing.rs
    ├── session_manager.rs
    ├── state.rs
    ├── types.rs
    ├── control/
    │   ├── handler.rs
    │   ├── mod.rs
    │   └── server.rs
    ├── codex/
    │   ├── handler.rs
    │   ├── lifecycle.rs
    │   ├── mod.rs
    │   └── session.rs
    └── role_config/
        ├── mod.rs
        └── roles.rs
```

### Frontend

```text
src/
├── App.tsx
├── main.tsx
├── types.ts
├── index.css
├── animations.css
├── utilities.css
├── stores/
│   ├── bridge-store/
│   │   ├── index.ts
│   │   └── types.ts
│   └── codex-account-store.ts
├── components/
│   ├── AgentStatus/
│   │   ├── index.tsx
│   │   ├── CodexPanel.tsx
│   │   ├── AuthActions.tsx
│   │   ├── CodexUsageSection.tsx
│   │   ├── CodexConfigRows.tsx
│   │   ├── CodexHeader.tsx
│   │   ├── RoleSelect.tsx
│   │   └── StatusDot.tsx
│   ├── ClaudePanel/
│   │   └── index.tsx
│   ├── CodexAccountPanel/
│   │   ├── MiniMeter.tsx
│   │   └── helpers.ts
│   ├── MessagePanel/
│   │   ├── index.tsx
│   │   ├── PermissionQueue.tsx
│   │   ├── SourceBadge.tsx
│   │   └── TabBtn.tsx
│   ├── ReplyInput.tsx
│   ├── MessageMarkdown.tsx
│   └── ui/
└── lib/
    └── utils.ts
```

## 安全与边界

- `auth.json` 通过 symlink 透传，不复制凭证内容
- Codex 权限边界当前主要依赖 `sandbox_mode`、`approval_policy`、`apply_patch_freeform=false`
- Claude MCP 注册是项目级 `.mcp.json`，由用户显式点击触发
- bridge 只负责 MCP/WS 协议转换，不负责业务决策
- CSP 已配置为 `default-src 'self'`，限制 XSS 风险
- 非 macOS 平台使用 `std::process::Command` 直接启动 Claude CLI，避免 shell 注入

## 当前状态

### 已实现

- Rust 内嵌 daemon（无独立 Bun daemon）
- Rust bridge sidecar（Cargo workspace 成员）
- Claude 项目级 `.mcp.json` 注册
- Claude channel preview 启动链路与版本 preflight
- Claude channel `instructions` / `reply(chat_id, text)` / permission relay
- 外部终端启动 Claude CLI
- Codex account / OAuth / models / usage
- 临时 `CODEX_HOME` + `auth.json` symlink + `config.toml`
- Rust control server + routing + message buffering
- Tauri event 驱动的消息 / 日志 / agent 状态 / permission prompt 同步

### 已删除或不再适用

- `daemon/**/*.ts` Bun daemon 体系
- GUI WebSocket `:4503`
- PTY 注入链路、`portable-pty`、`node-pty`
- 旧 `test-e2e.ts` / `test-routing.ts` / `test-codex-mcp.ts` 手工脚本
- `tsconfig.daemon.json`

### 尚未实现，但旧文档中曾经提过的内容

- Claude CLI `--agent --agents` 角色注入
- Starlark rules / `rules/role.rules`
- AGENTS/指令合并注入
- 三模式 orchestrator
- 独立 MCP register/unregister CLI

## 历史文档说明

- `UPDATE.md` 记录当前这轮 Rust 架构整理结果
- `docs/superpowers/plans/**`、`docs/superpowers/specs/**` 是带日期的迁移记录
- 历史文档可用于追溯设计决策，但当前实现以本文件和 `.claude/rules/` 为准
