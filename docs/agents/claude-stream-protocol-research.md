# Claude Stream Protocol 调研

**日期**: 2026-04-08
**来源**: Claude Code VSCode Extension 2.1.92 源码逆向 (`extension.js`, `webview/index.js`)
**目的**: 搞清楚 Claude SDK streaming 的完整事件协议，为 Dimweave stream UI 升级提供依据

## 1. `--include-partial-messages` Flag

### 来源证据

Extension 启动 Claude 时的代码（`extension.js` ~line 795）:
```javascript
D = {
  ...,
  includePartialMessages: !N6.env.remoteName,  // 本地=true, remote=false
  ...
}
```

初始化时传入 CLI（`extension.js` ~line 153）:
```javascript
if (X) a.push("--include-partial-messages");  // X = includePartialMessages
```

### 效果

- **开启时**: Claude 在每个 content block 完成后额外发送 `"assistant"` 事件，包含完整的累积消息
- **关闭时**: 只有 `stream_event`（低层 delta）和最终 `"result"`

### 事件序列对比

| Flag ON | Flag OFF |
|---------|----------|
| `stream_event` (deltas) | `stream_event` (deltas) |
| `assistant` (partial, per block) | _(无)_ |
| `result` (final) | `result` (final) |

## 2. 完整事件类型清单

### 顶层事件类型（SDK stdout NDJSON）

| 类型 | 说明 | 我们当前是否处理 |
|------|------|-----------------|
| `system` | session init/state | 是 |
| `user` | echo 用户输入 | 忽略 |
| `assistant` | 累积消息快照（需要 `--include-partial-messages`） | **否** |
| `result` | turn 结束，含最终文本 | 是 |
| `stream_event` | 底层 Anthropic API 流事件 | 部分（只看 text_delta） |
| `control_request` | permission/init 请求 | 是 |
| `control_response` | 对 control_request 的回复 | 是 |
| `control_cancel_request` | 取消请求 | 忽略 |
| `keep_alive` | 心跳 | 忽略 |
| `rate_limit_event` | 限流状态 | 是（只 log） |
| `prompt_suggestion` | 建议 | 是（只 log） |
| `auth_status` | 认证状态 | 是（只 log） |

### `stream_event.event` 子类型

| 子类型 | 说明 | 我们当前是否处理 |
|--------|------|-----------------|
| `message_start` | assistant 消息开始 | **否** |
| `message_delta` | 消息级更新（stop_reason 等） | **否** |
| `message_stop` | 消息结束 | **否** |
| `content_block_start` | 新 content block 开始 | 部分（只 text） |
| `content_block_delta` | block 内增量 | 部分（只 text_delta） |
| `content_block_stop` | block 结束 | **否** |
| `ping` | 心跳 | 忽略 |
| `error` | 流错误 | **否** |

### `content_block_start` 的 block 类型

| type | 说明 | 我们当前是否处理 |
|------|------|-----------------|
| `"text"` | 正文文本 | 是（发 ThinkingStarted + preview） |
| `"thinking"` | 扩展思考 | **否** |
| `"tool_use"` | 工具调用 | **否** |

### `content_block_delta` 的 delta 类型

| delta.type | 字段 | 说明 |
|------------|------|------|
| `text_delta` | `delta.text` | 正文文本增量 |
| `thinking_delta` | `delta.thinking` | 思考文本增量 |
| `input_json_delta` | `delta.text` | 工具输入 JSON 增量 |

## 3. 消息完整生命周期

