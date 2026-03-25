# Codex 链路修复记录

> 每次修复 Codex 链路时必须更新本文档。未修复的问题也必须记录。

## 官方文档参考

https://developers.openai.com/codex/app-server

## 协议对照与修复记录

### 2026-03-25: 初始协议审计

对照官方文档 vs 当前 `session.rs` 实现，发现以下不一致：

#### [已修复] 缺少 `initialized` 通知

**问题:** 官方文档要求 initialize 响应收到后，客户端必须发送 `{ "method": "initialized", "params": {} }` 通知。当前实现没有发送此通知，导致 app-server 不会继续处理后续请求。

**修复:** 在 `session.rs` 收到 initialize response 后立即发送 `initialized` 通知。

**影响:** 这是 Codex 无法连接的根本原因。

#### [已修复] dynamicTools schema 字段名 — 文档与实现不一致

**问题:** 官方文档写 `parameters`，但 Codex CLI 实际要求 `inputSchema`。
使用 `parameters` 时报错：`Invalid request: missing field 'inputSchema'`。

**验证:** `bun` 脚本直接测试 app-server，确认 `inputSchema` 正确。

**修复:** 保持 `inputSchema`（之前错误地改成了 `parameters`，已改回）。

**教训:** 官方文档不可信，必须运行时测试验证。

#### [已修复] sandbox 值格式 — 两套上下文不同格式

**问题:** Codex 有两个不同接口使用 sandbox 值：
- **CLI `--config` / `config.toml`**: 要求 kebab-case (`workspace-write`, `read-only`)
- **JSON-RPC `thread/start` params.sandbox**: 要求 camelCase (`workspaceWrite`, `readOnly`)

**第一次修复(错误):** 把 `roles.rs` 全部改成 camelCase，导致 config.toml 写入无效值，
app-server 启动时报 `unknown variant 'workspaceWrite', expected 'workspace-write'`。

**第二次修复:** `roles.rs` 保持 kebab-case，`session.rs` 转 camelCase → 仍然失败。

**第三次修复(正确):** `thread/start` 的 `sandbox` 参数也要求 kebab-case！
camelCase `workspaceWrite` 报错：`unknown variant 'workspaceWrite', expected 'workspace-write'`

**结论:** Codex CLI 实现 **全部使用 kebab-case**，与官方文档的 camelCase 描述完全相反。
`roles.rs` 和 `session.rs` 统一使用 kebab-case 即可，无需转换。

**运行时验证:** `inputSchema` + kebab-case `workspace-write` → thread/start 成功。

#### [已修复] `--config` CLI flags 格式

**验证结果:** `--config sandbox_mode="workspace-write"` 格式正确。

#### [待确认] `settings.developer_instructions` 有效性

**问题:** 当前实现把 `developer_instructions` 放在 `params.settings.developer_instructions`。官方文档未明确此字段路径。文档提到 `personality` 和 `collaborationMode` 相关的 `settings`。

**状态:** 保持当前实现，运行时测试确认。

#### [未修复] tool response 格式

**问题:** 当前 handler.rs 回复格式：
```json
{
  "id": id,
  "result": {
    "contentItems": [{ "type": "inputText", "text": "..." }],
    "success": true
  }
}
```
官方文档 dynamic tool call 流程描述的回复结构需要确认是否与此匹配。

**状态:** 需要运行时测试验证。

## 当前已知限制

- 端口 4500 固定，不可配置
- 不处理 `turn/completed` 通知
- 不处理 `item/agentMessage/delta` 流式文本
- 不处理 `item/commandExecution/requestApproval` 审批
- 不处理 `-32001` 过载错误重试
