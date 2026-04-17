# Claude 链路修复记录

> **文档状态：** `Legacy`
>
> **说明：** 这是 Claude 链路的修复总账，保留了大量 PTY/channel 时代记录，也包含后续向 `--sdk-url` 迁移的节点。它适合查历史问题，不适合单独充当“当前架构说明”。当前方案请先看 [claude-docs-index.md](/Users/jason/floder/agent-bridge/docs/agents/claude-docs-index.md)、[claude-sdk-url-validation.md](/Users/jason/floder/agent-bridge/docs/agents/claude-sdk-url-validation.md) 和 [CLAUDE.md](/Users/jason/floder/agent-bridge/CLAUDE.md)。

> **强制规则:** 每次修复或发现 Claude 链路问题，必须在此文档记录。
> 包括：问题描述、根因、修复方案、运行时验证结果。
> 错误的修复尝试也必须记录。

## 官方文档参考

- Channel API 完整文档: `docs/agents/claude-channel-api.md`
- 在线: https://docs.anthropic.com/en/docs/claude-code

## Channel 启动参数

### CLI 启动方式

```bash
claude --dangerously-load-development-channels server:<mcp_server_name> --dangerously-skip-permissions
```

- `server:agentnexus` — 加载 `.mcp.json` 中名为 `agentnexus` 的 MCP server 作为 channel
- `plugin:<name>@<marketplace>` — 加载插件形式的 channel
- 此 flag 绕过 allowlist，仅限开发测试使用
- `--dangerously-skip-permissions` — 默认跳过 Claude CLI 的本地 permission 确认，当前 Dimweave 以 bypass permission 作为默认启动方式
- 需要 Claude Code >= 2.1.80
- 需要 claude.ai 登录（不支持 Console/API key）

### Server 构造函数参数

MCP `Server` 构造函数接受 `(serverInfo, options)`：

| 参数 | 类型 | 必填 | 作用 |
|------|------|------|------|
| `serverInfo.name` | `string` | 是 | Server 名称，对应 `.mcp.json` 的 key 和 `<channel source="...">` 的 `source` 属性 |
| `serverInfo.version` | `string` | 是 | Server 版本号 |
| `options.capabilities.experimental['claude/channel']` | `{}` | **是** | 声明这是一个 channel。必须为空对象 `{}`。缺少此项则不是 channel |
| `options.capabilities.experimental['claude/channel/permission']` | `{}` | 否 | 声明可以接收 permission relay 请求（远程审批）。需 >= 2.1.81 |
| `options.capabilities.tools` | `{}` | 否 | 声明提供 tools（双向 channel 需要）。空对象 `{}` 即可，具体 tool 通过 handler 注册 |
| `options.instructions` | `string` | 推荐 | 注入到 Claude system prompt。告诉 Claude 事件格式、是否需要回复、用哪个 tool 回复 |

### Channel Notification 参数

发送事件: `mcp.notification({ method, params })`

| 参数 | 类型 | 必填 | 作用 |
|------|------|------|------|
| `method` | `"notifications/claude/channel"` | 是 | 固定值，channel 事件通知 |
| `params.content` | `string` | 是 | 事件正文，成为 `<channel>` 标签的 body |
| `params.meta` | `Record<string, string>` | 否 | 每个 key 成为 `<channel>` 标签属性。key 只允许字母/数字/下划线，含连字符的 key 会被静默丢弃 |

发送到 Claude 后的格式:
```xml
<channel source="agentnexus" chat_id="123" from="user">
消息内容
</channel>
```

### Reply Tool 参数

tool 通过 `ListToolsRequestSchema` handler 注册:

| 参数 | 类型 | 必填 | 作用 |
|------|------|------|------|
| `name` | `string` | 是 | Tool 名称，如 `"reply"` |
| `description` | `string` | 是 | Tool 描述，Claude 用来决定何时调用 |
| `inputSchema` | JSON Schema | 是 | 输入参数 schema。当前 bridge 用 `to` + `text` + `status` |

tool 调用通过 `CallToolRequestSchema` handler 处理，返回格式:
```json
{ "content": [{ "type": "text", "text": "sent" }] }
```

### Permission Relay 参数

#### Permission Request（Claude Code → Channel）

通知方法: `notifications/claude/channel/permission_request`

| 字段 | 类型 | 作用 |
|------|------|------|
| `request_id` | `string` | 5 个小写字母（a-z 不含 l），唯一请求标识 |
| `tool_name` | `string` | 要执行的工具名，如 `"Bash"`、`"Write"` |
| `description` | `string` | 人类可读的操作描述 |
| `input_preview` | `string` | 工具参数 JSON 预览，截断到约 200 字符 |

#### Permission Verdict（Channel → Claude Code）

通知方法: `notifications/claude/channel/permission`

| 字段 | 类型 | 值 | 作用 |
|------|------|---|------|
| `request_id` | `string` | 必须回传原 request 的 ID | 匹配挂起的请求 |
| `behavior` | `string` | `"allow"` 或 `"deny"` | 允许或拒绝工具调用 |

### .mcp.json 注册格式

```json
{
  "mcpServers": {
    "agentnexus": {
      "command": "/absolute/path/to/dimweave-bridge",
      "args": []
    }
  }
}
```

- `command` 当前有意使用绝对路径（Tauri 打包要求）
- Claude Code 在启动时读取并 spawn 每个 server 为子进程
- stdio 通信（newline-delimited JSON-RPC 2.0，省略 `"jsonrpc":"2.0"` header）

## 当前实现与 API 的对照

| API 功能 | bridge 实现 | 状态 |
|----------|-------------|------|
| `claude/channel` capability | `mcp_protocol.rs` initialize result | ✅ 已实现 |
| `claude/channel/permission` capability | `mcp_protocol.rs` initialize result | ✅ 已实现 |
| `instructions` | `mcp_protocol.rs` initialize result | ✅ 已实现 |
| `tools` capability + `reply` tool | `tools.rs` + `mcp.rs` ListTools handler | ✅ 已实现 |
| `notifications/claude/channel` | `channel_state.rs` prepare_channel_message | ✅ 已实现 |
| `notifications/claude/channel/permission_request` | `mcp.rs` parse + bridge outbound | ✅ 已实现 |
| `notifications/claude/channel/permission` | `channel_state.rs` permission_notification | ✅ 已实现 |
| meta 属性 (`from`, 可选 `status`) | `channel_state.rs` prepare_channel_message | ✅ 已实现 |
| Sender gating | `channel_state.rs` ALLOWED_SENDERS（当前为 `user/system/lead/coder/reviewer`） | ✅ 已实现 |
| Pre-init message buffering | `mcp.rs` pre_init_buffer | ✅ 已实现 |

