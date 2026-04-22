# Dimweave

Dimweave 是一个面向多 Agent 协作的桌面应用：它把 Claude Code、Codex 和一个统一的任务/会话/产物模型接到同一套桌面界面与 Rust daemon 上，让用户能在一个应用里完成连接、路由、历史恢复、任务切换和消息审计。

当前产品形态以 **Tauri 桌面壳 + Rust 内嵌 daemon + React 前端** 为核心，其中：

- **产品内核** 是 `src-tauri/src/daemon/`：负责 Agent 生命周期、消息路由、任务图、历史恢复和状态广播
- **Claude 主链路** 是 `--sdk-url` + MCP tools bridge
- **Codex 主链路** 是 `codex app-server` + WebSocket session
- **前端状态层** 使用 Zustand，按 task-scoped 方式消费 daemon 事件

## 核心架构

```text
React / Vite / Zustand
  └─ Tauri invoke + event listeners
      └─ Rust daemon (task/session/artifact source of truth)
          ├─ Claude CLI (--sdk-url, stream-json, MCP tools bridge)
          ├─ Codex app-server (WS / readyz / healthz)
          ├─ task_graph persistence
          └─ provider history + runtime resume

Claude MCP bridge sidecar (bridge/)
  └─ 提供 reply / get_online_agents 等 MCP tools
```

### 关键模块

| 模块 | 作用 |
|---|---|
| `src-tauri/src/daemon/` | 产品内核：路由、运行时、task graph、provider history、resume |
| `src/` | React 前端与 Zustand store |
| `bridge/` | `dimweave-bridge` sidecar，当前主要服务 Claude MCP tools |
| `src-tauri/src/mcp.rs` | `.mcp.json` 注册与 strict MCP 配置拼装 |
| `src-tauri/src/commands*.rs` | Tauri 命令面 |
| `docs/agents/` | Claude / Codex 链路记录、协议验证与历史修复 |

## 当前链路概览

### Claude

- 由 Tauri 直接启动 Claude CLI
- 通过 `--sdk-url ws://127.0.0.1:4502/claude?...` 接回 daemon
- Claude 会读取项目 `.mcp.json`，再启动 `dimweave-bridge`
- bridge 提供 MCP tools，如 `reply()`、`get_online_agents()`
- 历史恢复支持两层：
  - provider-native `session_id`
  - normalized task session

### Codex

- daemon 启动 `codex app-server`
- app-server 默认监听 `127.0.0.1:4500`
- daemon 通过 WebSocket 完成 `initialize` / `thread/start` / resume
- provider-native history 以 `thread_id` 为主

### Task / Session Memory

Dimweave 明确区分三层语义：

1. **provider history**
   - Claude `session_id`
   - Codex `thread_id`
2. **live provider connection**
   - 当前在线的 provider 运行时连接
3. **normalized task session**
   - daemon `task_graph` 中的标准化 session 记录

这三者不是同一个概念。当前系统的很多 UI 和恢复逻辑都围绕这层区分设计。

## 环境要求

建议至少具备：

- **Bun**：前端依赖安装与脚本执行
- **Rust toolchain**：构建 Tauri 主程序和 bridge sidecar
- **Tauri 构建前置**：按 Tauri 2 官方要求安装本机依赖
- **Claude CLI**
- **Codex CLI**

如果要实际体验完整功能，通常还需要：

- 可用的 Claude 登录态 / provider auth
- 可用的 Codex 登录态 / provider auth

## 快速开始

### 1. 安装依赖

```bash
bun install
```

### 2. 启动桌面开发环境

```bash
bun run tauri dev
```

这个命令会自动执行：

1. `cargo build -p dimweave-bridge`
2. `bun run dev`
3. 启动 Tauri 桌面应用

默认端口：

- Vite: `1420`
- Claude / bridge control: `4502`
- Codex app-server: `4500`

### 3. 使用备用端口启动

如果本机已有端口冲突，可以使用内置脚本：

```bash
bun run dev:alt
bun run dev:tg
```

## 打包与发布

### 前端单独构建

```bash
bun run build
```

### 单独构建 bridge sidecar

调试版：

```bash
cargo build -p dimweave-bridge
```

发布版：

```bash
cargo build -p dimweave-bridge --release
```

### 构建完整桌面应用

```bash
bun run tauri build
```

`src-tauri/tauri.conf.json` 已配置：

- `beforeBuildCommand`: `cargo build -p dimweave-bridge --release && bun run build`
- `externalBin`: `binaries/dimweave-bridge`

也就是说，**正常情况下不需要手动先构建 bridge 和前端**，`tauri build` 会自动处理。

### 打包产物位置

完整 Tauri bundle 一般会出现在：

```text
src-tauri/target/release/bundle/
```

常见平台产物包括：

- macOS: `.app`, `.dmg`
- Windows: `.msi`, `.exe`
- Linux: `.AppImage`, `.deb`（取决于本机工具链）

## 测试与验证

### Rust 主测试集

```bash
cargo test -p dimweave
```

### 前端测试

```bash
bun test
```

### 构建验证

```bash
bun run build
```

### E2E

```bash
bun run test:e2e
```

## 仓库结构

```text
.
├─ src/                 # React 前端
├─ src-tauri/           # Tauri 主程序 + Rust daemon
├─ bridge/              # dimweave-bridge MCP sidecar
├─ docs/agents/         # Claude / Codex 文档与修复记录
├─ docs/superpowers/    # 计划、执行记录、迁移文档
└─ package.json         # 前端与 tauri 脚本入口
```

## 关键文档入口

优先阅读这些文档来理解当前实现，而不是只看历史 plan：

- [CLAUDE.md](./CLAUDE.md)
- [docs/agents/claude-docs-index.md](./docs/agents/claude-docs-index.md)
- [docs/agents/claude-sdk-url-validation.md](./docs/agents/claude-sdk-url-validation.md)
- [docs/agents/claude-chain.md](./docs/agents/claude-chain.md)
- [docs/agents/codex-chain.md](./docs/agents/codex-chain.md)

## 当前实现约定

- Bun 只承担前端脚本和依赖管理，不作为后端常驻 daemon
- `src-tauri/src/daemon/` 是运行时事实源
- `task_graph` 是任务/会话/产物的标准化持久化模型
- 历史 plan 文档可用于追溯问题，但**不自动等于当前架构**

## 常见开发命令

```bash
bun run dev          # Vite dev server
bun run build        # 前端构建
bun run tauri dev    # 桌面开发模式
bun run tauri build  # 打包桌面应用
bun run bridge       # 构建 bridge sidecar
cargo test -p dimweave
bun test
```

## 备注

- 从 worktree 启动 `tauri dev` 时，如果资源走到主工作区 `node_modules`，Vite 可能会报 `server.fs.allow` 相关警告；这通常不影响主流程，但会影响字体资源访问。
- 若问题与 Claude/Codex 链路相关，优先查 `docs/agents/*.md` 和 `src-tauri/src/daemon/`，不要只看旧计划文档。
