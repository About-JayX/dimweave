# Claude Code 集成方案深度分析（2026-04-01）

> 目标：系统性对比 Claude Code 所有可用集成方式，为 AgentNexus 找到比当前 `channel` 方案更优的接入路径。

## 结论先行

| 方案 | 稳定性 | 控制力 | 适配难度 | 认证 | 推荐度 |
|------|--------|--------|----------|------|--------|
| **`--sdk-url` WS 直连** (Rust 原生) | ★★☆ | ★★★★★ | ★★☆ | claude.ai | **最优解（需验证 + 风险承担）** |
| **Claude Agent SDK** (TS/Python) | ★★★★★ | ★★★★★ | ★★★☆ | API key | **首选稳定方案** |
| **CLI stream-json stdio** (Rust 原生) | ★★★☆ | ★★★★ | ★★★☆ | API key / claude.ai | Rust 原生备选 |
| Agent Teams (实验性) | ★★☆ | ★★★★ | ★★★★ | claude.ai | 观望 |
| Channel (当前方案) | ★★★ | ★★★ | ★★★ | claude.ai | 当前可用 |
| 直接 Anthropic API | ★★★★★ | ★★★★★ | ★★★★★ | API key | 最终形态 |

**核心判断：Claude Agent SDK (`@anthropic-ai/claude-agent-sdk`) 是当前最佳替代方案。** 它提供了 Channel 的所有能力（session 管理、tool 控制、MCP 支持），还额外提供了 Channel 不具备的能力（programmatic hook callbacks、structured output、multi-turn streaming input、subagent 定义），而且它是官方正式发布的 API，不需要 `--dangerously-*` flag。

---

## 方案一：Claude Agent SDK（首选推荐）

### 是什么

`@anthropic-ai/claude-agent-sdk` (TypeScript) / `claude-agent-sdk` (Python) 是 Anthropic 官方发布的库，允许以编程方式运行 Claude Code。底层仍然是 Claude Code 进程，但暴露了完整的类型安全 API。

### 核心 API

```typescript
import { query, listSessions, getSessionMessages } from "@anthropic-ai/claude-agent-sdk";

// 基本用法 - 返回 AsyncGenerator<SDKMessage>
const q = query({
  prompt: "Fix the bug in auth.py",
  options: {
    allowedTools: ["Read", "Edit", "Bash"],
    model: "claude-sonnet-4-6",
    cwd: "/path/to/project",
    permissionMode: "bypassPermissions",  // 或 "default" / "acceptEdits" / "dontAsk"
    allowDangerouslySkipPermissions: true,
  }
});

for await (const message of q) {
  // 处理流式消息
}
```

### 对 AgentNexus 的关键能力

#### 1. Session 管理（取代 PTY + channel session tracking）

```typescript
// 列出历史 session
const sessions = await listSessions({ dir: "/path/to/project", limit: 20 });

// 恢复 session
const q = query({
  prompt: "继续之前的工作",
  options: { resume: "session-uuid" }
});

// Fork session（从某个点分叉）
const q2 = query({
  prompt: "试另一个方案",
  options: { resume: "session-uuid", forkSession: true }
});
```

#### 2. Multi-turn Streaming Input（取代 channel notification）

```typescript
// 创建一个持续对话的 async iterable
async function* userMessages(): AsyncIterable<SDKUserMessage> {
  yield { type: "user", content: "开始工作" };
  // ... 等待 daemon 路由来的消息
  yield { type: "user", content: "<channel from='coder'>完成了代码</channel>" };
}

const q = query({
  prompt: userMessages(),  // 替代 channel notification 推送
  options: { ... }
});

for await (const msg of q) {
  // Claude 的回复流式到达
}
```

#### 3. Programmatic Hooks（取代 bridge permission relay）

```typescript
const q = query({
  prompt: "修复 bug",
  options: {
    hooks: {
      PreToolUse: [{
        matcher: "Bash|Edit|Write",
        hooks: [async (input, toolUseId, context) => {
          // 发到 GUI 审批，等待用户决策
          const verdict = await daemonPermissionRelay(input);
          return {
            hookSpecificOutput: {
              hookEventName: "PreToolUse",
              permissionDecision: verdict ? "allow" : "deny",
            }
          };
        }]
      }]
    }
  }
});
```

#### 4. Subagent 定义（取代角色系统的部分功能）

```typescript
const q = query({
  prompt: "用 code-reviewer 审查代码",
  options: {
    allowedTools: ["Read", "Glob", "Grep", "Agent"],
    agents: {
      "code-reviewer": {
        description: "代码审查专家",
        prompt: "你是高级代码审查员...",
        tools: ["Read", "Glob", "Grep"],
        model: "sonnet"
      }
    }
  }
});
```