## 最新修复记录

### 2026-04-01: Claude session memory / transcript history / runtime resume

#### [已修复] Claude 历史会话之前没有进入统一 task workspace

**问题:** Claude managed PTY 已经能跑通 channel，但 session metadata、workspace transcript history、runtime resume 之前没有接进统一 task/session 模型，用户也无法在 task workspace 里选择历史会话。

**根因:** Claude 启动链缺少显式 session id、transcript path 注册和 workspace transcript index；前端只有最小 task shell，没有 history picker / session tree / artifact timeline。

**修复:**
- managed launch 现在显式分配 `--session-id <uuid>`
- 恢复历史会话时走 `--resume <session_id>`
- `provider/claude.rs` 新增：
  - transcript path 推导
  - workspace transcript index（`$HOME/.claude/projects/<workspace-slug>/*.jsonl`）
  - `build_resume_target()`
  - `register_on_launch()` / `register_on_connect()` metadata capture
- `launch_claude_terminal` 在 daemon 中注册 `external_id + transcript_path`
- `ResumeSession` / `AttachProviderHistory` 对 Claude provider 走真实 runtime resume
- 前端新增 session tree、history picker、artifact timeline、review gate badge，以及 ReplyInput / MessagePanel / AgentStatus 的 task context 摘要

**验证:**
- `cargo test --manifest-path src-tauri/Cargo.toml provider`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun test tests/task-store.test.ts tests/task-panel-view-model.test.ts`
- `bun run build`
- 运行中的 `dimweave` daemon 继续监听 `127.0.0.1:4502`

**已知限制:**
- 原生 Tauri 窗口中的“完整人工点击恢复 Claude 历史会话”仍需在本机 GUI 上做最终人工回放；当前自动化环境只能完成启动 smoke 与后端/前端验证

## 修复记录

### 2026-03-25: 初始审计

- [已修复] bridge pre-init 消息丢失 — 添加本地缓冲 + 回放
- [已修复] stdout 写失败静默丢消息 — 写失败时 break MCP 循环
- [已修复] push_tx 死通道检测 — send 失败时退出
- [已修复] 重连反压级联 — 退避期间 drain reply_rx
- [已修复] shell 注入风险 — 非 macOS 用 Command::new

### 2026-03-25: 深度审查

- [已修复] pre-init buffer replay break 不传播到外层循环
- [已修复] Claude 启动改为静默后台进程（dev 模式弹终端，release 静默）

### 2026-03-25: Claude 断开交互与进程所有权

- [已修复] `Disconnect Claude` 之前没有 loading，用户点击后缺少可见反馈；现在前端会进入 `Disconnecting...`，直到控制链路把状态回落为 disconnected 或 stop 直接报错
- [已修复] Claude 关闭链路之前依赖模糊 `pkill`，容易把关闭问题误判成 channel API 问题；当前实现改为宿主进程记录 Claude PID，并在断开时优先精确终止 tracked session
- [已修复] Claude 关闭成功命中 tracked PID 后不再继续扫射其他相关进程；`Disconnect Claude` 的职责收敛为“关闭当前 Claude 终端会话”，不负责顺带回收 app / bridge
- [记录] Claude channel 官方 contract 只定义 channel event、reply tool、permission relay；“关闭 Claude”不是 channel RPC，而是本地宿主进程生命周期管理

### 2026-03-25: Claude hidden PTY 与开发确认自动选择

- [已修复] Claude 启动不再依赖 macOS Terminal 可见窗口；当前统一由 Tauri 进程托管 hidden PTY，会话不再要求用户盯着终端确认
- [已修复] research preview 下 `--dangerously-load-development-channels server:agentnexus` 的本地开发确认提示，现在由 PTY watcher 自动输入 `1`
- [已修复] Claude 默认启动参数现在显式附带 `--dangerously-skip-permissions`，不再只依赖运行时交互去放行本地 permission；当前默认行为就是 bypass permission
- [约束] 自动确认只对 `Channels: server:agentnexus` 生效，不会对其他 development channel 做泛化放行
- [已修复] app 退出时会顺带停止当前 Claude PTY 会话，避免隐藏 Claude 进程在 GUI 关闭后残留
- [已修复] dev 模式下 Claude PTY 输出会实时转发到 GUI 的 `Logs` 标签，便于观察启动、确认提示和异常输出；release 仍保持隐藏
- [已修复] dev 模式下 `Connect Claude` 会先弹应用内确认框，而不是把 Claude CLI 的开发确认直接暴露给用户；支持按项目记住选择，用户确认后后台 PTY 再自动续跑
- [已修复] Claude CLI 的确认 prompt 在 PTY 输出里有时会丢空格，导致自动确认失效；matcher 现在同时支持正常文案和空格塌缩后的输出
- [已修复] `Connect Claude` 每次都会重写项目根目录 `.mcp.json`，在 `vite dev` 下会触发前端整页刷新；当前改为幂等写入，配置无变化时不再落盘
- [已修复] Claude PTY 现在会把原始终端数据直接送进 GUI，前端新增嵌入式 terminal 面板并支持键盘输入与 resize；不再只能依赖日志查看启动过程
- [已修复] Claude 终端不再强制抢焦点切页；现在会在消息区弹出 `Claude Terminal` 标签并带活动提示，由用户自己决定是否切过去
- [已修复] `Claude Terminal` 切换到其他 tab 再切回时之前会空白；根因是组件卸载后重建时终端初始化与重放时序错开，当前改为在挂载时先完成 xterm 初始化，再重放已有 PTY 数据
- [已修复] Claude 终端渲染之前使用了非等宽字体优先级，字符宽度容易偏；当前改为等宽字体优先，并补上 xterm 的 Unicode 11 和 WebGL 渲染增强，向 VS Code 终端方案靠拢
- [已修复] 引入 `Unicode11Addon` 后终端黑屏并报 `You must set the allowProposedApi option to true to use proposed API`；根因是 xterm 需要在 terminal options 中显式开启 `allowProposedApi`，当前已抽成独立配置并加回归测试锁住
- [已修复] `claude_terminal_attention` 事件之前无法稳定把 GUI 切到终端 tab；根因是 prompt 检测把最近 500 字符里只要还含有 `server:agentnexus` 和 `local development` 就整段排除，导致开发确认 prompt 后紧接着出现的真实交互 prompt 也被一起误判为“无需 attention”；当前改为只分析最后一个非空 prompt block，并补了回归测试
- [已修复] Claude PTY watcher 曾在含 box-drawing 字符的终端输出上 panic，原因是 attention 检测直接按字节截取 `最近 500`，会切进多字节 Unicode 字符中间；当前改为按字符边界安全截断，并补了 Unicode 回归测试
- [已修复] 在 Claude development prompt 里手动选择 `Exit` 之前会让嵌入式终端停在最后一帧，看起来像“卡死”；根因是 PTY reader 读到 EOF 后只会结束线程，没有任何“会话退出”事件回传给 GUI。当前新增了 Claude PTY exit watcher：会在进程结束时清理 tracked session、发出 `claude_terminal_status(false)`、向终端追加退出说明，并把 Claude 面板重新切回可重连状态
- [已修复] Claude 面板之前只根据 channel bridge 的 `connected` 状态判断 UI，导致“PTY 还在启动但 bridge 尚未连上”与“PTY 已退出”都会落在同一个 `disconnected` 外观；当前前端单独维护 `claudeTerminalRunning` 状态，启动中会禁用重复 `Connect`，退出后会显示终端结束说明

### 2026-03-26: Claude 嵌入式终端渲染修复

#### 问题1: 终端内容无法滚动（钉死在屏幕）

- **根因**: 前端给 `.xterm-viewport` 加了 Tailwind 类 `[&_.xterm-viewport]:overflow-y-auto`，覆盖了 xterm.js 内部的 `overflow-y: scroll`，导致 xterm 滚动机制失效
- **修复**: 删除该覆盖 class，让 xterm 使用默认滚动行为
- **文件**: `src/components/MessagePanel/ClaudeTerminalPane.tsx`
- **验证**: ✅ 终端内容超过可视区后可正常上下滚动

#### 问题2: 终端失焦后无光标显示

- **根因**: xterm.js 默认在失焦时隐藏光标，未配置 `cursorInactiveStyle`
- **修复**: 配置 `cursorStyle: "block"`, `cursorInactiveStyle: "outline"`，失焦时显示外框光标
- **文件**: `src/components/MessagePanel/claude-terminal-config.ts`
- **验证**: ✅ 失焦时光标以橙色 outline 显示，不再消失

#### 问题3: Slash 命令 picker 上下选中无高亮样式

- **根因**: PTY 启动时未设置 `TERM` 环境变量，React Ink 降级为不支持颜色的模式，ANSI 颜色码被丢弃
- **修复**: spawn PTY 时设置 `TERM=xterm-256color` 和 `COLORTERM=truecolor`；同时补全 xterm bright 颜色主题（brightRed/Green/Yellow/Blue/Magenta/Cyan/White）及 selection 颜色
- **文件**: `src-tauri/src/claude_session/process.rs`, `src/components/MessagePanel/claude-terminal-config.ts`
- **验证**: ✅ Slash 命令选中项有反色高亮显示

#### 问题4: Picker 刷新时旧内容残留（ghost content）

- **根因**: xterm.js 使用 WebGL renderer 时，ANSI cursor-up + erase-to-EOL 序列（React Ink 的 TUI 更新机制）在 WKWebView 的 WebGL 上下文中无法正确重绘，导致旧内容不被擦除
- **尝试（失败）**: 调整 PTY 列数使其与前端实际宽度一致 → 无效，ghost content 问题与列数无关
- **修复**: 移除 `WebglAddon`，回退到 xterm.js 默认 Canvas 渲染器。WKWebView 对 WebGL 的支持不完整，导致部分绘制调用被忽略
- **文件**: `src/components/MessagePanel/ClaudeTerminalPane.tsx`
- **注意**: 移除 WebglAddon 后同步清除了 `WebglAddon` import 和 `webglAddon?.dispose()` cleanup 引用（TypeScript 编译错误）
- **验证**: ✅ Picker 上下选择时旧行被正确擦除，无 ghost content 残留

#### PTY 尺寸透传（2026-03-26 同步修复）

- **问题**: PTY 默认 cols=220/rows=50，与实际终端面板尺寸不符，导致行折叠和光标定位偏差
- **修复**: 前端在 `launch_claude_terminal` 调用前，根据 `window.innerWidth/Height` 估算实际终端尺寸（cols/rows），透传到 Rust `openpty`
- **公式**: `cols = max(80, floor((innerWidth - 296) / 7.8))`, `rows = max(24, floor((innerHeight - 140) / 15))`
- **文件**: `src/components/ClaudePanel/index.tsx`, `src-tauri/src/mcp.rs`, `src-tauri/src/claude_launch.rs`, `src-tauri/src/claude_session/mod.rs`, `src-tauri/src/claude_session/process.rs`
- **验证**: ✅ 启动时 PTY 尺寸与可视区匹配

### 2026-03-26: Superpowers Code Review 修复

#### [已修复] `claude_terminal_status` 错误驱动 `claudeNeedsAttention`

- **根因**: `helpers.ts` 在 `claude_terminal_status(running=true)` 时强制设置 `claudeNeedsAttention: true`，导致每次 PTY 启动都会触发 tab 自动切换，与"不再强制抢焦点"的已记录修复相矛盾
- **修复**: 从 `claude_terminal_status` handler 中移除 `claudeNeedsAttention` 写入；`claudeNeedsAttention` 的唯一驱动来源改为 `claude_terminal_attention` 事件（已有去抖处理）
- **文件**: `src/stores/bridge-store/helpers.ts:101`
- **验证**: ✅ Connect Claude 后终端 tab 不再自动切换；只有真实交互 prompt 出现时才触发 attention

#### [已修复] Claude terminal attention 改为 prompt 边沿触发

- **现象**: 之前 `attention_fired` 在首次 attention 后永久为 true，同一 PTY session 内后续 prompt 不再触发 tab badge，也不会再次递增 `claudeFocusNonce`

### 2026-03-27: Claude Code 2.1.85 PTY 崩溃保护

- [已修复] Claude Code `2.1.85` 在 Dimweave 的 managed PTY 下会崩在上游 TUI bundle，报错形态为 `_4.useRef is not a function`，随后终端无响应但 GUI 仍可能残留“已连接”错觉
- **根因**: 这不是 Dimweave 的 `status` 协议或 PTY 写入链直接打坏，而是 Claude Code `2.1.85` 自身交互 TUI 的回归；错误栈位于 `claude-standalone` 内部 tool activity 渲染代码
- **修复**: `ensure_claude_channel_ready()` 现在把 `2.1.85` 视为已知坏版本并在启动前直接拒绝，给出明确降级提示，不再让坏版本进入 managed PTY 后静默炸掉
- **当前策略**:
  - `>= 2.1.80` 仍是 channel preview 最低版本要求
  - `2.1.85` 被额外列为 Dimweave managed PTY 黑名单版本
  - 推荐回退到 `2.1.84`：`claude install 2.1.84 --force`
  - 或 `npm i -g @anthropic-ai/claude-code@2.1.84`
- **验证**:
  - 本地 `claude --version` = `2.1.85`
  - npm registry 仍可获取 `2.1.84`
  - Rust 单测新增：`reject_known_bad_managed_pty_version`、`accept_supported_non_bad_version`
- **根因**: `prompt.rs` 使用会话级一次性门闩去抑制 attention storm，结果把“同一 prompt 不要连发”误实现成了“整个 session 只发一次”
- **修复**: `prompt.rs` 现在改成 prompt 可见性的边沿触发。当前 prompt 仍然可见时不重复 emit；prompt 消失后再次出现时，会重新 emit `claude_terminal_attention`
- **结果**: 后续真实交互 prompt 会再次自动切到 `Claude Terminal` 并 force focus，方向键等键盘输入不再要求用户先手动点击终端

### 2026-03-26: Bridge 未被 Claude 启动 ("1 MCP server failed")

#### 问题描述

Claude PTY 启动后出现 `"Listening for channel messages from: server:agentnexus"` 紧接 `"1 MCP server failed · /mcp"`，bridge 进程从未被 spawn。

#### 根因

`--dangerously-load-development-channels server:agentnexus` 只告知 Claude 要加载名为 `server:agentnexus` 的 channel，但 Claude Code 不知道如何 spawn 对应的 MCP server。没有 `--mcp-config`，Claude 不读取项目 `.mcp.json`，因此无法找到 `dimweave-bridge` 的命令路径。

#### 失败尝试

- `--setting-sources user,project,local` — 合法的 flag，但不影响项目 `.mcp.json` 的读取逻辑，无效

#### 修复

在 `build_claude_command`（`src-tauri/src/claude_session/process.rs`）中追加：

```rust
let mcp_config_path = std::path::Path::new(dir).join(".mcp.json");
cmd.arg("--mcp-config");
cmd.arg(mcp_config_path.to_string_lossy().to_string());
```

### 2026-03-31: Claude CLI 深度逆向（prompt / channel / stream / teammate）

- [记录] 本机 Claude CLI 样本：
  - 二进制：`/Users/jason/.nvm/versions/node/v24.14.0/bin/claude`
  - 包目录：`/Users/jason/.nvm/versions/node/v24.14.0/lib/node_modules/@anthropic-ai/claude-code`
  - 版本：`2.1.88`
- [记录] 安装包内存在 `cli.js.map`，可直接从 source map 反推出原始 TS/TSX 源文件
- [确认] Claude CLI 不仅支持 `--append-system-prompt`，还支持 **更强的** `--system-prompt`
- [确认] `../src/utils/systemPrompt.ts` 的 `buildEffectiveSystemPrompt()` 中：
  - `customSystemPrompt`（即 `--system-prompt`）优先级高于 default prompt
  - `appendSystemPrompt`（即 `--append-system-prompt`）只在末尾追加
  - 因此 `--system-prompt` 是“替换主 prompt”，`--append-system-prompt` 是“尾部附言”
- [确认] Claude CLI 内建但在常规文档里不显眼的参数至少包括：
  - `--channels`
  - `--dangerously-load-development-channels`
  - `--agent-id`
  - `--agent-name`
  - `--team-name`
  - `--parent-session-id`
  - `--teammate-mode`
  - `--agent-type`
  - `--sdk-url`
- [确认] `main.tsx` 注释明确写了：
  - `--channels` 可在 interactive 和 print/SDK 模式共用
  - `--sdk-url` 会自动切到 `stream-json`
  - 说明 **stream 和 channel 本来就是可共存设计**
- [确认] 最小运行探测已验证这些参数会被当前 `2.1.88` 正常解析，不是 dead code：
  - `claude --system-prompt-file /definitely/missing -p hi`
  - `claude --append-system-prompt-file /definitely/missing -p hi`
  - `claude --sdk-url ws://127.0.0.1:1 --system-prompt-file /definitely/missing -p hi`
  - `claude --channels server:test --system-prompt-file /definitely/missing -p hi`
  - `claude --parent-session-id abc --agent-id a --agent-name b --team-name c --teammate-mode auto --agent-type worker --system-prompt-file /definitely/missing -p hi`
