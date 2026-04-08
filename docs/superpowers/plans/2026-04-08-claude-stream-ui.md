# Claude Stream UI Upgrade Plan

**日期**: 2026-04-08
**目标**: 让 Claude 的流式输出实时渲染到消息气泡，替代当前的 "thinking..." 静态提示

## 根因

1. 启动 Claude 时没有传 `--include-partial-messages`
2. `stream_event` 只处理 `text` block 的 `text_delta`，忽略了 `thinking` block
3. 前端只有一个 `previewText` 字符串，没有多 block 结构
4. UI 在 streaming 时只显示 "thinking..." 或固定 preview，不实时更新气泡

## 文件映射

### Rust 后端
- `src-tauri/src/daemon/claude_sdk/process.rs` — 加 `--include-partial-messages`
- `src-tauri/src/daemon/claude_sdk/event_handler_stream.rs` — 处理 thinking/tool_use block
- `src-tauri/src/daemon/gui.rs` — 扩展 ClaudeStreamPayload 枚举

### 前端
- `src/stores/bridge-store/types.ts` — ClaudeStreamState 多 block
- `src/stores/bridge-store/listener-payloads.ts` — ClaudeStreamPayload 类型
- `src/stores/bridge-store/stream-reducers.ts` — 处理新 payload 类型
- `src/stores/bridge-store/stream-batching.ts` — batching 逻辑适配
- `src/components/MessagePanel/ClaudeStreamIndicator.tsx` — 实时渲染流式文本

## Task 分解

### Task 1: 启动参数 + Rust stream 事件扩展
- 加 `--include-partial-messages` 到 Claude 启动命令
- 扩展 `ClaudeStreamPayload` 支持 thinking/text/tool_use block delta
- 处理 `content_block_start` 的 thinking/tool_use 类型
- 处理 `thinking_delta` 和 `input_json_delta`
- 处理 `content_block_stop` 和 `message_stop`

### Task 2: 前端 store 适配
- `ClaudeStreamState` 从单 `previewText` 改为 `{ thinking, text, toolName, blockType }`
- stream-reducers 处理新的 payload kind
- batching 逻辑适配新 block 类型

### Task 3: UI 渲染
- ClaudeStreamIndicator 按 blockType 区分展示
- thinking → 可折叠思考区域
- text → 实时流式文本（替代 "thinking..."）
- tool_use → 工具名称展示

## CM Memory

| Task | Commit | Verification |
|------|--------|--------------|
| Task 1 | `PENDING` | `cargo test` |
| Task 2 | `PENDING` | `bun test` + `bun run build` |
| Task 3 | `PENDING` | `bun run build` + 运行时验证 |
