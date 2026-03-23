# AgentBridge

通用 AI Agent 桥接 GUI 桌面应用，让多个 AI 编程助手（Claude Code、Codex、未来 Gemini 等）在同一台机器上实时双向通信，支持角色分工和自动协作。

## 技术栈

- **桌面壳**: Tauri 2 (Rust) — 窗口管理 + Codex auth/usage/models 查询 + MCP 注册 + Claude PTY (portable-pty)
- **前端**: React 19 + Vite + TypeScript + Tailwind CSS v4 + shadcn/ui + Zustand + xterm.js
- **后端 daemon**: Bun + TypeScript — 角色管理 + 消息硬转发 + Codex WS 代理
- **通信协议**: MCP Tools + WebSocket + JSON-RPC 2.0 + Tauri Events/Commands

## 架构

```
┌─ Tauri Rust ──────────────────────────┐
│ codex/auth.rs   → JWT 解码             │
│ codex/usage.rs  → ChatGPT API 用量     │◄── invoke ──┐
│ codex/models.rs → 模型列表              │             │
│ dialog/mcp      → 目录选择/MCP注册       │             │
│ pty.rs          → Claude PTY 管理       │── event ──►│
│   (portable-pty: launch/write/resize/stop)            │
└───────────────────────────────────────┘             │
                                                       │
┌─ Bun Daemon ──────────────────────────┐             │
│ daemon.ts         → 入口/角色转发       │             │
│ role-config.ts    → 角色定义/能力矩阵    │             │
│ gui-server.ts     → GUI WS 服务        │──WS 4503──►│
│ control-server.ts → Claude MCP 桥      │             │
│ codex-adapter.ts  → WS 代理/拦截       │             │
│   message-handler → turn/model 捕获    │             │
│   response-patcher → 响应兼容 patch     │             │
│ bridge.ts         → MCP server         │             │
└───────────────────────────────────────┘             │
                                                       │
┌─ React 前端 ─────────────────────────────────────────┘
│ bridge-store        → daemon WS 状态 (消息/agent/角色)
│ codex-account-store → Tauri invoke (profile/usage/models)
│ agent-roles.ts      → Claude --agents JSON 生成
│ AgentStatus         → 侧边栏 + 角色下拉选择
│ ClaudePanel         → Claude PTY 启停 (Tauri invoke) + 配额/角色
│ CodexAccountPanel   → 可折叠配置面板 (model/reasoning)
│ MessagePanel        → Messages | Terminal (xterm.js + Tauri event) | Logs
└──────────────────────────────────────────────────────
```

## PTY 架构

Claude Code PTY 由 Tauri Rust 层直接管理（`portable-pty` crate），不经过 daemon。

```
ClaudePanel.tsx                         pty.rs (Rust)
  invoke("launch_pty", {cwd,           →  portable_pty::openpty()
    cols, rows, roleId, agentsJson})    →  spawn "claude --dangerously-skip-permissions --agent <role> --agents <json>"
                                           │
MessagePanel.tsx (xterm.js)             ←  emit("pty-data", chunk)   ← reader thread
  term.onData(keystroke)                →  invoke("pty_write", data) → writer
  term.onResize({cols,rows})            →  invoke("pty_resize")      → master.resize()
                                           │
ClaudePanel.tsx                         ←  emit("pty-exit", code)    ← child monitor thread
  invoke("stop_pty")                    →  drop writer+pair → kills child
```

**为什么用 Rust PTY 而非 Node PTY**：Node.js `node-pty` 通过 JSON 序列化传输 PTY 数据到前端，高频输出时导致终端卡死。Rust `portable-pty` 直接通过 Tauri event 传输，性能与原生终端一致。

## 角色系统

### 角色定义

角色配置分两处：
- `daemon/role-config.ts` — Codex 侧配置（developer_instructions、sandbox、approval）+ Claude agent 定义
- `src/lib/agent-roles.ts` — 前端可访问的 Claude `--agents` JSON 生成器（ClaudePanel 启动 PTY 时使用）

| 角色 | 定位 | Codex 硬限制 | Claude 硬限制 |
|------|------|-------------|-------------|
| **Lead** | 主控决策者，审核其他 Agent 输出，有最终执行权 | sandbox: workspace-write | permissionMode: bypass, 全工具 |
| **Coder** | 代码执行者，写代码交给 Lead 审核 | sandbox: workspace-write | permissionMode: bypass, 全工具 |
| **Reviewer** | 只读审核，不改文件 | sandbox: **read-only** (OS强制) | tools: Read,Grep,Glob only (**硬限制**) |
| **Tester** | 跑测试，不改文件 | sandbox: **read-only** (OS强制) | tools: Read,Grep,Glob,Bash (**硬限制**) |