- [结论] Dimweave 当前 Claude 链路只使用了较弱的 `--append-system-prompt`。若要把 Claude 侧强约束提升到接近 Codex `base_instructions` 的级别，应优先改为：
  - 主角色 contract → `--system-prompt`
  - 临时附加规则 / 调试尾注 → `--append-system-prompt`
- [详情] 逆向链路与证据已单独记录到：`docs/agents/claude-cli-reverse-engineering.md`

### 2026-03-31: Claude prompt 升级为双层注入

- [已修复] Claude launcher 不再只注入 `--append-system-prompt`
- [已修复] 当前启动参数改为双层：
  - `--system-prompt`：承载主角色 contract（路由、reply 协议、必须回传）
  - `--append-system-prompt`：承载轻量 addendum（handoff/结果格式补充）
- [已修复] Claude prompt 文字约束进一步收紧：
  - 必须在需要可见结果的 turn 结束前调用 `reply()`
  - 非 lead 完成任务时默认必须回 lead
  - 没有既有 chat thread 不是静默丢结果的理由
  - worker 未通过 `reply()` 交付结果，不算完成
- [文件]:
  - `src-tauri/src/claude_launch.rs`
  - `src-tauri/src/daemon/role_config/claude_prompt.rs`
  - `src-tauri/src/daemon/role_config/mod.rs`