#### 5. MCP 服务器集成（保留现有 MCP 工具）

```typescript
const q = query({
  prompt: "...",
  options: {
    mcpServers: {
      "agentnexus": {
        command: "/path/to/agent-nexus-bridge",
        // 或者用 in-process SDK MCP server 替代外部进程
      }
    }
  }
});
```

#### 6. Structured Output（新能力）

```typescript
const q = query({
  prompt: "分析这个 PR",
  options: {
    outputFormat: {
      type: "json_schema",
      schema: {
        type: "object",
        properties: {
          summary: { type: "string" },
          issues: { type: "array", items: { type: "object", properties: {
            severity: { type: "string" },
            description: { type: "string" },
          }}}
        }
      }
    }
  }
});
```

### 迁移影响分析

| 当前组件 | SDK 方案替代 | 变化 |
|----------|-------------|------|
| `bridge/` (Rust MCP sidecar) | **可能完全移除** — SDK 内建 MCP + hooks 取代 channel 通信 |
| `claude_session/` (PTY 管理) | **移除** — SDK 管理 Claude 进程生命周期 |
| `claude_cli.rs` (版本检查) | **简化** — SDK 自带版本管理 |
| `mcp.rs` (.mcp.json 注册) | **简化** — SDK 内建 MCP server 配置，不需要文件注册 |
| `claude_prompt.rs` (channel instructions) | **改为 systemPrompt 选项** — 通过 SDK 参数注入 |
| `daemon/routing.rs` (Claude 方向) | **改用 streamInput()** — 通过 SDK 流式输入替代 channel notification |
| Permission relay | **改用 SDK hooks** — PreToolUse callback 替代 channel/permission |
| Provider history | **listSessions() + getSessionMessages()** — SDK 原生支持 |

### 架构变化

```text
┌─ React 前端 ─────────────────────────────────────────────────────┐
│ bridge-store / task-store                                       │
└──────────────┬──────────────────────────────────────────────────┘
               │ Tauri invoke / listen
               ▼
┌─ Tauri 主进程 / Rust daemon ─────────────────────────────────────┐
│ Claude adapter:                                                  │
│   - 调用 Node.js 子进程运行 Agent SDK wrapper                     │
│   - 或通过 Rust FFI 调用 SDK                                     │
│   - SDK wrapper 管理 query() 生命周期                             │
│   - hook callbacks 桥接回 daemon 做 permission relay             │
│   - streamInput() 接收路由来的消息                                │
│                                                                  │
│ Codex adapter: (保持不变)                                         │
│   - WS :4500 连 codex app-server                                 │
│                                                                  │
│ routing.rs: (简化)                                                │
│   - Claude 方向改为 SDK streamInput                               │
│   - Codex 方向保持不变                                            │
└──────────────────────────────────────────────────────────────────┘
```

### 实现难点

1. **SDK 是 Node.js 包** — Tauri/Rust 主进程需要通过子进程或 sidecar 运行 TS/JS wrapper
2. **Hook callbacks 是 async 函数** — 需要跨进程桥接（daemon → SDK wrapper → hook callback → daemon → GUI → daemon → SDK wrapper）
3. **streamInput 的生命周期管理** — 需要把 daemon 路由层的消息转为 `AsyncIterable<SDKUserMessage>`
4. **认证模型变化** — SDK 需要 API key（`ANTHROPIC_API_KEY`），不支持 claude.ai 订阅认证。这改变了计费模型（API token 费率 vs 订阅包月）。当前 channel 方案基于 claude.ai 登录
5. **SDK 版本** — TypeScript SDK 当前 v0.2.71，尚未 1.0。API 可能变动

### V2 API（Preview，更适合 AgentNexus）

SDK 还有一个 V2 preview 接口，用 `send()` / `stream()` 模式替代 async generator，对多轮对话特别友好：

```typescript
import { unstable_v2_createSession, unstable_v2_resumeSession } from "@anthropic-ai/claude-agent-sdk";

// 创建新 session
await using session = unstable_v2_createSession({
  model: "claude-opus-4-6",
  // ... 其他选项同 V1 Options
});

// 发送消息（等同于 channel notification 推送）
await session.send("开始工作");

// 接收 Claude 回复流
for await (const msg of session.stream()) {
  // 转发到 daemon routing
}

// 发送第二轮消息（daemon 路由来的 Codex 消息）
await session.send("<channel from='coder'>代码完成了</channel>");

for await (const msg of session.stream()) { ... }

// 恢复历史 session
await using resumed = unstable_v2_resumeSession("session-uuid", {
  model: "claude-opus-4-6"
});
```