### 强制性机制

| 机制 | 强制等级 | 用于 |
|------|---------|------|
| Codex `sandbox_mode: read-only` | **OS 内核强制** | Reviewer/Tester 不可写文件 |
| Claude `--disallowed-tools Write,Edit` | **客户端强制** | Reviewer/Tester 不可编辑 |
| Claude `--permission-mode plan` | **客户端强制** | Reviewer/Tester 只读模式 |
| Codex `developer_instructions` | 软引导 | 角色行为指引 |
| Claude `--agent --agents` JSON | 软引导+硬限制 | system prompt + tools 限制 |

### 数据流（双向已验证）

```
用户发任务 → GUI 发给 Codex
  ↓
Codex 执行（受角色 sandbox/developer_instructions 限制）
  ↓ turn 完成
Daemon 注入 Codex 输出到 Claude PTY（pty_inject → frontend → Rust PTY stdin）
  短消息(≤500字符): 全文注入 "Coder says: ..."
  长消息(>500字符): 截断摘要 + check_messages 指引
  ↓
Claude（Lead）审核
  ├── 合理 → 自己执行代码修改 → 通过 MCP reply tool 通知 Codex
  ├── 有疑问 → 通过 MCP reply tool 发回 Codex 讨论 → Codex 回复 → daemon 再注入 → 自动协商
  └── 不合理 → 通过 MCP reply tool 说明原因
```

### Claude 启动命令（由 Rust PTY 自动构建）

```bash
claude --dangerously-skip-permissions \
  --mcp-config ~/.claude/mcp.json \
  --agent <roleId> \
  --agents '{"<roleId>":{"description":"...","prompt":"...","tools":"...","permissionMode":"..."}}'
```

### 防无限循环

- Codex → Claude：daemon 硬转发（pty_inject），每次 turn 完成触发一次
- Claude → Codex：Claude 主动调用 MCP reply tool（非自动）
- Claude 不调用 reply tool = 流程结束

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
- `architecture.md` — 架构、端口、消息流、模块结构
- `daemon.md` — daemon 端规范（匹配 `daemon/**/*.ts`）
- `frontend.md` — 前端规范（匹配 `src/**/*.{ts,tsx}`）
- `tauri.md` — Tauri 规范（匹配 `src-tauri/**`）

**闭环要求:**
- 遇到 bug 或设计问题，修复后必须将根因和解法写入对应 rules 文件和踩坑记录
- 每次架构变更必须同步更新本文件的架构图
- 每个文件最多 500 行
- Daemon 代码修改后必须重启 daemon，Rust 代码修改后必须重启 Tauri

## 当前状态 (MVP)

已实现:
- Tauri 壳 + Rust auth/usage/models/MCP注册 模块
- **Rust PTY** (portable-pty) 管理 Claude Code 进程，Tauri event 直传前端 xterm.js
- **双向通信链路** — Codex→daemon→pty_inject→Claude PTY + Claude→MCP reply→daemon→Codex
- **MCP bridge 自动加载** — `--mcp-config` 确保 Claude 启动时加载 agentbridge tools
- **Bridge 自动重连** — daemon 重启后 bridge WS 自动重连
- Daemon 消息路由 (模块化: daemon-state / gui-server / control-server)
- Codex 适配器 (模块化: adapter / message-handler / response-patcher / port-utils)
- Codex 流式消息 (started/delta/completed + thinking/streaming 阶段指示)
- Codex 配置面板 (model/reasoning 下拉选择、CWD 目录选择、5h/7d 用量进度条)
- Claude MCP Tools (reply / check_messages / get_status)
- Claude 面板 (项目选择 → 角色选择 → Tauri invoke 一键启动 → PTY 终端 tab → 停止)
- 角色系统 (Lead/Coder/Reviewer/Tester + 硬限制 + 角色驱动硬转发)
- Claude `--agent --agents` 角色注入 (tools/permissionMode 硬限制)
- Codex `sandbox` + `developer_instructions` 角色注入 (OS 强制 + 软引导)
- 三标签消息面板 (Messages / Terminal / Logs)