- [验证]:
  - `cargo test claude_launch --manifest-path src-tauri/Cargo.toml`
  - `cargo test prompt_mentions_reply_status_contract --manifest-path src-tauri/Cargo.toml`

- **文件**: `src-tauri/src/claude_session/process.rs:74-76`
- **验证**: ✅ Bridge PID 被成功 spawn；`lsof` 确认 `localhost:PORT->localhost:4502 ESTABLISHED`

#### 诊断陷阱

- `pgrep -la dimweave-bridge` 会匹配 `cargo build -p dimweave-bridge`（构建脚本），误报 bridge 存在
- 正确方法: `pgrep -fl "target/debug/dimweave-bridge"` 匹配完整二进制路径
- 二进制替换为 shell wrapper 无效：`register_mcp` 每次都重写 `.mcp.json` 为 Tauri 注册的绝对路径，wrapper 不会被调用；且 wrapper 重定向 stdout 到日志会破坏 MCP stdio 协议

### 2026-03-26: Bridge → Claude Channel 通知端到端验证

#### 路径描述

```
测试 WS 客户端 → daemon :4502 → routing.rs → bridge tx → bridge mcp_io.rs → Claude MCP stdin
```

#### 正确 Wire Format（重要）

`FromAgent` 枚举用 `#[serde(tag = "type", rename_all = "snake_case")]`：

