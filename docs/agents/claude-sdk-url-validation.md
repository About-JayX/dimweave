# Claude --sdk-url / stream-json 协议验证记录（2026-04-01）

## 验证结论

| 方案 | 结果 | 原因 |
|------|------|------|
| `--sdk-url ws://...` | **✅ 可用** | 需要精确 bridge 环境变量配置 |
| `--output-format stream-json` (stdio) | **✅ 可用** | 完整 NDJSON 消息流，无额外依赖 |

**两个方案都可用。`--sdk-url` 需要设置 `CLAUDE_CODE_ENVIRONMENT_KIND=bridge` + dummy token + `POST_FOR_SESSION_INGRESS_V2=1`。**

---

## --sdk-url 深入逆向分析

### Token 获取链

```javascript
// ED() - token 获取优先级
function ED() {
  // 1. 环境变量
  let q = process.env.CLAUDE_CODE_SESSION_ACCESS_TOKEN;
  if (q) return q;
  // 2. 文件回退 (CLAUDE_SESSION_INGRESS_TOKEN_FILE 或 ~/.claude/remote/.session_ingress_token)
  return HJz();
}

// BN6() - 根据 token 类型构造 auth header
function BN6() {
  let q = ED();
  if (!q) return {};
  if (q.startsWith("sk-ant-sid")) {
    // claude.ai session cookie
    return { Cookie: `sessionKey=${q}` };
  }
  // JWT from CCR
  return { Authorization: `Bearer ${q}` };
}
```

**发现：** `sk-ant-sid` 开头的 token 是 claude.ai session cookie。任意非空字符串都可以通过 token 检查。

### 传输层选择逻辑

```javascript
function Qq5(url, headers, sessionId, refreshHeaders) {
  if (process.env.CLAUDE_CODE_USE_CCR_V2) 
    return new lJ6(...)  // SSE (CCR v2)
  if (url.protocol === "ws:" || url.protocol === "wss:") {
    if (process.env.CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2)
      return new h48(...)  // 混合: WS收 + HTTP POST发
    return new L48(...)    // 纯 WS
  }
}
```

### 两个传输层的致命问题

#### L48 (纯 WS) — 没有 write() 方法！

```javascript
class L48 {
  // 只有 sendLine()，没有 write()
  sendLine(q) { this.ws.send(q); }
  // Gn8 调用 this.transport.write(q) → TypeError/静默失败
}
```

**测试结果：** Claude 连接 WS、调用 API 成功，但回复无法写回（write 方法不存在）。

#### h48 (混合模式) — POST 工作但初始化卡住

```javascript
class h48 extends L48 {
  // 有 write() 方法，通过 HTTP POST 发送
  async write(q) {
    await this.uploader.enqueue([q]);
  }
  // POST URL = ws→http + pathname.replace("/ws/","/session/") + "/events"
  // ws://127.0.0.1:4599 → POST http://127.0.0.1:4599/events
}
```

**测试结果：** hook 事件成功 POST 到服务器，但之后初始化过程卡住，永远不到 API 调用。

### 完整测试矩阵

| 模式 | Token | -p | POST_V2 | WS 连接 | Hook POST | API 调用 | 回写 |
|------|-------|-----|---------|---------|-----------|---------|------|
| L48 纯 WS | 无 | yes | 无 | ✅ | N/A | ✅ | ❌ (no write) |
| L48 纯 WS | dummy | yes | 无 | ✅ | N/A | ✅ | ❌ (no write) |
| h48 混合 | dummy | yes | =1 | ✅ | ✅ | ❌ (卡住) | N/A |
| h48 混合 | dummy | 无(WS送) | =1 | ✅ | ✅ | ❌ (卡住) | N/A |

### 结论

### ✅ 成功配置（精确复制 bridge 环境）

```bash
CLAUDE_CODE_OAUTH_TOKEN="" \
CLAUDE_CODE_ENVIRONMENT_KIND="bridge" \
CLAUDE_CODE_SESSION_ACCESS_TOKEN="any-non-empty-string" \
CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2="1" \
  claude \
  --print \
  --sdk-url ws://127.0.0.1:4599 \
  --session-id <uuid> \
  --input-format stream-json \
  --output-format stream-json \
  --replay-user-messages \
  --strict-mcp-config '{}'
```

**结果：** 完整通信链路打通！
- 服务器通过 WS 发送 user message → Claude 接收处理
- Claude 通过 HTTP POST `/events` 回传所有事件（init, assistant, result）
- Claude 同时在 stdout 输出完整 NDJSON（bridge 模式特性）
- `SESSION_ACCESS_TOKEN` 可以是任意非空字符串（dummy token）
- claude.ai OAuth 认证正常工作（不需要 API key）

**通信协议：**
- 入站（daemon → Claude）：WS 发送 NDJSON `{"type":"user",...}\n`
- 出站（Claude → daemon）：HTTP POST `http://host:port/events` body=`{"events":[...]}`
- 出站镜像：stdout NDJSON（可选，bridge 模式自动启用）