```
用户发送消息
    ↓
message_start: { id, role: "assistant", content: [] }
    ↓
━━ Block 0: Thinking ━━━━━━━━━━━━━━━━━━━━━
content_block_start: { index: 0, type: "thinking" }
content_block_delta: { delta: { type: "thinking_delta", thinking: "分析问题..." } }
content_block_delta: { delta: { type: "thinking_delta", thinking: "考虑方案..." } }
content_block_stop: {}
    ↓
[如果 --include-partial-messages]
assistant (partial): { content: [{ type: "thinking", thinking: "完整思考..." }] }
    ↓
━━ Block 1: Text ━━━━━━━━━━━━━━━━━━━━━━━━
content_block_start: { index: 1, type: "text" }
content_block_delta: { delta: { type: "text_delta", text: "这是" } }
content_block_delta: { delta: { type: "text_delta", text: "回答" } }
content_block_stop: {}
    ↓
[如果 --include-partial-messages]
assistant (partial): { content: [thinking, { type: "text", text: "这是回答" }] }
    ↓
━━ Block 2: Tool Use ━━━━━━━━━━━━━━━━━━━━
content_block_start: { index: 2, type: "tool_use", id: "call_123", name: "Edit" }
content_block_delta: { delta: { type: "input_json_delta", text: "{\"path\":" } }
content_block_delta: { delta: { type: "input_json_delta", text: " \"file.ts\"}" } }
content_block_stop: {}
    ↓
[如果 --include-partial-messages]
assistant (partial): { content: [thinking, text, { type: "tool_use", ... }] }
    ↓
message_delta: { delta: { stop_reason: "tool_use" } }
message_stop: {}
    ↓
assistant (final): { content: [所有 blocks], usage: { input_tokens, output_tokens }, stop_reason }
    ↓
result: { type: "result", subtype: "success", result: "..." }
```

## 4. Claude Code VSCode 的渲染策略

### Extension 层（extension.js）

1. 使用 `uh` transport 类启动 Claude 进程
2. `readMessages()` 从 stdout 读取 NDJSON
3. 所有事件（除 `keep_alive` 和 `post_turn_summary`）都通过 `inputStream.enqueue(K)` 发给 webview
4. webview 通过 `postMessage` 接收

### Webview 层（webview/index.js）

1. 收到 `content_block_start` → 创建新的 content block 条目
2. 收到 `content_block_delta` → 追加文本到当前 block
3. 按 block type 区分渲染：
   - `thinking` → 可折叠的思考区域
   - `text` → 主文本区域，实时流式渲染
   - `tool_use` → 工具调用展示（参数预览）
4. 收到 `assistant` (partial) → 更新 token 计数/usage 信息
5. 收到 `result` → 标记 turn 完成

### 关键差异：我们 vs Claude Code

| 方面 | Claude Code | 我们当前 |
|------|------------|---------|
| `--include-partial-messages` | 是 | **否** |
| thinking block 展示 | 可折叠区域 | 只显示 "thinking..." |
| text 流式渲染 | 逐字符追加 | 50ms batch 合并后显示 preview |
| tool_use 展示 | 参数预览 | 不展示 |
| 多 block 支持 | 是（thinking + text + tool_use 并存） | 否（只识别 text block） |
| content_block_stop | 追踪 block 完成 | 不处理 |
| message_start/stop | 追踪消息生命周期 | 不处理 |

## 5. 我们需要改什么

### 必须改（最小可行）

1. **启动参数**: 加 `--include-partial-messages`
2. **stream_event 处理**: 区分 `thinking` / `text` / `tool_use` block
3. **前端 store**: 从单一 `previewText` 改为多 block 结构
4. **UI 渲染**: 流式 text 直接渲染到气泡，不再只显示 "thinking..."

### 可选改进

1. thinking block 可折叠展示
2. tool_use block 参数预览
3. token usage 实时展示
4. `content_block_stop` 追踪 block 完成状态
5. `message_start` / `message_stop` 追踪消息生命周期边界

## 6. `assistant` 事件结构（--include-partial-messages 开启时）

```json
{
  "type": "assistant",
  "message": {
    "id": "msg_abc123",
    "role": "assistant",
    "content": [
      { "type": "thinking", "thinking": "完整的思考文本..." },
      { "type": "text", "text": "完整的回答文本..." },
      { "type": "tool_use", "id": "call_123", "name": "Edit", "input": { "path": "..." } }
    ],
    "usage": {
      "input_tokens": 245,
      "output_tokens": 1089,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 0
    },
    "stop_reason": "tool_use"
  }
}
```

关键特征：
- 是**完整累积快照**，不是增量 delta
- 每个 block 完成后发一次，content 数组逐步增长
- `usage` 是累积值
- `stop_reason` 在最后一次出现

## 7. 风险和注意事项

- `--include-partial-messages` 会增加数据量，remote 环境下 Claude Code 主动关闭
- `assistant` 事件和 `stream_event` 是并行的，不要重复渲染
- `thinking` block 可能很长，需要考虑性能
- tool_use 的 `input_json_delta` 是流式 JSON 碎片，不能中途 parse
- 当前的去重状态机需要和新的 streaming 逻辑兼容