```json
{ "type": "agent_connect", "agentId": "codex" }
{ "type": "agent_reply", "message": { "id": "...", "from": "coder", "to": "lead", "content": "...", "timestamp": 1234567890 } }
```

注意：`agentId` 是 camelCase（显式 `#[serde(rename = "agentId")]`），不是 snake_case 的 `agent_id`。`agent_connect` 解析失败时连接不会断开（handler `continue`），但 `agent_reply` 不需要前置 `agent_connect` 也能路由。

#### 验证结果

1. 测试脚本连接为 `codex`，向 `lead`（Claude role）发送测试消息
2. daemon `routing.rs` 匹配 `msg.from == codex_role` → 通过 sender gate
3. bridge 的 `channel_state.prepare_channel_message` 生成 `notifications/claude/channel` 通知
4. Claude 接收到通知，用 `reply` tool 回复：`"Hello coder! Bridge channel test received successfully. Everything is working."`
5. 回复经 bridge → daemon → buffered（codex 未连），连接时 flush 回测试客户端

- **验证**: ✅ Claude 成功收到并回复 channel 消息，end-to-end 路径完整

#### 注意

- `ALLOWED_SENDERS` = `["user", "system", "lead", "coder", "reviewer"]`，bridge 会拒绝 `"claude"` 以外不在列表内的 sender（`"intruder"` 等）
- 只有 `from` 在 `ALLOWED_SENDERS` 且 `to == claude_role` 时 channel 通知才会发出
- `codex_role` 默认为 `"coder"`，`claude_role` 默认为 `"lead"`

### 2026-03-26: Bridge 角色身份注入

#### 问题描述

`channel_instructions()` 是静态字符串，bridge 不知道 Claude 被赋予的角色（lead/coder/reviewer 等）。MCP `instructions` 中缺少 "Your role: ..." 声明，Claude 不知道自己在多 agent 系统中的身份。

#### 修复

1. `mcp.rs` 在 `.mcp.json` 中写入 `"env": { "AGENTBRIDGE_ROLE": "lead" }`
2. bridge `main.rs` 读取 `AGENTBRIDGE_ROLE` 环境变量
3. `mcp_protocol.rs` 的 `initialize_result(role)` 在 instructions 末尾追加 `"Your role: {role}"`
4. `channel_instructions` 从函数改为 `CHANNEL_INSTRUCTIONS` 常量，运行时通过 `format!` 拼接角色

**文件:** `mcp.rs`(Tauri), `main.rs`(bridge), `mcp.rs`(bridge), `mcp_protocol.rs`

**限制:** `register_mcp` 当前硬编码 role="lead"。前端切角色后需重新 register + 重启 Claude 才能更新 bridge env。

**验证:** ✅ bridge 启动日志显示 `role=lead`

### 2026-03-26: Claude 角色注入方案研究

#### 研究结论

Claude Code CLI 支持多种注入机制，按强制性排序：

| 机制 | 强制性 | 用途 |
|------|--------|------|
| `--tools` / `--disallowedTools` | L1 硬约束 | 物理移除工具，reviewer 可限定只有 Read/Grep/Glob |
| `--agents '{"role":{...}}'` + `--agent role` | L1+L4 | 自定义 subagent：限定 tools + permissionMode + system prompt |
| `permissionMode: "plan"` | L3 硬约束 | read-only 探索模式 |
| `--append-system-prompt` | L4 软约束 | 追加到 system prompt 末尾，高遵从度 |
| MCP `instructions` | L5 软约束 | 当前唯一 Claude 侧注入点，行为指引但不能限制工具 |
| CLAUDE.md | L6 最弱 | 项目级上下文 |

**当前实现:** 仅用 MCP `instructions`（L5）+ routing gating（L2）。
**可加强:** 通过 `--agents` JSON + `--agent` 注入角色定义（含 tools 白名单），当前未实现。
**产品定位:** 自动化执行工具，权限全开，不做限制。instructions 只规范路由和回复格式。

#### MCP instructions 扩充

`CHANNEL_INSTRUCTIONS` 从简短格式说明扩展为完整指引：
- 角色图谱（user/lead/coder/reviewer/tester 职责）
- 路由规则（按上下文决定 reply 目标）
- 工作风格（权限全开、主动汇报、简洁消息）
- `initialize_result(role)` 动态追加 `"Your role: {role}"`

**文件:** `bridge/src/mcp_protocol.rs`

### 2026-03-26: 诊断陷阱 — WS 测试导致 bridge 注册丢失

#### 陷阱 1: is_allowed_agent 拒绝后静默断连

用非法 agentId（如 `"test-user"`、`"test-monitor"`）连接 daemon 控制 WS 时，`is_allowed_agent` 返回 false → handler `break` → 连接立即关闭。后续 `agent_reply` 永远不会被处理，但没有任何客户端错误提示。

**只允许 `"claude"` 和 `"codex"` 两个 agentId。**

#### 陷阱 2: 连接为 "claude" 会覆盖真实 bridge

用 `agentId: "claude"` 连接 WS 时，`attached_agents.insert("claude", new_tx)` 会覆盖真实 bridge 的 tx。断开后 `attached_agents.remove("claude")` 清除条目。此时真实 bridge 的 WS 连接仍然活着，但已不在 `attached_agents` 中 — 所有发给 Claude 的消息都会被 buffer 而非 deliver。

**唯一恢复方式:** 重启 app 或重启 Claude（让 bridge 重新连接并发 `agent_connect`）。

**架构隐患:** bridge 的 daemon_client 不感知 tx 被替换，不会主动重新注册。后续可考虑在 daemon 侧检测同 agentId 重复连接并拒绝/通知。

