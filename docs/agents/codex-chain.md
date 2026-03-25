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

#### [已修复] dynamicTools schema 字段名错误

**问题:** 官方文档使用 `parameters` 作为 tool schema 字段名：
```json
{ "name": "my_tool", "parameters": { "type": "object", ... } }
```
当前实现使用 `inputSchema`，导致工具可能无法被正确注册。

**修复:** 将 `inputSchema` 改为 `parameters`。

#### [已修复] sandbox 值格式错误

**问题:** 官方文档使用 camelCase: `workspaceWrite`, `readOnly`。
当前实现使用 kebab-case: `workspace-write`, `read-only`。

**修复:** `roles.rs` 中的 `sandbox_mode` 改为 camelCase 格式，同时更新 `lifecycle.rs` 和 `session_manager.rs` 中的相关引用。

#### [未修复] `--config` CLI flags 格式未验证

**问题:** `lifecycle.rs` 通过 `--config sandbox_mode="workspace-write"` 传递配置，但 `--config` flag 的精确格式未在官方文档中明确。可能需要验证这个格式是否正确。

**状态:** 需要运行时测试验证。

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