待实现: Gemini CLI 适配器、多会话支持、消息搜索、Agent 编排面板、设置页面、打包分发(.dmg)

## 模块结构

### Tauri Rust
```
src-tauri/src/
├── main.rs                # Tauri commands 注册
├── pty.rs                 # Claude PTY (portable-pty: launch/write/resize/stop)
└── codex/
    ├── mod.rs
    ├── auth.rs            # JWT 解码 + profile
    ├── usage.rs           # ChatGPT API 用量 + SQLite 缓存
    └── models.rs          # 模型列表
```

### Daemon (Bun)
```
daemon/
├── daemon.ts              # 入口: 配置/事件绑定/角色转发
├── daemon-state.ts        # 共享状态 + 广播 helper
├── gui-server.ts          # GUI WS 服务 + apply_config
├── control-server.ts      # 控制 WS + Claude MCP 管理
├── role-config.ts         # 角色定义 + developer_instructions + Claude agent 配置
├── tui-connection-state.ts
├── bridge.ts              # MCP bridge server (Claude spawn)
├── types.ts               # 共享类型
├── control-protocol.ts
└── adapters/
    ├── codex-adapter.ts          # 编排: 生命周期/WS/代理
    ├── codex-message-handler.ts  # 通知解析/turn 追踪/账号捕获
    ├── codex-response-patcher.ts # 响应兼容 patch
    ├── codex-port-utils.ts       # 端口检查
    └── codex-types.ts            # 类型定义
```

### Frontend (React)
```
src/
├── lib/
│   ├── utils.ts                # cn() 工具函数
│   └── agent-roles.ts          # Claude --agents JSON 生成 (前端用)
├── stores/
│   ├── bridge-store.ts         # daemon WS 状态 (消息/agent/角色)
│   └── codex-account-store.ts  # Tauri invoke 状态 (profile/usage/models)
├── components/
│   ├── AgentStatus.tsx         # 侧边栏面板 + 角色下拉
│   ├── ClaudePanel.tsx         # Claude PTY 启停 (Tauri invoke) + 配额/角色
│   ├── CodexAccountPanel.tsx   # 可折叠配置面板 (model/reasoning)
│   ├── MessagePanel.tsx        # Messages | Terminal (xterm.js) | Logs
│   ├── ReplyInput.tsx          # 输入框
│   └── ui/                    # shadcn 组件
├── types.ts
└── App.tsx
```

## 踩坑记录

| 问题 | 根因 | 解法 | 规则 |
|------|------|------|------|
| 下拉菜单被面板截断 | 父容器 `overflow-hidden` | 去掉父级 `overflow-hidden` | frontend.md |
| Codex 账号信息拿不到 | `intercept` 读原始 error 对象 | patch 后重新 parse 传给 intercept | daemon.md |
| GUI 白屏 (Zustand) | selector 内 `.filter()` 新引用 | selector 取原始数组，组件内 for 循环 | frontend.md |
| GUI 白屏 (process.cwd) | 前端用了 Node.js API | 禁止前端 Node API | frontend.md |
| xterm.js 黑屏 | PTY 数据在 xterm 前到达 | 缓冲 PTY 数据，open 后回放 | frontend.md |
| Codex sandbox 参数错 | 发 `{type:"read-only"}` | 直接传字符串 `"read-only"` | — |
| `@claude` 协议不可靠 | LLM 不遵守 prompt 指令 | 改为 daemon 硬转发，不依赖 LLM | daemon.md |
| developer_instructions 不可靠 | prompt 级别，模型可能忽略 | 用硬限制 (sandbox/tools/permissions) 控制能力 | role-config.ts |
| 内嵌终端卡死 | node-pty JSON 序列化开销 | 迁移到 Rust portable-pty + Tauri event 直传 | tauri.md |
| pty_inject `[` 被吃 | `\x1b[` 构成 ANSI CSI 转义序列 | 去掉 `\x1b` 前缀和 `[` 括号 | daemon.md |
| Claude 不用 reply tool | MCP bridge 未加载 | Rust PTY 加 `--mcp-config` 参数 | pty.rs |
| Bridge 断连不重连 | daemon-client.ts 无重连逻辑 | `onclose` 触发 `tryReconnect` 自动重连 | daemon.md |
| Claude 忽略 reply 指令 | 系统 prompt 软引导不够强 | 注入消息末尾附加 reply tool 使用提醒 | daemon.ts |