### 2026-03-27: Claude thinking 与终端强制聚焦

- [已修复] Claude 现在有独立的 `claude_stream` 事件链：`thinkingStarted`、`preview`、`done`、`reset`。当消息成功投递给 Claude 时，Messages 面板会出现 Claude thinking 占位；Claude 回 reply、终端退出、显式断开或用户 stop 时会清空状态。
- [已修复] Claude thinking 现在按“直接做减法”收口。虽然 daemon 侧仍保留 `claude_stream.preview` 生命周期事件，但前端不再消费 preview 文本，也不再尝试把 PTY 内容摘要渲染进消息区；Messages 面板只显示一个稳定的 Claude `thinking…` 占位。
- [已修复] `claude_terminal_attention` 现在不只负责切 tab，daemon 还会先尝试 `show -> unminimize -> set_focus` 主窗口，前端再递增 `claudeFocusNonce` 让 `ClaudeTerminalPane` 在 tab 可见时直接 `terminal.focus()`。这条逻辑不再依赖 Claude bridge 已连接，因此 development prompt / 启动期交互 prompt 也能直接接收键盘输入。
- [已修复] bridge `reply` tool 现在拒绝空白 `text`，避免 Claude 产生空消息后污染 daemon 路由和消息历史。

**验证:** ✅ Claude terminal attention 触发后无需鼠标点击即可直接输入；Messages 面板只显示单一 Claude thinking 占位；Claude 发送空 reply 时不会出现空白消息气泡。

### 2026-03-27: 现场复核补充（runtime panic / 窗口前置）

- [已修复] `claude-pty-watch` 线程之前会在运行态直接 panic。根因是 `prompt.rs` 的 PTY watcher 运行在普通 `std::thread` 中，但 `gui.rs` 的 `emit_claude_stream(Preview)` 内部又调用了 `tokio::spawn` 来挂 thinking idle timeout；该线程没有 Tokio reactor，于是现场报错 `there is no reactor running` 并直接打死 watcher。当前已改为 `tauri::async_runtime::spawn`，不再依赖调用线程本身挂着 Tokio runtime。
- [已修复] 仅靠前端 `terminal.focus()` 还不足以覆盖“App 窗口本身已经失焦或被最小化”的场景。当前 daemon 在发 `claude_terminal_attention` 前，会先尝试拉起主窗口，再由前端聚焦 xterm，从而把“自动切 tab”补全为真正可输入的 force focus。

**验证:** ✅ `cargo test --manifest-path src-tauri/Cargo.toml` 通过（78 tests）；`cargo clippy --workspace --all-targets -- -D warnings` 通过；`bun run build` 通过；dev 模式下重建后未再复现 `there is no reactor running` panic。

### 2026-03-27: Claude thinking 不再被静默超时提前结束

- [已修复] daemon 侧 15 秒 Claude idle timeout 已移除。之前即使 Claude reply 还没结束，只要一段时间没有新的 preview/终端输出，daemon 也会主动发 `claude_stream.done`，导致 Messages 面板里的 Claude thinking 卡片提前消失。
- [已修复] 当前 Claude thinking 只由真实生命周期事件结束：
  - Claude 发回非空 reply 时 `control/handler.rs` 发 `Done`
  - Claude 断开、终端退出、用户强制断开时发 `Reset`
- [结果] 对“Claude 还在处理但暂时没有终端输出”的场景，Messages 面板会继续保留 Claude thinking，占位不会再被提前清掉。

### 2026-03-27: Superpowers 复核收口

- [已修复] `claude_terminal_attention` 在用户已经停留在 Claude tab 时，不再把 `claudeNeedsAttention` 脏状态残留在 store。当前前端通过 `getClaudeAttentionResolution()` 同时决定“是否切 tab”和“是否清空 store attention”，避免后续切去别的 tab 时被强行弹回 Claude。
- [已修复] `claudeNeedsAttention` 的清理不再通过组件内 `useBridgeStore.setState(...)` 直接写 store；前端 store 新增 `clearClaudeAttention()` action，保持 attention 生命周期与其它 bridge 状态更新路径一致。
- [已修复] `ClaudeTerminalPane` 的强制 focus effect 已缩减为只由 `focusNonce` 驱动；`connected` / `running` 状态变化不再隐式触发 `terminal.focus()`。
- [说明] Claude preview 文本清洗链路仍保留在 daemon 侧，但当前前端已不再消费该 preview；UI 行为以单一 thinking 占位为准。
- [已修复] Claude thinking 的启动判定现在与真实路由结果绑定在同一份 daemon state 快照里，不再依赖 `route_message_with_display()` 额外读取一次 `claude_role`。角色切换瞬间不会再出现“消息已按旧角色投递、thinking 却按新角色判断”的竞态。

### 2026-03-27: Claude reply 协议新增 `status`

- [已修复] Claude `reply` tool 现在使用 `reply(to, text, status)`，`status` 固定为三态：`in_progress`、`done`、`error`。
- [已修复] bridge 侧新增严格校验：缺失 `status` 仍兼容并默认按 `done`，但只要传了非法值，就会返回明确的 MCP tool error：`Invalid status: "<value>". Expected "in_progress", "done", or "error".`
- [已修复] Claude → daemon 的统一 `BridgeMessage` 已新增可选 `status` 字段；bridge 转发到 Claude channel 时，也会把 `status` 作为可选 meta 属性透传到 `<channel ... status="...">`。
- [已修复] Claude thinking 的结束条件已切到显式状态：`done` / `error` 会结束 thinking，`in_progress` 不会；空消息仍不渲染，但允许 `status=done|error` 只负责结束 thinking。

**验证:** ✅ `cargo test --manifest-path bridge/Cargo.toml` 通过（19 tests）；`cargo test --manifest-path src-tauri/Cargo.toml` 通过（85 tests）。

### 2026-03-27: 非 lead 默认只回 lead

- [已修复] Claude prompt 与 bridge `CHANNEL_INSTRUCTIONS` 现在都增加了分层路由默认值：
  - `lead` 可以按上下文直接回复用户或分派给其他 worker
  - 非 `lead` 角色默认只回 `lead`
  - 只有当用户明确点名该身份或明确要求该身份直接回答时，非 `lead` 才允许直接回复 `user`
  - 非 `lead` 只有在当前指令明确点名目标 worker 时，才允许直接发给其他非 `lead` 角色；否则仍回 `lead`
