# Dimweave

通用 AI Agent 桥接桌面应用。当前实现把 **Tauri/Rust 主进程** 作为唯一常驻后端，把 **Claude Code** 和 **Codex app-server** 接到同一个消息路由层里，并由 daemon 维护标准化的 **task / session / artifact** 图，让用户在一个桌面界面里协调两个 agent、恢复历史会话并查看任务上下文。

### 当前产品形态

| 组件 | 当前实现 |
|------|----------|
| 桌面壳 | Tauri 2 |
| 主后端 | Rust 内嵌 async daemon（`src-tauri/src/daemon/`） |
| Claude 接入 | `--sdk-url` + `stream-json` transport；bridge sidecar 仅保留为 MCP tools 提供者 |
| Codex 接入 | Rust daemon 启动 `codex app-server` 并通过 WS 建立 session |
| 桥接 sidecar | Rust 二进制 `dimweave-bridge`（`bridge/` crate），当前主要服务 Claude MCP tools |
| 会话记忆 | provider-native history + daemon `task_graph` + runtime resume |
| 前端 | React 19 + Vite + TypeScript + Tailwind CSS v4 + Zustand + task-centric shell |

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
- **Claude MCP bridge sidecar**: Rust MCP stdio server + daemon WS client
- **前端**: React 19 + Vite + TypeScript + Tailwind CSS v4 + Zustand + shadcn/ui
- **外部工具**: Claude Code CLI、Codex CLI
- **通信协议**: MCP stdio、WebSocket、Tauri `invoke` / `listen`、Codex JSON-RPC 2.0

## Claude 文档地图

- 当前采用方案: [docs/agents/claude-docs-index.md](/Users/jason/floder/agent-bridge/docs/agents/claude-docs-index.md)
- 当前 Claude 主链路验证: [docs/agents/claude-sdk-url-validation.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-validation.md)
- 当前与旧链路差异: [docs/channel-vs-sdk-url-diff.md](/Users/jason/floder/agent-bridge/docs/channel-vs-sdk-url-diff.md)
- `--sdk-url` 协议逆向: [docs/agents/claude-sdk-url-protocol-deep-dive.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-protocol-deep-dive.md)
- 全量备选方案分析: [docs/claude-code-integration-alternatives.md](/Users/jason/floder/agent-bridge/docs/claude-code-integration-alternatives.md)
- 旧 channel 合同参考: [docs/agents/claude-channel-api.md](/Users/jason/floder/agent-bridge/docs/agents/claude-channel-api.md)
- 历史修复总账: [docs/agents/claude-chain.md](/Users/jason/floder/agent-bridge/docs/agents/claude-chain.md)

## 当前架构

```text
┌─ Claude Code（--sdk-url child） ───────────────────────────────┐
│  WS /claude 接收 NDJSON user/control_response                  │
│  POST /claude/events 回传 system/assistant/result/control_request │
│  同时读取项目 .mcp.json / inline strict MCP config            │
└───────────────┬─────────────────────────────────────────────────┘
                │ MCP stdio（tools only）
                ▼
┌─ bridge/dimweave-bridge ────────────────────────────────────┐
│ tools.rs         → reply + get_online_agents tools             │
│ mcp.rs           → MCP tools/list / call                        │
│ channel_state.rs → legacy channel state + shared reply helpers  │
│ mcp_protocol.rs  → RPC parsing / initialize result              │
│ daemon_client.rs → WS client → 127.0.0.1:4502/ws                │
└───────────────┬─────────────────────────────────────────────────┘
                │ WS :4502/ws
                ▼
┌─ Tauri 主进程 / Rust daemon ────────────────────────────────────┐
│ main.rs                   → commands 注册 + daemon task 启动     │
│ mcp.rs                    → .mcp.json 注册 + inline MCP config    │
│ claude_cli.rs             → Claude CLI 版本校验                    │
│ codex/auth|oauth|usage    → 账号/OAuth/用量/模型                 │
│ daemon/control/           → bridge WS + Claude SDK WS/HTTP       │
│ daemon/routing.rs         → Claude / Codex / GUI 路由            │
│ daemon/codex/             → app-server 生命周期 + session        │
│ daemon/session_manager.rs → 临时 CODEX_HOME 生命周期             │
│ daemon/task_graph/        → task/session/artifact 标准化持久化    │
│ daemon/provider/          → Claude/Codex history + resume adapter│
└───────────────┬─────────────────────────────────────────────────┘
                │ invoke / listen
                ▼
┌─ React 前端 ────────────────────────────────────────────────────┐
│ bridge-store      → 监听 agent_message / system_log / claude_stream / codex_stream │
│ task-store        → 监听 task/session/artifact/provider history                   │
│ ClaudePanel       → project picker + history dropdown + connect/resume            │
│ AgentStatus/      → CodexPanel / RoleSelect / StatusDot                           │
│ TaskPanel         → session tree / artifact timeline                              │
│ MessagePanel      → 消息与日志与 Permission 审批                  │
└──────────────────────────────────────────────────────────────────┘

Codex app-server ← WS :4500 → Rust daemon/codex/session.rs
```