**V2 对 AgentNexus 的意义：**
- `send()` / `stream()` 直接对应 AgentNexus 的 "路由消息到 Claude" + "接收 Claude 回复" 模型
- 不需要管理 async generator / yield 协调
- Session 生命周期管理更清晰（create → send/stream 循环 → close）
- 可以在两轮之间做任意处理（permission relay、消息转换等）

### 可行实现路径

**方案 A：Node.js sidecar wrapper（推荐）**
- 写一个 TypeScript sidecar 进程，import `@anthropic-ai/claude-agent-sdk`
- Tauri daemon 通过 stdin/stdout JSON-RPC 或 WS 与 sidecar 通信
- Sidecar 用 V2 `createSession()` / `send()` / `stream()` 管理 Claude 生命周期
- Hook callbacks 通过 WS 桥接回 daemon 做 permission relay

**方案 B：保留 Rust bridge + 用 SDK 替代 channel**
- 仍用 Rust bridge 作为 MCP server（保留 reply/get_online_agents tool）
- 但启动 Claude 时不用 `--dangerously-load-development-channels`
- 改用 `claude -p --mcp-config .mcp.json --output-format stream-json`
- 损失：没有 channel notification 的主动推送能力

**方案 C：渐进式迁移（最稳妥）**
- Phase 1: 用 SDK `listSessions()` / `getSessionMessages()` 替代自写的 provider history 扫描
- Phase 2: 用 SDK hooks 替代 channel permission relay
- Phase 3: 用 SDK V2 `createSession()` + `send()` / `stream()` 替代 PTY + channel 全链路

---

## 方案二：Agent Teams（实验性，观望）

### 是什么

Claude Code 内建的多 agent 协调系统。一个 lead session 可以 spawn 多个 teammate，它们有独立 context window，通过 shared task list 和 mailbox 通信。

### 关键特性

- 每个 teammate 是一个独立的 Claude Code instance
- Shared task list: pending → in_progress → completed，支持 dependency
- 直接消息: teammate 之间可以 `SendMessage` 直接通信
- Display modes: in-process (同一终端) 或 split-pane (tmux/iTerm2)
- Permission 继承: teammates 继承 lead 的权限设置
- 存储位置: `~/.claude/teams/{team-name}/config.json`

### 与 AgentNexus 的契合度

**高度契合的部分：**
- AgentNexus 的核心需求就是"协调多个 agent 协同工作"
- Agent Teams 的 task list + mailbox 模型和 AgentNexus 的 task_graph + routing 非常对应
- 角色系统 (lead/coder/reviewer) 可以直接映射到 teammate 的 subagent type

**不契合的部分：**
- **实验性** — 需要 `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1`，API 随时可能变
- **仅限 Claude** — Agent Teams 只协调 Claude Code instances，无法接入 Codex
- **固定 lead** — 不能动态切换 lead，而 AgentNexus 的 lead 可以是任何 provider
- **无 programmatic API** — 目前只通过 interactive CLI 控制，没有 SDK 接口

### 判断

**短期不适合替代 channel，但长期值得关注。** 如果 Agent Teams 稳定下来并获得 SDK 支持（能 programmatically spawn teammates、send messages、manage tasks），它可能成为 AgentNexus Claude 侧的最佳选择。

---

## 方案三：Channel（当前方案）

### 当前实现

- MCP server 声明 `experimental['claude/channel']` capability
- 通过 `notifications/claude/channel` 推送消息到 Claude context
- 通过 `reply` tool 让 Claude 发消息回来
- 通过 `notifications/claude/channel/permission` relay permission prompts
- 启动时需要 `--dangerously-load-development-channels server:agentnexus`

### 优势

- **真正的双向通信** — notification 主动推送 + tool 主动回复
- **Channel 官方文档完善** — 有正式的 API reference
- **AgentNexus 已经完全实现** — 所有基础设施已就位
- **与 PTY 共存** — 用户仍能看到 Claude 的终端输出

### 劣势

- **`--dangerously-*` flag** — 名字本身就暗示非生产用途
- **Research preview** — 文档明确说 "research preview"，需要 claude.ai 登录
- **No programmatic control** — 无法通过代码控制 Claude 的行为，只能通过 notification + system prompt
- **Permission relay 笨重** — 需要自己实现 request_id tracking、verdict format 解析
- **PTY 管理复杂** — 需要 portable-pty + auto-confirm + exit watcher

### 去掉 `--dangerously-*` 的路径：Plugin 系统