- [目的] 收紧多 agent 的外显发言面，默认由 `lead` 作为对用户的汇总出口，减少 worker 角色在 auto/broadcast 场景下直接面向用户发言的噪声。

### 2026-03-27: 移除 tester，reviewer 覆盖测试职责

- [已修复] Claude 当前角色模型已收敛为 `lead / coder / reviewer` 三角色；`tester` 已从当前 allowlist、prompt 角色图谱和可选 target 中移除。
- [已修复] `reviewer` 现在同时承担 code review 与 test verification：既负责质量审查，也负责运行测试、验证行为、汇总测试结果。
- [已修复] Codex 在线占用 `lead` 时，Claude 启动前会先做角色冲突检查；同 role 冲突会在启动前直接拒绝，不再等 PTY 拉起后才在 bridge 连接层失败。

### 2026-03-27: register_mcp 改为写入真实 Claude role

- [已修复] `register_mcp()` 之前每次都把 `.mcp.json` 写成 `AGENTBRIDGE_ROLE=lead`。当 Claude 在 UI 中被切成 `coder` 或 `reviewer` 后，bridge 初始化给 Claude 的 `instructions` 与 reply tool 上下文仍然会按 `lead` 注入，导致 MCP 调用语义漂移。
- [已修复] 现在 `register_mcp()` 会先从 daemon 读取当前 `claudeRole`，再把真实角色写进 `.mcp.json`。这样 Claude 重新连接后，bridge `initialize_result(role)` 与 reply tool 所在上下文就会和 UI 选中的角色一致。
- [结果] Claude 作为 `coder` / `reviewer` 运行时，不再因为 bridge env 被硬编码成 `lead` 而走错 reply 语义。

### 2026-03-27: Claude 消息颜色固定按模型身份显示

- [已修复] Claude 发回消息时，daemon 现在会同时保留两层身份：
  - `from=lead|coder|reviewer` 作为内部路由角色
  - `displaySource=claude` 作为前端展示身份
- [已修复] Messages 面板的 badge 和颜色改为优先使用 `displaySource`，因此 Claude 即使当前扮演 `coder` / `reviewer`，气泡仍保持 Claude 紫色。
- [已修复] 当展示身份与路由角色不同，UI 会在 Claude badge 旁边显示一个次级 role label，保留“这条消息是 Claude 以哪个角色发出的”语义，但不再让颜色跟角色漂移。

**验证:** ✅ `cargo test --manifest-path src-tauri/Cargo.toml` 通过；`bun test tests/message-panel-view-model.test.ts` 通过；attention 留存与强制 focus 回归路径已覆盖。

### 2026-03-27: 现场故障修复（终端空白等待态）

- [已修复] Claude terminal 在“channel 已连接 / PTY 正在启动，但 아직没有任何 terminal chunk”时，不再显示空白黑屏。当前前端通过 `getClaudeTerminalPlaceholder()` 区分三种状态：
  - idle: `Claude terminal is idle. Connect Claude to start an embedded session.`
  - starting: `Claude terminal is starting. Waiting for output…`
  - connected/no-output: `Claude is connected. Waiting for terminal output…`
- [已修复] 这样即使 Claude 还没开始打印任何 ANSI 输出，用户也能知道它是“正在等输出”，不是前端已经失去响应。

### 2026-03-27: Claude system instructions 更新 — online agents 查询文档

**问题:** `CHANNEL_INSTRUCTIONS` 中没有说明 lead 应如何发现在线的 worker agents。Claude 作为 lead 时不知道有 `get_online_agents()` 工具，也不知道其返回结构，导致委派任务时无法明确选择目标。

**修复:** 在 `CHANNEL_INSTRUCTIONS` 的 `## Routing Policy` 前新增 `## Discovering Online Agents` 章节，说明：

- 委派前应先调用 `get_online_agents()` 查询在线 agents
- 返回的每个条目包含 `agent_id`、`role`、`model_source` 三个字段
- transport 层**不会**自动选择目标；lead 必须自己根据列表决定委派对象

**文件:** `bridge/src/mcp_protocol.rs`

**测试:** 新增 `instructions_document_online_agents_query` 测试，断言 instructions 中包含 `get_online_agents`、`agent_id`、`role`、`model_source`，以及"transport 层不自动选择"说明。

**验证:** ✅ `cargo test --manifest-path bridge/Cargo.toml mcp_protocol` — 7 tests passed.

### 2026-03-27: 统一在线 Agent 查询 — 全量验证通过

**摘要:** Claude 和 Codex 的在线 agent 查询能力已统一。两侧使用同一个 `DaemonState::online_agents_snapshot()` 数据源，返回结构相同（`agent_id`, `role`, `model_source`）。

**当前状态:**
- Claude 通过 `get_online_agents()` MCP tool 查询
- Codex 通过 `get_status()` 动态工具查询
- 两者返回格式一致
- 不支持 `send_to_agent_id`（实例级精确路由）
- 路由目标仍按角色名匹配

**验证:** 全量通过 — 112 Tauri tests, 26 bridge tests, 26 frontend tests, clippy clean, build success.

### 2026-04-01: Claude SDK (`--sdk-url`) 链路补齐多代理协议