**Claude 当前采用方案：** `--sdk-url` transport + MCP tools bridge。
**Claude 非当前方案：** PTY/channel transport、纯 stdio stream-json transport、Agent SDK sidecar。

## 运行链路

### Claude 链路

1. `ClaudePanel` 先根据当前项目目录拉 Claude provider history；用户可选择 `New session` 或某个历史 `session_id`。
2. 前端选择项目目录后调用 `register_mcp`。
3. Tauri 在项目根写入 **`.mcp.json`**，注册 `dimweave-bridge`。
4. 前端调用 `daemon_launch_claude_sdk`，由 Tauri 直接启动 `claude --sdk-url ws://127.0.0.1:4502/claude?launch_nonce=...`。
5. Claude Code 读取项目 `.mcp.json`，以 MCP stdio 方式启动 bridge sidecar；bridge 继续提供 `reply(to, text, status)` 和 `get_online_agents()`。
6. Claude 通过 `WS /claude` 收取宿主 NDJSON，通过 `POST /claude/events` 回传 `system/user/assistant/result/control_request` 事件；`launch_nonce` 用于绑定当前 launch 并允许安全重连。
7. SDK 模式当前 **不走 GUI permission gate**：运行时保留 `--dangerously-skip-permissions`，daemon 对偶发 `control_request(can_use_tool)` 直接回 `control_response(allow)`；因此 `permission_request -> GUI -> permission_verdict` 只属于旧 channel 链路。
8. Claude 新会话显式分配 `--session-id`，恢复历史会话走 `--resume <session_id>`，两者都统一走 `--sdk-url` 运行时。

### Codex 链路

1. `CodexPanel` 先根据当前项目目录拉 Codex provider history；用户可选择 `New session` 或某个历史 `thread_id`。
2. 前端调用 `daemon_launch_codex`。
3. `session_manager.rs` 创建 `/tmp/dimweave-<pid>-<sessionId>/` 临时 `CODEX_HOME`。
4. 当前实现会写入：
   - `auth.json` symlink → `$HOME/.codex/auth.json`
   - `config.toml` → `sandbox_mode` / `approval_policy` / `apply_patch_freeform=false`
5. daemon 启动 `codex app-server --listen ws://127.0.0.1:4500`。
6. `session.rs` 通过 WS 完成 `initialize` + `thread/start`，并注册动态工具：
   - `reply`
   - `check_messages`
   - `get_status`
7. Codex history 优先走 `thread/list`；离线时回退扫描 `$HOME/.codex/sessions/**/*.jsonl`
8. 恢复时重新连回原 thread，而不是把 transcript 重放给模型

### 消息路由

- **用户 → agent**: 前端 `daemon_send_user_input` → `routing::route_user_input`（单次 GUI echo + 内部 fan-out）
- **Claude → Codex**: bridge `reply` tool → WS :4502/ws → `routing.rs` → `codex_inject_tx`
- **Codex → Claude**: `routing.rs` 直接写 Claude SDK WS NDJSON user message；Claude 可继续调用 bridge tools
- **任一 agent → user**: `to = "user"` 时只发到 GUI
- **离线缓冲**: 目标离线时写入 `buffered_messages`，Claude bridge 重连或 Codex session 启动后回放
- **Sender gating**: Claude 只接受 `user`/`system`/当前 `codex_role`；bridge control WS 只接受 `claude`/`codex`

