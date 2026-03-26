# Claude 链路修复记录

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

- `server:agentbridge` — 加载 `.mcp.json` 中名为 `agentbridge` 的 MCP server 作为 channel
- `plugin:<name>@<marketplace>` — 加载插件形式的 channel
- 此 flag 绕过 allowlist，仅限开发测试使用
- `--dangerously-skip-permissions` — 默认跳过 Claude CLI 的本地 permission 确认，当前 AgentBridge 以 bypass permission 作为默认启动方式
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
<channel source="agentbridge" chat_id="123" from="user">
消息内容
</channel>
```

### Reply Tool 参数

tool 通过 `ListToolsRequestSchema` handler 注册:

| 参数 | 类型 | 必填 | 作用 |
|------|------|------|------|
| `name` | `string` | 是 | Tool 名称，如 `"reply"` |
| `description` | `string` | 是 | Tool 描述，Claude 用来决定何时调用 |
| `inputSchema` | JSON Schema | 是 | 输入参数 schema。当前 bridge 用 `chat_id` + `text` |

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
    "agentbridge": {
      "command": "/absolute/path/to/agent-bridge-bridge",
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
| meta 属性 (`from`, `chat_id`) | `channel_state.rs` prepare_channel_message | ✅ 已实现 |
| Sender gating | `channel_state.rs` ALLOWED_SENDERS | ✅ 已实现 |
| Pre-init message buffering | `mcp.rs` pre_init_buffer | ✅ 已实现 |

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
- [已修复] research preview 下 `--dangerously-load-development-channels server:agentbridge` 的本地开发确认提示，现在由 PTY watcher 自动输入 `1`
- [已修复] Claude 默认启动参数现在显式附带 `--dangerously-skip-permissions`，不再只依赖运行时交互去放行本地 permission；当前默认行为就是 bypass permission
- [约束] 自动确认只对 `Channels: server:agentbridge` 生效，不会对其他 development channel 做泛化放行
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
- [已修复] `claude_terminal_attention` 事件之前无法稳定把 GUI 切到终端 tab；根因是 prompt 检测把最近 500 字符里只要还含有 `server:agentbridge` 和 `local development` 就整段排除，导致开发确认 prompt 后紧接着出现的真实交互 prompt 也被一起误判为“无需 attention”；当前改为只分析最后一个非空 prompt block，并补了回归测试
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

#### [已知限制] `attention_fired` 每次 PTY 生命周期只触发一次

- **现象**: `attention_fired` 标志在首次 attention 后永久为 true，同一 PTY session 内后续 prompt 不再触发 tab badge
- **根因**: `prompt.rs` 中 `attention_fired` 设为 true 后无重置机制
- **影响**: 低频场景（同一 session 内多次需要用户关注）会漏发 attention 事件
- **建议**: 在 `confirmed` 从 false → true 时重置 `attention_fired`，允许后续真实 prompt 再次触发

## 当前已知限制

- Channel preview 是实验性功能，需要 `--dangerously-load-development-channels`
- 依赖 Claude Code >= 2.1.80 / permission relay >= 2.1.81
- 当前只有 `reply` 一个 tool
- 不支持 `--agent --agents` 角色注入
- meta key 不能包含连字符（会被 Claude Code 静默丢弃）
- `chat_targets` eviction 是随机的（HashMap 无序），长会话可能影响活跃对话
- bridge 重连时不重发 pending permission requests