Channel 有一条通往稳定发布的路径：**把 MCP server 包装成 Claude Code plugin**。Plugin 系统已经 stable（2025-10 公测），支持 `channels` 字段：

```json
// plugin.json
{
  "channels": [{
    "server": "agentnexus",
    "userConfig": { ... }
  }]
}
```

用户安装后可以用 `--channels plugin:agentnexus@marketplace` 启动，不需要 `--dangerously-*`。

**但是：** 进入 approved allowlist 需要 Anthropic 审核。对于本地桌面应用来说，这条路可能走不通或周期很长。

### 判断

**Channel 是"能用"但不是"最优"的方案。** 如果 SDK 方案可行，应该迁移。但 channel 作为过渡方案没有紧急替换的压力。Plugin 路径可以作为"channel 方案的正式化出路"追踪。

---

## 方案四：CLI Subprocess（轻量备选 / Rust 原生方案）

### 两个层级

#### 4a. 单次 `-p` 模式

```bash
claude -p "Fix the bug" \
  --output-format stream-json \
  --verbose \
  --include-partial-messages \
  --allowedTools "Read,Edit,Bash" \
  --mcp-config .mcp.json
```

适合"发一条指令，等结果"的简单场景。

#### 4b. 双向 `--input-format stream-json` 模式（重要发现）

```bash
claude -p --input-format stream-json --output-format stream-json \
  --verbose --include-partial-messages
```

通过 stdin/stdout NDJSON 实现**双向通信**：

**发送消息（stdin）：**
```json
{"type": "user", "message": {"role": "user", "content": "开始工作"}, "session_id": "..."}
```

**接收回复（stdout）：**
```json
{"type": "assistant", "message": {"role": "assistant", "content": [...]}, "session_id": "..."}
```

**Permission 响应（stdin）：**
```json
{"type": "control_response", "subtype": "can_use_tool", "allow": true}
```