## Task / Session Memory

- `src-tauri/src/daemon/task_graph/` 是标准化 `task / session / artifact` 事实源，并带 JSON 持久化快照
- `src-tauri/src/daemon/provider/claude.rs` 负责 Claude transcript history index 与 runtime resume
- `src-tauri/src/daemon/provider/codex.rs` / `history.rs` 负责 Codex thread history、离线 fallback 与 provider-agnostic history DTO
- `src/stores/task-store/` 把 daemon 的 task/session/artifact/provider-history 事件水合到前端 store
- `src/components/ClaudePanel/` 与 `src/components/AgentStatus/CodexPanel.tsx` 是 provider history 的主入口：按 `cwd` 拉历史，选择后决定 `new` 或 `resumed` 连接
- `src/components/TaskPanel/` 现在只展示 active normalized task 的 session tree 和 artifact timeline，不再负责 provider history picker
- `daemon_attach_provider_history` 仍然存在，但它是“把外部 provider history 显式挂到当前 task”的 task 语义入口，不等同于 provider-native connect/resume

### 三种会话语义

当前项目里必须明确区分这三层状态，很多历史 bug 都来自把它们混为一谈：

1. **provider history**
   - Claude 的 `session_id` / transcript
   - Codex 的 `thread_id`
   - 来源于 provider 自身的历史存储或 API，不要求先有 active task
2. **live provider connection**
   - 当前在线的 Claude/Codex 运行时连接
   - daemon 通过 `ProviderConnectionState { provider, external_session_id, cwd, connection_mode }` 广播给前端
   - 只表示“现在连着哪条外部会话”，不自动等同于 task attachment
3. **normalized task session**
   - daemon `task_graph` 里的持久化 session 记录
   - 可以绑定外部 `session_id` / `thread_id`
   - 只有进入 task graph 后，才会参与 artifact timeline 和 session tree

结论：`provider history`、`live connection`、`normalized task session` 不是同一个概念，UI 和文档都必须保持这个边界。

### 当前已知限制

- Claude 的 provider-native resume 已接通，但当前前端仍然只消费稳定的 `thinking…` / 最终结果，不展示 `claude_stream.preview` 文本
- 这是当前实现的有意选择，避免 `stream_event` 级摘要噪音直接污染消息区
- provider history 当前按 workspace/cwd 查询；如果没有 active task，也仍然可以在 Claude/Codex 面板里查看历史并恢复连接
- `resume_session()` 既可用于 normalized session 指针恢复，也会在 provider 具备外部会话 id 时尝试触发真实 runtime reconnect；不要再把它理解成“只移动 task graph 指针”的旧逻辑

## 端口分配

| 端口 | 用途 | 当前实现 |
|------|------|----------|
| `4500` | Codex app-server WebSocket | `src-tauri/src/daemon/codex/` |
| `4502` | Claude SDK WS/HTTP + bridge ↔ daemon 控制通道 | `src-tauri/src/daemon/control/` |
| `1420` | Vite dev server | `bun run dev` |

当前 **没有** GUI WebSocket `4503`。Claude 当前也不再走 `claude_session/` PTY runtime。

## 角色系统

角色定义以 `src-tauri/src/daemon/role_config/roles.rs` 为准：

| 角色 | 当前作用 | Codex 约束 |
|------|----------|------------|
| `user` | 管理员直控 | `workspace-write` / `never` |
| `lead` | 决策与汇总 | `workspace-write` / `never` |
| `coder` | 代码实现 | `workspace-write` / `never` |
注意当前实现的真实边界：

- Codex 角色约束已经接到启动链路中。
- Claude 角色目前主要用于 **路由标签和 UI 状态**。
- 当前仓库 **没有** 把 Claude 角色通过 `--agent --agents` 注入到 CLI。
- 当前仓库 **没有** 实现 Starlark rules、AGENTS 合并、编排器三模式。

## 会话管理

`src-tauri/src/daemon/session_manager.rs` 负责临时 `CODEX_HOME` 生命周期：

