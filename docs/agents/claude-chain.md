# Claude 链路修复记录

> 每次修复 Claude 链路时必须更新本文档。未修复的问题也必须记录。

## 官方文档参考

Claude Code MCP: https://docs.anthropic.com/en/docs/claude-code

## 当前实现状态

### 已实现

- MCP stdio 协议（bridge sidecar）
- Claude Channel preview 模式（`--dangerously-load-development-channels server:agentbridge`）
- `reply(chat_id, text)` tool
- Channel notification for inbound messages
- Permission request/verdict relay
- Bridge pre-init message buffering

### 已知限制

- 依赖 Claude Code >= 2.1.80 的 channel preview 功能
- channel preview 是实验性功能，未来可能变更
- 当前只有 `reply` 一个 tool，无文件操作或代码执行 tool
- 不支持 `--agent --agents` 角色注入

## 修复记录

### 2026-03-25: 初始审计

- [已修复] bridge pre-init 消息丢失 — 添加本地缓冲 + 回放
- [已修复] stdout 写失败静默丢消息 — 写失败时 break MCP 循环
- [已修复] push_tx 死通道检测 — send 失败时退出
- [已修复] 重连反压级联 — 退避期间 drain reply_rx
- [已修复] shell 注入风险 — 非 macOS 用 Command::new