- [已修复] SDK 直连模式下，`ReplyInput` 默认 `auto` 目标之前只把 legacy bridge `attached_agents["claude"]` 视为 Claude 在线，导致“只连了 Claude SDK 时，用户输入直接在 daemon 入口被判成没有在线目标”，消息面板连用户自己的发送也不显示。当前 `resolve_user_targets()` 已改为统一走 `DaemonState::is_agent_online()`，Claude SDK `claude_sdk_ws_tx` 也算在线。
- [已修复] SDK 启动之前把 `mcp_config` 传成 `None`，实际等价于 `--strict-mcp-config '{}'`。这会把 `reply()` / `get_online_agents()` 两个 Dimweave MCP tool 全部拿掉，而当前 Claude role prompt 又明确要求依赖这两个工具完成交付和委派。现在 SDK launch 会复用宿主生成的 `agentnexus` MCP 配置，直接把 inline `mcpServers.agentnexus` JSON 传给 `--strict-mcp-config`，恢复多代理协议。
- [已修复] SDK strict MCP config 现在会先读取目标 workspace 现有的 `.mcp.json`，再把 `agentnexus` server upsert 进去，而不是用空对象覆盖整个配置。这样用户项目里原本已有的其他 MCP servers 不会在 SDK 模式下丢失。
- [已修复] ClaudePanel 的 SDK launch 请求之前把 `roleId` 写死成 `lead`，UI 上选中的 `coder` / `reviewer` 根本不会进入后端。现在前端会显式把当前 `claudeRole` 带进 `daemon_launch_claude_sdk` 请求，并加了回归测试锁住。
- [已修复] SDK launch 的 ready 握手之前存在本地竞态：先 spawn Claude，再登记 `claude_sdk_ready_tx` 和 epoch。Claude 如果足够快先连回 `/claude`，launch 会错过 ready 信号并在 30 秒后假失败。现在改为先保留 ready slot，再 spawn 子进程。
- [已修复] Claude SDK 子进程的 `stdout` / `stderr` 之前都被 `piped` 但没人消费。bridge 模式下 stdout 会镜像 NDJSON，长会话可把 pipe buffer 塞满，表现成 Claude 卡死不再继续回消息。现在后台会持续 drain 两路 stdio，避免阻塞。
- [已修复] 恢复 MCP bridge 后，SDK `assistant/result` 事件如果继续直接路由到 `user`，会和 `reply()` 工具送回的正式消息双轨并存。当前改为“只有在 bridge 尚未接入时，SDK 文本事件才 direct-to-user 作为 fallback”；一旦 bridge 在线，正式可见消息统一走 `reply()`。
- [已修复] MCP bridge 在 SDK 会话仍在线时断开，daemon 之前会按旧语义直接把 `claude` 置为 offline，并清空 provider connection。现在 bridge 断开只会移除附着的 MCP 控制连接；只要 SDK WS 仍在线，Claude provider 状态保持 connected。

**验证:**
- `cargo test --manifest-path src-tauri/Cargo.toml` — 230 passed
- `bun test` — 59 passed
- `bun x tsc --noEmit` — passed
- `bun run build` — passed

### 2026-04-02: Claude `--sdk-url` 全量回归收口

- [已修复] Claude 运行时主链路现在完全收口到 `--sdk-url`。前端 `Connect Claude`、daemon `ResumeSession`、`AttachProviderHistory` 三条路径都统一走 SDK launch，不再回落到 PTY/channel runtime。
- [已修复] 旧的 `launch_claude_terminal` Tauri command 已从运行时移除；`main.rs` 不再编译 `claude_launch` / `claude_session` 模块，避免“UI 已切 SDK、编译入口仍保留 PTY”的混合态。
- [已修复] ClaudePanel 在 SDK 化过程中曾残留一个 `terminalRunning` 引用，导致前端仍带着 PTY 启动文案。当前按钮和提示文本都已改成 SDK 语义。
- [已修复] 消息区的 Claude thinking 占位之前还写着 `Live in Claude Terminal`。当前已统一改为 SDK 文案，MessagePanel 也不再保留 `claude` tab 类型和对应测试分支。
- [已修复] bridge-store 之前仍在监听 `claude_terminal_*` 事件并维护 `claudeTerminalChunks / Running / Detail / FocusNonce` 等状态，但运行时已经没有任何组件消费。当前已把这些事件、store 字段和前端残留组件一起移除。
- [已修复] SDK 启动路径之前只做了 `which("claude")`，没有复用 GUI 形态下必须的 PATH enrich / sidecar 解析。当前改为复用 `resolve_claude_bin()` 和 `enriched_path()`，并补了 `spawn_claude` 参数测试锁住 `PATH`、`--sdk-url`、`--resume`、`--strict-mcp-config`。
- [已修复] Claude MCP bridge 在一个 SDK turn 中途 attach 时，后续 `assistant/result` 事件之前会因为 `attached_agents["claude"]` 变为在线而被立即停发，造成最后一段 SDK fallback 文本丢失。当前 daemon state 新增 “本轮是否已经开始 SDK 直出” 状态；只要本轮已经开始 direct routing，就会保留到 `result` 再清掉，避免 hard handoff 吞消息。
- [已修复] `.claude/rules/tauri.md` 和 `.claude/rules/frontend.md` 之前仍把 `launch_claude_terminal` / `claude_terminal_*` 写成当前协议。当前规则文档已同步成 SDK-only 运行面。

**验证:**
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun test`
- `bun x tsc --noEmit`

## 当前已知限制

- Channel preview 是实验性功能，需要 `--dangerously-load-development-channels`
- 依赖 Claude Code >= 2.1.80 / permission relay >= 2.1.81
- 当前暴露 2 个 MCP tool：`reply` 和 `get_online_agents`
- `--agent --agents` 角色注入方案已研究（可行），尚未实现
- 切角色后仍需要重新 register + 重启 Claude，bridge 才会读取新的 `.mcp.json` env
- meta key 不能包含连字符（会被 Claude Code 静默丢弃）
- `chat_targets` eviction 是随机的（HashMap 无序），长会话可能影响活跃对话
- bridge 重连时不重发 pending permission requests
- 同 agentId 重复 WS 连接会覆盖已有注册，无保护机制

---

## 2026-04-17 — MCP reply tool schema + envelope field canonicalization

**问题**：Claude MCP `reply` tool input schema 与 Codex `output_schema` 字段名和结构形态分裂（`text` vs `message`，松散 object vs 扁平全必填），BridgeMessage wire 存储用第三种形态（judgment enum serde）。三源契约漂移。

**修复**：
1. `bridge/src/tools.rs::reply_tool_schema` 改为扁平 3-field target（`kind` + `role` + `agentId` 全必填 + `additionalProperties:false`），与 Codex `output_schema::target` 完全对齐。
2. envelope 字段 `text` → `message`，Claude MCP / Codex output / BridgeMessage 存储全部统一。
3. `MessageTarget` 移到独立模块，自定义 `Serialize`/`Deserialize` 发射扁平形态，Rust 枚举变体保留（类型安全不损失）。`Deserialize` 兼容老判别联合形态（持久化数据不破）。
4. `parse_target` 检测 legacy `{to: ..., ...}` 时返回明确错误，帮助模型自修复。

**对 Claude 链路的影响**：
- Incoming `<channel>` 标签新增 `sender_agent_id` 和 `task_id` 属性（source 为 Agent 时），帮助 worker 用 `{kind:"agent", agentId}` 回链具体 delegator。
- Prompt 教学增加"agent_id-first targeting" 段，要求模型优先按 `sender_agent_id` 回复而非 role-broadcast。

**详细修复记录**：见 [codex-chain.md 2026-04-17 条目](codex-chain.md)（主要落地在 daemon 侧，两链路共享同一份 wire 契约修复）。