```text
创建会话
  → /tmp/dimweave-<pid>-<sessionId>/
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
- sidecar 命令: `dimweave-bridge`

当前仓库没有单独的 `dimweave mcp register` CLI。

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

**Plan-first 强制规范:**

- **所有非 trivial 改动（涉及 3 个以上文件或跨模块）必须先生成 plan 文档**，路径 `docs/superpowers/plans/YYYY-MM-DD-<topic>.md`
- Plan 必须包含：目标、根因/动机、文件映射、分步任务、验收标准
- **实施完成后必须把 commit 记录回填到 plan 文档**，包含 commit hash、摘要、code review 发现
- 不允许跳过 plan 直接实施——即使"看起来很简单"
- Trivial 单文件修复不受此限制，但仍需在 commit message 中说明根因

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
├── mcp.rs                 # MCP stdio loop
├── mcp_io.rs              # write_line, tool response, inbound dispatch
├── mcp_protocol.rs        # RPC parsing + initialize result
├── channel_state.rs       # reply target tracking / permission cache
├── tools.rs               # reply + get_online_agents tool schema + parsing
└── types.rs               # bridge ↔ daemon protocol mirror
```

### Tauri / Rust

```text
src-tauri/src/
├── main.rs                 # entry + setup
├── commands.rs             # Tauri command handlers
├── mcp.rs                  # .mcp.json 注册 + Claude launch
├── claude_cli.rs           # Claude CLI version check
├── claude_launch.rs        # Terminal launch helpers (macOS/other)
├── claude_session/         # Claude PTY session management
│   ├── mod.rs
│   ├── process.rs          # spawn/kill/signal helpers
│   └── prompt.rs           # dev confirmation auto-answer
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
    ├── routing_format.rs        # agent-specific 消息格式化 + 附件上下文注入
    ├── session_manager.rs
    ├── state.rs
    ├── types.rs
    ├── types_dto.rs             # 前端 DTO（TaskSnapshot, HistoryEntry 等）
    ├── control/
    │   ├── handler.rs
    │   ├── mod.rs
    │   └── server.rs
    ├── codex/
    │   ├── handler.rs
    │   ├── handshake.rs    # initialize + thread/start WS handshake
    │   ├── lifecycle.rs
    │   ├── mod.rs
    │   ├── session.rs
    │   ├── session_event.rs # Codex WS event processing
    │   └── ws_client.rs     # Codex WS pump loop
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
│   │   ├── helpers.ts
│   │   ├── sync.ts
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
│   │   ├── index.tsx
│   │   ├── ClaudeConfigRows.tsx
│   │   ├── DevConfirmDialog.tsx
│   │   └── dev-confirm.ts
│   ├── CodexAccountPanel/
│   │   ├── MiniMeter.tsx
│   │   └── helpers.ts
│   ├── MessagePanel/
│   │   ├── index.tsx
│   │   ├── MessageList.tsx
│   │   ├── MessageBubble.tsx
│   │   ├── ClaudeTerminalPane.tsx
│   │   ├── CodexStreamIndicator.tsx
│   │   ├── claude-terminal-config.ts
│   │   ├── PermissionQueue.tsx
│   │   ├── SourceBadge.tsx
│   │   └── TabBtn.tsx
│   ├── ReplyInput/
│   │   ├── index.tsx          # 主输入组件（拖拽 + 附件按钮 + 发送）
│   │   ├── TargetPicker.tsx   # 目标选择下拉
│   │   ├── AttachmentStrip.tsx # 附件预览条
│   │   └── use-attachments.ts # 附件状态管理 hook
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
- Claude `--sdk-url` 启动链路与版本 preflight
- Claude MCP bridge `reply(to, text, status)` / `get_online_agents()` + SDK transport
- SDK 控制面权限 auto-allow（不再经过 GUI permission relay）
- Codex account / OAuth / models / usage
- 临时 `CODEX_HOME` + `auth.json` symlink + `config.toml`
- Rust control server + routing + message buffering
- Tauri event 驱动的消息 / 日志 / agent 状态 / permission prompt 同步
- unified task/session/artifact graph + provider history / runtime resume

### 已删除或不再适用

- `daemon/**/*.ts` Bun daemon 体系
- GUI WebSocket `:4503`
- 旧 PTY / channel 注入链路（`node-pty`、旧 TS daemon、`claude_session/`）
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