**关键要素：**
1. `CLAUDE_CODE_ENVIRONMENT_KIND=bridge` — 必须，切换到 remote-control 模式
2. `CLAUDE_CODE_POST_FOR_SESSION_INGRESS_V2=1` — 必须，启用 h48 混合传输
3. `CLAUDE_CODE_SESSION_ACCESS_TOKEN` — 必须非空，dummy 即可
4. `CLAUDE_CODE_OAUTH_TOKEN=""` — 推荐，清除避免冲突
5. `--replay-user-messages` — 推荐，回显输入用于确认

### 之前失败的原因

| 测试 | 失败原因 |
|------|---------|
| 纯 WS (L48) | `write()` 方法不存在，输出无法写回 |
| h48 + 无 `ENVIRONMENT_KIND` | 初始化路径不对，不进入 remote-control 模式，卡在 plugin 加载 |
| h48 + `ENVIRONMENT_KIND=bridge` + `--bare` | `--bare` 跳过 OAuth，无法认证 API |
| h48 + `ENVIRONMENT_KIND=bridge` + OAuth | ✅ 成功 |

---

## stdio stream-json 协议格式

### 测试命令
```bash
claude -p "say hello" --output-format stream-json --verbose
```

### 消息流（stdout NDJSON）

每行一个 JSON 对象，按顺序产生：

#### 1. `system/hook_started` + `system/hook_response`（hook 事件，可忽略）
```json
{"type":"system","subtype":"hook_started","hook_id":"...","hook_name":"SessionStart:startup",...}
{"type":"system","subtype":"hook_response","hook_id":"...","output":"...",...}
```

#### 2. `system/init`（会话初始化，关键信息源）
```json
{
  "type": "system",
  "subtype": "init",
  "cwd": "/Users/jason/floder/agent-bridge",
  "session_id": "b83b1d47-...",
  "tools": ["Task","Bash","Edit","Read",...,"mcp__agentnexus__reply"],
  "mcp_servers": [{"name":"agentnexus","status":"connected"}],
  "model": "claude-opus-4-6[1m]",
  "permissionMode": "bypassPermissions",
  "agents": ["general-purpose","Explore","Plan",...],
  "skills": [...],
  "plugins": [...]
}
```

#### 3. `assistant`（Claude 回复）
```json
{
  "type": "assistant",
  "message": {
    "model": "claude-opus-4-6",
    "role": "assistant",
    "content": [{"type":"text","text":"hello"}],
    "stop_reason": null,
    "usage": {"input_tokens":3,"output_tokens":1,...}
  },
  "parent_tool_use_id": null,
  "session_id": "b83b1d47-..."
}
```

#### 4. `rate_limit_event`
```json
{"type":"rate_limit_event","rate_limit_info":{"status":"allowed",...}}
```

#### 5. `result`（最终结果）
```json
{
  "type": "result",
  "subtype": "success",
  "result": "hello",
  "duration_ms": 3474,
  "num_turns": 1,
  "total_cost_usd": 0.1093,
  "session_id": "b83b1d47-...",
  "stop_reason": "end_turn"
}
```

### 双向通信（stream-json input）

用 `--input-format stream-json` 启用 stdin 输入：

```bash
claude -p --input-format stream-json --output-format stream-json --verbose
```

发送用户消息到 stdin：
```json
{"type":"user","session_id":"","message":{"role":"user","content":[{"type":"text","text":"hello"}]},"parent_tool_use_id":null}
```

Permission 响应到 stdin：
```json
{"type":"control_response","subtype":"can_use_tool","allow":true}
```

### 关键参数组合

```bash
claude -p \
  --input-format stream-json \
  --output-format stream-json \
  --verbose \
  --session-id <uuid> \        # 新 session
  --resume <uuid> \            # 恢复 session（二选一）
  --agent <role> \             # 角色 system prompt
  --model <model> \            # 模型选择
  --mcp-config <path> \        # MCP 服务器配置
  --bare                       # 跳过 hooks/skills/plugins 加载（更快启动）
```

---

## 完整协议规格（逆向工程）

### 传输层（--sdk-url 混合模式）

| 方向 | 传输 | 格式 |
|------|------|------|
| daemon → Claude | WebSocket | NDJSON（每条 `\n` 结尾） |
| Claude → daemon | HTTP POST `/events` | `{"events": [...]}` JSON array |
| Claude → stdout | NDJSON | 镜像输出（bridge 模式自动启用） |

POST URL 推导规则：
- `ws://host:port/` → POST `http://host:port/events`
- `ws://host:port/ws/` → POST `http://host:port/session/events`（`/ws/` 被替换为 `/session/`）
- POST Headers: `Authorization: Bearer <SESSION_ACCESS_TOKEN>`, `Content-Type: application/json`

### 消息类型完整列表

| type | 方向 | 用途 |
|------|------|------|
| `system` | out | 系统事件（init, hook_started, hook_response, compact_boundary 等） |
| `assistant` | out | Claude 回复（text blocks, tool_use blocks） |
| `user` | in | 用户消息 / tool results |
| `result` | out | 会话结束（success/error + 统计） |
| `control_request` | out | 权限请求等（can_use_tool, initialize, set_model 等） |
| `control_response` | in | 权限回复（allow/deny） |
| `control_cancel_request` | out | 取消 pending 权限请求 |
| `stream_event` | out | 流式 token delta（h48 会批量 POST） |
| `keep_alive` | 双向 | 心跳 |
| `rate_limit_event` | out | 速率限制信息 |
| `prompt_suggestion` | out | 建议的下一条 prompt |
| `update_environment_variables` | in | 动态更新环境变量 |