> **关键发现：** 这就是 Agent SDK 底层使用的协议。SDK 只是它的 TypeScript wrapper。
> 直接用 Rust 实现这个 NDJSON 协议可以**完全消除 Node.js 依赖**。
>
> **但是：** 这个协议是 [未文档化的](https://github.com/anthropics/claude-code/issues/24594)（GitHub issue #24594 请求文档化被关闭为 NOT PLANNED，因为 Anthropic 认为 SDK 才是官方接口）。使用它意味着依赖一个逆向工程的内部协议。

### 对 AgentNexus 的意义

**方案 4b 是"无 Node.js 依赖的 SDK 替代"**。如果不想引入 Node.js sidecar：

```text
Tauri Rust daemon
  ├── spawn: claude -p --input-format stream-json --output-format stream-json
  ├── stdin → 发送用户/agent 消息（NDJSON）
  ├── stdout → 接收 Claude 回复流（NDJSON）
  ├── stdin → permission verdict（control_response）
  └── 用 --mcp-config 加载 .mcp.json（保留 bridge MCP tools）
```

### 优势

- **Rust 原生** — 不需要 Node.js 运行时
- **双向通信** — 和 SDK 等效的能力
- **结构化输出** — NDJSON 事件流
- **Session 续接** — `--resume <session_id>`
- **可以加载 MCP** — `--mcp-config` flag 支持

### 劣势

- **未文档化协议** — 随时可能变化，无官方保证
- **需要逆向工程维护** — SDK 更新时协议可能变
- **MCP 加载依赖 `--mcp-config`** — 需要验证在 `-p` 模式下是否完整支持
- **认证** — 支持 API key 和 claude.ai auth（比 SDK 灵活）

### 判断

如果不想引入 Node.js 依赖，4b 是唯一可行的 Rust 原生双向通信方案。但依赖未文档化协议有风险。建议同时跟踪 SDK 的 NDJSON 协议变化。

---

## 方案五：直接 Anthropic API

### 是什么

绕过 Claude Code，直接使用 Anthropic Messages API + tool_use 构建自己的 agent loop。

### 优势

- **完全控制** — 100% 掌控 system prompt、tool 定义、token 消耗
- **无外部依赖** — 不需要 Claude Code CLI
- **跨 provider** — 可以对接 Bedrock/Vertex/Azure
- **最灵活** — 可以实现任意 agent 架构

### 劣势

- **工作量巨大** — 需要自己实现 Claude Code 的所有能力：
  - File read/write/edit tools
  - Bash execution + sandbox
  - Glob/Grep 搜索
  - Context window management + compaction
  - Session persistence
  - MCP server 管理
  - Permission system
  - Auto-retry / error handling
- **失去 Claude Code 生态** — 无法使用 CLAUDE.md、skills、plugins、subagents
- **维护成本** — API 变更需要自己跟进

### 判断

**最终形态但当前 ROI 不合理。** Claude Code/Agent SDK 已经封装了 agent loop，自己重写没有额外价值。除非需要极致定制（比如自定义 token 计费、自定义 model routing），否则不建议。

---

## 迁移建议

### 推荐路线：渐进式迁移到 Agent SDK

```text
Phase 0 (当前)
  Channel + PTY + Rust bridge
  ↓
Phase 1 (低风险改进)
  保持 channel 不变
  + 用 SDK listSessions() / getSessionMessages() 替代自写的 history 扫描
  + 用 SDK getSessionInfo() 替代 transcript JSONL 解析
  ↓
Phase 2 (SDK wrapper sidecar)
  新建 TypeScript sidecar: sdk-adapter.ts
  Tauri daemon 通过 WS/stdin 与 sidecar 通信
  sidecar 用 query() 管理 Claude 生命周期
  streamInput() 替代 channel notification
  SDK hooks 替代 channel permission relay
  bridge/ 可能保留为 MCP tool provider（reply/get_online_agents），或改为 SDK 内建 MCP server
  ↓
Phase 3 (完全 SDK)
  移除 PTY 管理（claude_session/）
  移除 channel flag（--dangerously-load-development-channels）
  移除 .mcp.json 注册（SDK 内建配置）
  bridge/ 完全移除或改为 SDK createSdkMcpServer()
  Claude terminal 面板改为展示 SDK 事件流
```

### Phase 2 架构草图

```text
┌─ React 前端 ──────────────────────────────────────────────────┐
│ bridge-store → 监听 agent_message / claude_stream / permission │
│ task-store   → 监听 task / session / artifact / history        │
└──────────────┬────────────────────────────────────────────────┘
               │ Tauri invoke / listen
               ▼
┌─ Tauri Rust daemon ──────────────────────────────────────────┐
│ claude_sdk_adapter.rs:                                        │
│   spawn TypeScript sidecar → WS :4503                        │
│   route_to_claude() → sidecar.sendMessage()                   │
│   sidecar.onPermission() → GUI permission prompt              │
│   sidecar.onMessage() → routing.rs → fan-out                  │
│                                                               │
│ codex/ (不变):                                                │
│   WS :4500 → codex app-server                                │
│                                                               │
│ routing.rs (简化):                                             │
│   to=claude → claude_sdk_adapter                              │
│   to=codex → codex_inject_tx                                  │
│   to=user → GUI                                               │
└──────────────────────────────────────────────────────────────┘
               ↕ WS :4503
┌─ SDK Adapter sidecar (TypeScript) ────────────────────────────┐
│ import { query } from "@anthropic-ai/claude-agent-sdk"         │
│                                                                │
│ // 管理 Claude session                                         │
│ const q = query({                                              │
│   prompt: streamFromDaemon(),                                  │
│   options: {                                                   │
│     hooks: { PreToolUse: [permissionRelayHook] },              │
│     systemPrompt: rolePrompt,                                  │
│     allowedTools: [...],                                       │
│     mcpServers: { agentnexus: inProcessMcpServer },            │
│   }                                                            │
│ });                                                            │
│                                                                │
│ // 流式转发 SDK 事件到 daemon                                   │
│ for await (const msg of q) {                                   │
│   ws.send(JSON.stringify(msg));                                │
│ }                                                              │
└────────────────────────────────────────────────────────────────┘
```

---

## 逐条对照：SDK vs Channel

| 维度 | Channel（当前） | Agent SDK |
|------|----------------|-----------|
| 启动 flag | `--dangerously-load-development-channels` | 无（SDK 管理进程） |
| 消息推送 | `notifications/claude/channel` MCP notification | `streamInput()` async iterable |
| 消息接收 | `reply` MCP tool | SDK 消息流 `for await (const msg of q)` |
| Permission | `channel/permission_request` → `channel/permission` | `hooks.PreToolUse` callback |
| Session 管理 | PTY + `--session-id` / `--resume` | `resume` option + `listSessions()` |
| History | 自写 transcript JSONL 扫描 | `listSessions()` + `getSessionMessages()` |
| System prompt | `instructions` field in MCP initialize | `systemPrompt` option |
| Tool 控制 | MCP `tools/list` (reply + get_online_agents) | `allowedTools` + `agents` + `mcpServers` |
| 输出格式 | PTY raw text + channel notification | Typed SDKMessage stream |
| Structured output | 无 | `outputFormat: { type: 'json_schema', schema }` |
| Subagents | 无 | `agents` option + `Agent` tool |
| 进程管理 | portable-pty + exit watcher | SDK 内部管理 |
| 文档稳定性 | "research preview" | 正式发布 |

---

## 补充：`--agent` flag 可以立即改善当前方案

即使不迁移到 SDK，当前 channel 方案也可以通过 `--agent` flag 大幅改善角色注入。

### 当前做法
```rust
// claude_session/process.rs
cmd.arg("--dangerously-load-development-channels");
cmd.arg("server:agentnexus");
// + --append-system-prompt（弱注入）
```

### 改进做法
```rust
// 在 .claude/agents/ 目录下为每个角色创建 agent 定义文件
// 然后用 --agent flag 启动
cmd.arg("--agent");
cmd.arg("lead");  // 或 "coder" / "reviewer"
cmd.arg("--dangerously-load-development-channels");
cmd.arg("server:agentnexus");
```

Agent 定义文件（`.claude/agents/lead.md`）：
```markdown
---
name: lead
description: AgentNexus lead coordinator
model: inherit
---

你是 AgentNexus 多 agent 协作系统中的 lead 角色。
（完整的角色 prompt，替代当前 claude_prompt.rs 的内容）
```

**`--agent` 的优势：**
- Agent 的 system prompt **完全替换**默认 Claude Code system prompt（比 `--append-system-prompt` 强得多）
- 可以定义 `tools`（限制 lead 不能用 Edit，reviewer 只能 Read 等）
- 可以定义 `mcpServers`（每个角色不同的 MCP 配置）
- 可以定义 `hooks`（角色特定的 pre/post tool 校验）
- 文件版本控制，可以提交到仓库
- 对应 SDK 的 `agents` option，迁移到 SDK 时直接复用

---

## 补充：Claude Code 内部架构关键细节

> 来自源码分析（source map 还原 + GitHub 仓库研究）

### Session 存储格式

```text
~/.claude/projects/<encoded-cwd>/<session-id>.jsonl
```
- `<encoded-cwd>` 是绝对路径，所有非字母数字字符替换为 `-`
- Session 是 append-only JSONL transcript

**对 AgentNexus 的意义：** 当前 `provider/claude.rs` 扫描的就是这个路径。用 SDK `listSessions()` 可以直接替代自写的扫描逻辑。

### MCP 配置优先级（8 层）

1. local (`.mcp.json` 当前目录)
2. project (`.claude/.mcp.json`)
3. user (`~/.claude/.mcp.json`)
4. userSettings
5. policySettings
6. enterprise (managed)
7. claudeai
8. dynamic (runtime)

### Tool 系统

- 43+ 内建工具，24 个 feature-gated
- Read-only 工具（Read/Glob/Grep/MCP read-only）并发执行（~10 max）
- Write 工具（Edit/Write/Bash）串行执行
- 当工具描述占 context >10% 时，自动切换到 lazy loading（ToolSearch 索引）

### `--permission-prompt-tool`（重要发现）

这个 flag 可以**大幅简化 permission relay**。它把 permission prompt 委托给一个 MCP tool，而不是走 channel/permission 协议：

```bash
claude -p --permission-prompt-tool mcp__agentnexus__handle_permission \
  --mcp-config .mcp.json
```

当 Claude 需要权限审批时，不再弹出终端对话框或走 channel permission_request，而是**直接调用指定的 MCP tool**。这意味着：

- 当前链路：`bridge permission_request → daemon → GUI → permission_verdict → bridge`
- 可能的新链路：`Claude → MCP tool call(handle_permission) → bridge → daemon → GUI → tool result`

**意义：** 即使保留 channel 方案，也可以用这个 flag 替代笨重的 `claude/channel/permission` 协议。Bridge 只需要额外注册一个 `handle_permission` MCP tool。

### `claude remote-control` 模式（潜在替代方案）

```bash
claude remote-control --spawn worktree --capacity 32
```

- 启动一个**仅出站 HTTPS** 的 WebSocket 服务器模式
- 注册到 Anthropic API，轮询工作任务
- claude.ai/code 或移动端的 session 通过 Anthropic API 路由到本机
- 支持 `--spawn same-dir | worktree` 并发 session
- **可能的用途：** 如果 AgentNexus 能注册为 remote-control 的 work source，可以完全避免 PTY/channel/subprocess 管理

### Claude Code 内建 Bridge 系统

Claude Code 自身有一个 `src/bridge/` 模块（31 文件，~400KB），用于：
- 持久 WebSocket 连接到 claude.ai (Cloud Remote)
- JWT 每 3h55m 刷新
- 指数退避重连（2s → 2min → 10min）
- 容量管理（`capacityWake.ts`）

**注意：** 这个 bridge 和 AgentNexus 的 `bridge/` 不是同一个东西。Claude Code 的 bridge 是连 claude.ai 的远程协议，我们的 bridge 是 MCP channel sidecar。

---

## 隐藏参数补充（来自逆向工程）

项目已有的 `docs/agents/claude-cli-reverse-engineering.md` 发现了以下未公开参数：

| 参数 | 用途 | 当前项目是否使用 |
|------|------|-----------------|
| `--system-prompt` | 替换默认 system prompt | 否（当前用 `--append-system-prompt`） |
| `--system-prompt-file` | 从文件读取 system prompt | 否 |
| `--sdk-url` | **WebSocket 直连 + stream-json headless 模式（见下方详细分析）** | 否（但极具潜力） |
| `--channels` | 加载已审批的 channel | 否（用 `--dangerously-*` 版本） |
| `--agent-id` / `--agent-name` / `--team-name` | Agent Teams 内部用 | 否 |
| `--parent-session-id` | 父 session 关联 | 否 |
| `--teammate-mode` | Agent Teams 模式 | 否 |
| `--agent-type` | Agent type 标识 | 否 |
| `--agent` | 用 subagent 定义作为主 session | 否 |
| `--agents` | CLI 内联 subagent 定义（JSON） | 否 |

其中 `--system-prompt` 和 `--agent` 可以在任何方案中立即使用，提升 prompt 注入强度。

---

## 重大发现：`--sdk-url` — 可能是最优解

### 这是什么

`--sdk-url ws://127.0.0.1:<port>` 让 Claude Code 作为 **WebSocket 客户端**直接连到指定地址，同时自动切到 `stream-json` headless 模式。

源码逆向确认的行为：
- 自动设置 `inputFormat = stream-json`、`outputFormat = stream-json`
- 自动启用 `print` + `verbose`
- Claude Code 是 **WS 客户端**，连到你指定的 URL
- Permission prompt 自动走 WS 上的 NDJSON `control_request`/`control_response` 消息
- MCP 服务器（`--mcp-config`）仍然正常加载
- `--channels` 仍然可以共存

### 协议格式

**和 `--input-format stream-json --output-format stream-json` 完全相同的 NDJSON 协议**，只是传输层从 stdin/stdout 变成了 WebSocket：

```json
// Claude → daemon (stdout/WS)
{"type": "assistant", "message": {"role": "assistant", "content": [...]}, "session_id": "..."}

// daemon → Claude (stdin/WS)  
{"type": "user", "message": {"role": "user", "content": "..."}, "session_id": "..."}

// Permission request (Claude → daemon)
{"type": "control_request", "subtype": "can_use_tool", "tool_name": "Bash", ...}

// Permission verdict (daemon → Claude)
{"type": "control_response", "subtype": "can_use_tool", "allow": true}

// Keep alive
{"type": "keep_alive"}
```

### 对 AgentNexus 的革命性意义

```text
当前架构:
  Claude Code (PTY) → MCP stdio → bridge sidecar (Rust) → WS :4502 → daemon
  + channel notification / permission relay

--sdk-url 架构:
  Claude Code (subprocess) → WS :4502 → daemon (直接!)
  + 无 PTY，无 bridge，无 channel
```

**直接消除的组件：**
- `bridge/` — 整个 Rust sidecar crate（`channel_state.rs`, `mcp.rs`, `mcp_io.rs`, `mcp_protocol.rs`, `tools.rs`, `daemon_client.rs`）
- `claude_session/` — PTY spawn、auto-confirm、exit watcher
- `claude_launch.rs` — Terminal launch helpers
- `.mcp.json` 注册 — 不再需要为 channel 注册 MCP server
- `--dangerously-load-development-channels` — 不再需要
- Channel permission relay — permission 直接走 WS NDJSON

**daemon 需要改造的部分：**
- `control/server.rs` — 当前只处理 bridge 的 `agent_connect`/`agent_reply` 协议，需要改为理解 stream-json NDJSON 协议
- `routing.rs` — Claude 方向从 "发送到 bridge WS channel" 改为 "发送 NDJSON user message 到 Claude WS"
- 新增 permission handler — 解析 `control_request`，推到 GUI，收到用户决策后回 `control_response`

### 启动链路

```rust
// 取代当前的 spawn_session() + PTY
let mut cmd = Command::new(claude_bin);
cmd.arg("--print");
cmd.arg("--sdk-url").arg("ws://127.0.0.1:4502");
cmd.arg("--session-id").arg(&session_id);
cmd.arg("--input-format").arg("stream-json");
cmd.arg("--output-format").arg("stream-json");
cmd.arg("--mcp-config").arg(&mcp_config_path);  // 保留其他 MCP tools（如果需要）
cmd.arg("--agent").arg(&role);  // 角色定义
// 可选: cmd.arg("--channels").arg("server:agentnexus");  // 如果还需要 channel

// 恢复 session
cmd.arg("--resume").arg(&existing_session_id);

let child = cmd.spawn()?;
// Claude 会主动连到 ws://127.0.0.1:4502
// daemon 的 WS server 接收 NDJSON 消息
```

### 关键注意事项

1. **Hidden API** — `--sdk-url` 是 `.hideHelp()` 的隐藏参数，用于 Anthropic 内部的 CCR (Claude Code Remote) 基础设施
2. **认证要求** — 源码显示它期望 `CLAUDE_CODE_SESSION_ACCESS_TOKEN` 环境变量和 `Authorization: Bearer <token>` header。本地使用可能需要设置 dummy token 或测试是否可省略
3. **Agent SDK 故意不用它** — SDK 选择 stdio pipes 而非 `--sdk-url`，说明 Anthropic 视 stdio 为稳定公共 API，`--sdk-url` 为内部基础设施 API
4. **WS 传输层特性** — 自带指数退避重连、ping/pong keepalive、消息缓冲与重放（`X-Last-Request-Id`）

### 风险评估

| 风险 | 级别 | 说明 |
|------|------|------|
| API 稳定性 | 高 | 隐藏参数，无文档，无承诺 |
| 认证要求 | 中 | 需要验证本地 WS 是否能跳过 token |
| 与 Rust daemon 协议兼容 | 低 | NDJSON 在 Rust 中解析简单 |
| 与 MCP 共存 | 低 | 源码确认可以同时使用 |
| 与 channel 共存 | 低 | 源码确认两者不冲突 |

### 验证计划

在做任何架构改造之前，需要先运行最小验证：

```bash
# 1. 启动一个简单的 WS echo server (如 websocat)
websocat -s 4599

# 2. 启动 Claude 连到它
claude --sdk-url ws://127.0.0.1:4599 \
  --session-id test-001 \
  -p "hello"

# 3. 观察 WS 上收到的 NDJSON 消息格式
# 4. 验证是否需要 CLAUDE_CODE_SESSION_ACCESS_TOKEN
# 5. 验证 permission 消息格式
```

---

## 最终建议

1. **立即可做（本周）**：
   - 把 `--append-system-prompt` 升级为 `--system-prompt`，提升角色 prompt 权威性
   - 创建 `.claude/agents/` 角色定义文件，用 `--agent` 启动（替代 claude_prompt.rs）
   - **运行 `--sdk-url` 最小验证实验**（见上方验证计划），确认是否需要 token、NDJSON 消息格式、permission 流程
   - 测试 `--permission-prompt-tool` 替代 channel permission relay（可能立即简化 bridge）

2. **中期路线三选一（2-4 周）**：
   - **路线 A (--sdk-url WS 直连，最激进)**：如果验证通过，Claude 直接 WS 连 daemon :4502。移除 bridge/、PTY、channel。daemon 只需要扩展 WS handler 理解 NDJSON 协议。**风险：隐藏 API 无稳定承诺**
   - **路线 B (TypeScript SDK sidecar，最稳妥)**：搭建 SDK adapter sidecar，用 V2 `createSession()` + `send()`/`stream()` + hooks。需要 Node.js 运行时 + API key 认证
   - **路线 C (CLI stream-json stdio，Rust 原生)**：Rust spawn claude 子进程，stdin/stdout NDJSON。无 Node.js 依赖，但未文档化协议

3. **长期**：关注 Agent Teams 稳定化 + `--sdk-url` 是否会被文档化/稳定化

### 认证模型决策

这是迁移的关键决策点：

| 方案 | 认证方式 | 计费模型 |
|------|---------|---------|
| Channel (当前) | claude.ai 订阅 | 包月/包年 |
| Agent SDK | API key | 按 token 计费 |
| CLI stream-json | API key 或 claude.ai | 灵活 |

如果用户群体是 claude.ai 订阅用户，**CLI stream-json (方案 4b)** 可能比 SDK 更合适，因为它保留 claude.ai auth 支持。

Channel 不需要紧急替换，但 SDK/stream-json 方案在每个维度都更优。迁移的主要工作量在"sidecar/子进程与 Rust daemon 的通信桥接"，这和当前 bridge 的工作量级相当，但换来的是更稳定的集成方式。
