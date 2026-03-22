# AgentBridge

通用 AI Agent 桥接 GUI 桌面应用，让多个 AI 编程助手（Claude Code、Codex、未来 Gemini 等）在同一台机器上实时双向通信。

## 技术栈

- **桌面壳**: Tauri 2 (Rust) — 窗口管理 + Codex auth/usage/models 查询
- **前端**: React 19 + Vite + TypeScript + Tailwind CSS v4 + shadcn/ui + Zustand
- **后端 daemon**: Bun + TypeScript — 消息路由 + Codex WS 代理
- **通信协议**: MCP + WebSocket + JSON-RPC 2.0

## 架构

```
┌─ Tauri Rust ──────────────────────┐
│ codex/auth.rs  → JWT 解码          │
│ codex/usage.rs → ChatGPT API 用量  │◄── invoke ──┐
│ codex/models.rs → 模型列表          │             │
│ dialog         → 目录选择器         │             │
└───────────────────────────────────┘             │
                                                   │
┌─ Bun Daemon ──────────────────────┐             │
│ daemon.ts        → 启动/事件绑定    │             │
│ control-server.ts → Claude MCP 桥  │             │
│ gui-server.ts    → GUI WS 服务     │──WS 4503──►│
│ codex-adapter.ts → WS 代理/协议拦截 │             │
│   message-handler → turn/model 捕获│             │
│   response-patcher → 响应兼容 patch │             │
└───────────────────────────────────┘             │
                                                   │
┌─ React 前端 ─────────────────────────────────────┘
│ bridge-store       → daemon WS 状态 (消息/agent/协议数据)
│ codex-account-store → Tauri invoke (profile/usage/models)
│ AgentStatus        → 侧边栏面板
│ CodexAccountPanel  → 可折叠配置面板 (model/reasoning 下拉选择)
│ MessagePanel       → 消息列表
└──────────────────────────────────────────────────
```

**数据职责分离:**
- **Tauri Rust** 负责静态/低频数据: 账号 profile (JWT)、用量配额 (ChatGPT API)、可用模型列表、目录选择
- **Bun Daemon** 负责运行时数据: 当前 model/reasoning/cwd (协议拦截)、消息路由、agent 状态
- **前端** 合并两个数据源渲染

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

**闭环要求:**
- 遇到 bug 或设计问题，修复后必须将根因和解法写入对应 rules 文件
- 每次架构变更（新增模块、数据流调整、职责迁移）必须同步更新本文件的架构图
- 每个文件最多 500 行

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

已实现:
- Tauri 壳 + Rust auth/usage/models 模块
- Daemon 消息路由 (模块化: daemon-state / gui-server / control-server)
- Codex 适配器 (模块化: adapter / message-handler / response-patcher / port-utils)
- Claude MCP 适配器 (bridge.ts)
- GUI: Agent 状态面板、可折叠 Codex 配置面板 (model/reasoning 下拉选择、CWD 目录选择、用量进度条)
- 一键连接 Codex + 配置热切换 (apply_config)

待实现: Gemini CLI 适配器、多会话支持、消息搜索、Agent 编排、设置页面

## 踩坑记录

| 问题 | 根因 | 解法 | 规则 |
|------|------|------|------|
| 下拉菜单被面板截断 | 父容器 `overflow-hidden` 裁剪了 z-50 弹出层 | 去掉父级 `overflow-hidden`，圆角在子元素上单独加 | frontend.md 层级与布局 |
| Codex 账号信息拿不到 | `patchResponse` 返回新 JSON 但 `intercept` 仍在读原始 error 对象 | patch 后重新 parse 传给 intercept | daemon.md |
| `initSession` 丢失 model 等字段 | `thread/start` 响应在 handleMessage listener 里直接 resolve，没走 `handler.intercept` | 在 resolve 前调用 `handler.intercept(msg)` | — |
| git 初始提交包含 node_modules | `.gitignore` 未被 tracked | 先 add `.gitignore` 再提交 | — |
| GUI 白屏 (Zustand 无限循环) | selector 内 `.filter()` 每次返回新引用 | selector 取原始数组，在组件内 for 循环过滤 | frontend.md 性能优化 |
| GUI 白屏 (process.cwd) | 前端用了 Node.js API `process.cwd()` | 改为空字符串，系统信息通过 Tauri invoke 获取 | frontend.md 性能优化 |
| Claude --print 不加载 MCP | `--print` 模式不读 mcp.json | 用 `--mcp-config` 传入内联 JSON | — |
| Claude --print 自动退出 | 单次执行模式 | 加 `--input-format stream-json` 保持 stdin 开着 | — |