in = daemon → Claude (via WS), out = Claude → daemon (via HTTP POST + stdout)

### Permission 请求（control_request）

Claude 想调用工具时通过 POST 发出：
```json
{
  "type": "control_request",
  "request_id": "uuid-string",
  "request": {
    "subtype": "can_use_tool",
    "tool_name": "Bash",
    "input": {"command": "ls -la", "description": "List files"},
    "tool_use_id": "toolu_xxx",
    "description": "List files in current directory",
    "permission_suggestions": [],
    "blocked_path": null
  }
}
```

### Permission 回复（control_response）

daemon 通过 WS 发回：

**批准：**
```json
{
  "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": "same-uuid",
    "response": {
      "behavior": "allow",
      "updatedInput": {},
      "updatedPermissions": []
    }
  }
}
```

**拒绝：**
```json
{
  "type": "control_response",
  "response": {
    "subtype": "success",
    "request_id": "same-uuid",
    "response": {
      "behavior": "deny",
      "message": "User denied this action"
    }
  }
}
```

注意：deny 也用 `subtype: "success"`，通过 `behavior` 字段区分。

### 其他 control_request 子类型

| subtype | 用途 | daemon 必须处理 |
|---------|------|----------------|
| `can_use_tool` | 工具权限审批 | ✅ 核心 |
| `initialize` | 初始化握手 | ✅ 需回 success（含 commands, output_style） |
| `set_permission_mode` | 切换权限模式 | 可选 |
| `set_model` | 切换模型 | 可选 |
| `interrupt` | 中断当前工作 | 可选 |

### system/init 消息

```json
{
  "type": "system",
  "subtype": "init",
  "session_id": "uuid",
  "cwd": "/path/to/project",
  "model": "claude-opus-4-6[1m]",
  "tools": ["Bash", "Read", "Edit", ...],
  "mcp_servers": [],
  "permissionMode": "bypassPermissions",
  "agents": [...],
  "skills": [...],
  "plugins": [...],
  "claude_code_version": "2.1.89"
}
```

### assistant 消息

```json
{
  "type": "assistant",
  "message": {
    "model": "claude-opus-4-6",
    "role": "assistant",
    "content": [
      {"type": "text", "text": "hello"},
      {"type": "tool_use", "id": "toolu_xxx", "name": "Bash", "input": {"command": "ls"}}
    ],
    "stop_reason": "end_turn",
    "usage": {"input_tokens": 100, "output_tokens": 50}
  },
  "session_id": "uuid",
  "parent_tool_use_id": null
}
```

### user 消息（daemon → Claude via WS）

```json
{"type":"user","session_id":"","message":{"role":"user","content":[{"type":"text","text":"hello"}]},"parent_tool_use_id":null}\n
```
**必须以 `\n` 结尾。**

### result 消息

```json
{
  "type": "result",
  "subtype": "success|error_max_turns|error_max_budget_usd|error_during_execution",
  "result": "response text",
  "session_id": "uuid",
  "duration_ms": 3474,
  "num_turns": 1,
  "total_cost_usd": 0.1093,
  "stop_reason": "end_turn",
  "usage": {...}
}
```

### 所有 system subtype 值

来源于源码分析的完整列表：
`init`, `hook_started`, `hook_response`, `hook_progress`, `hook_callback`,
`compact_boundary`, `bridge_status`, `bridge_state`, `session_state_changed`,
`api_error`, `api_retry`, `status`, `informational`, `error`,
`task_started`, `task_progress`, `task_notification`,
`agents_killed`, `stop_hook_summary`, `memory_saved`,
`file_snapshot`, `turn_duration`, `local_command`,
`scheduled_task_fire`, `permission_retry`, `mcp_message`,
`elicitation`, `elicitation_complete`

### daemon 实现清单

**必须实现：**
1. WS Server 接受 Claude 连接（同端口处理 WS upgrade 和 HTTP POST）
2. HTTP POST `/events` 端点接收 `{"events":[...]}` 并解析
3. WS 发送 NDJSON user messages（`\n` 结尾）
4. Permission relay: `control_request(can_use_tool)` → GUI → `control_response` via WS
5. Initialize 握手: `control_request(initialize)` → respond with commands/settings
6. 监听 `result` 事件标记会话结束

**建议实现：**
7. 心跳检测（`keep_alive` 双向）
8. 解析 `assistant` 消息提取 text/tool_use 用于 UI 展示
9. 解析 `stream_event` 用于实时流式展示
10. 处理 `rate_limit_event` 用于用量统计
11. 支持 `update_environment_variables` 动态更新

**可忽略：**
12. `prompt_suggestion`（除非需要建议下一步功能）
13. `control_cancel_request`（除非需要取消 pending permission）
