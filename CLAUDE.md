# AgentBridge

通用 AI Agent 桥接 GUI 桌面应用，让多个 AI 编程助手（Claude Code、Codex、未来 Gemini 等）在同一台机器上实时双向通信，支持角色分工和自动协作。

## 技术栈

- **桌面壳**: Tauri 2 (Rust) — 窗口管理 + Codex auth/usage/models 查询 + MCP 注册
- **前端**: React 19 + Vite + TypeScript + Tailwind CSS v4 + shadcn/ui + Zustand + xterm.js
- **后端 daemon**: Bun + TypeScript — 角色管理 + 消息硬转发 + Codex WS 代理 + Claude PTY
- **通信协议**: MCP Tools + WebSocket + JSON-RPC 2.0

## 架构

```
┌─ Tauri Rust ──────────────────────┐
│ codex/auth.rs   → JWT 解码         │
│ codex/usage.rs  → ChatGPT API 用量 │◄── invoke ──┐
│ codex/models.rs → 模型列表          │             │
│ dialog/mcp      → 目录选择/MCP注册   │             │
└───────────────────────────────────┘             │
                                                   │
┌─ Bun Daemon ──────────────────────┐             │
│ daemon.ts         → 入口/角色转发   │             │
│ role-config.ts    → 角色定义/能力矩阵│             │
│ gui-server.ts     → GUI WS 服务    │──WS 4503──►│
│ control-server.ts → Claude MCP 桥  │             │
│ claude-pty.ts     → PTY 管理器      │             │
│   claude-pty-helper.cjs → Node PTY │             │
│ codex-adapter.ts  → WS 代理/拦截   │             │
│   message-handler → turn/model 捕获│             │
│   response-patcher → 响应兼容 patch │             │
│ bridge.ts         → MCP server     │             │
└───────────────────────────────────┘             │
                                                   │
┌─ React 前端 ─────────────────────────────────────┘
│ bridge-store        → daemon WS 状态 (消息/agent/PTY/角色)
│ codex-account-store → Tauri invoke (profile/usage/models)
│ AgentStatus         → 侧边栏 + 角色下拉选择
│ ClaudePanel         → Claude 连接/配额/角色/停止
│ CodexAccountPanel   → 可折叠配置面板 (model/reasoning)
│ MessagePanel        → Messages | Terminal (xterm.js) | Logs
└──────────────────────────────────────────────────
```

## 角色系统

### 角色定义 (role-config.ts)

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

### 数据流

```
用户发任务 → GUI 发给 Codex
  ↓
Codex 执行（受角色 sandbox/developer_instructions 限制）
  ↓ turn 完成
Daemon 硬转发给 Claude PTY（代码逻辑，不靠 LLM，100%可靠）
  ↓
Claude（Lead）审核
  ├── 合理 → 自己执行代码修改
  ├── 有疑问 → 通过 reply tool 发回 Codex 讨论 → 自动协商
  └── 不合理 → 按自己判断处理
```

### Claude 启动命令

```bash
claude --dangerously-skip-permissions \
  --agent <roleId> \
  --agents '{"<roleId>":{"description":"...","prompt":"...","tools":"...","permissionMode":"..."}}'
```

### 防无限循环

- Codex → Claude：daemon 硬转发，每次 turn 完成触发一次
- Claude → Codex：不自动发生，Claude 在 PTY 中由用户控制
- Claude 不回复 = 流程结束

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
- Daemon 消息路由 (模块化: daemon-state / gui-server / control-server)
- Codex 适配器 (模块化: adapter / message-handler / response-patcher / port-utils)
- Codex 流式消息 (started/delta/completed + thinking/streaming 阶段指示)
- Codex 配置面板 (model/reasoning 下拉选择、CWD 目录选择、5h/7d 用量进度条)
- Claude MCP 纯 Tools 方案 (reply / check_messages / get_status)
- Claude PTY 真实终端 (node-pty via Node helper + xterm.js 渲染)
- Claude 面板 (项目选择 → 角色选择 → 一键启动 → PTY 终端 tab → 停止)
- 角色系统 (Lead/Coder/Reviewer/Tester + 硬限制 + 角色驱动硬转发)
- Claude `--agent --agents` 角色注入 (tools/permissionMode 硬限制)
- Codex `sandbox` + `developer_instructions` 角色注入 (OS 强制 + 软引导)
- 三标签消息面板 (Messages / Terminal / Logs)

待实现: Gemini CLI 适配器、多会话支持、消息搜索、Agent 编排面板、设置页面、打包分发(.dmg)

## 踩坑记录

| 问题 | 根因 | 解法 | 规则 |
|------|------|------|------|
| 下拉菜单被面板截断 | 父容器 `overflow-hidden` | 去掉父级 `overflow-hidden` | frontend.md |
| Codex 账号信息拿不到 | `intercept` 读原始 error 对象 | patch 后重新 parse 传给 intercept | daemon.md |
| GUI 白屏 (Zustand) | selector 内 `.filter()` 新引用 | selector 取原始数组，组件内 for 循环 | frontend.md |
| GUI 白屏 (process.cwd) | 前端用了 Node.js API | 禁止前端 Node API | frontend.md |
| node-pty Bun 不兼容 | native addon 不支持 | Node.js 子进程 PTY helper | daemon.md |
| xterm.js 黑屏 | PTY 数据在 xterm 前到达 | 缓冲 PTY 数据，open 后回放 | frontend.md |
| Codex sandbox 参数错 | 发 `{type:"read-only"}` | 直接传字符串 `"read-only"` | — |
| `@claude` 协议不可靠 | LLM 不遵守 prompt 指令 | 改为 daemon 硬转发，不依赖 LLM | daemon.md |
| developer_instructions 不可靠 | prompt 级别，模型可能忽略 | 用硬限制 (sandbox/tools/permissions) 控制能力 | role-config.ts |
