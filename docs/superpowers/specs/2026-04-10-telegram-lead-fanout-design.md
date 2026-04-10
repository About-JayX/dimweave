# Lead Telegram 全量转发设计

## 摘要

当前 Telegram 通知链路依赖一个显式协议字段 `report_telegram`。只有当消息满足：

- `from == "lead"`
- `report_telegram == true`

时，daemon 才会把消息扇出到 Telegram。

用户现在要把这套“显式打标”协议彻底移除，改成更简单的规则：

> **只要消息来自 `lead`，就发到 Telegram。**

这意味着 Telegram fan-out 不再是 prompt / tool / schema 层的显式意图，而是 daemon 里的固定路由策略。`report_telegram` 需要从端到端协议、提示词、schema、测试中移除。保留的只应是发送能力本身的运行时门槛，例如：

- Telegram 功能已启用
- runtime 在线
- 已配对 `chat_id`

这些不是业务判断，而是“能不能发出去”的基础条件。

## 产品目标

- 删除 `report_telegram` 协议字段，减少模型输出犯错点。
- 所有 `lead` 发出的消息都自动同步到 Telegram。
- 非 `lead` 发出的消息不进入 Telegram。
- 保持现有 Telegram runtime、pairing、HTML 格式化与消息分块逻辑不变。

## 范围

### 包含

- 从消息协议中移除 `report_telegram`
- 从 Claude `reply()` tool schema 中移除 `report_telegram`
- 从 Codex output schema 中移除 `report_telegram`
- 从 role prompt / Claude channel instructions 中移除 `report_telegram` 指引
- 把 Telegram 路由 gate 改成只判断 `msg.from == "lead"`
- 更新相关测试

### 不包含

- 不改 Telegram bot 配置、配对、轮询与发送实现
- 不改 Telegram HTML 消息模板
- 不改消息路由目标（user / coder / lead）的原有语义
- 不改 Feishu / Task / UI 逻辑

## 相关记忆

### 直接相关提交

- `3e2c95a7` — 为消息协议加入 `report_telegram`
- `6a6ad203` — 把 `report_telegram` 写进 prompt contract
- `fa5f16e4` — 把 `report_telegram` 接入 Telegram fan-out
- `d5e76ef5` — 增加 notifications_enabled gate
- `85ea11ad` — ingress 处强制只有 lead 可带 `report_telegram`
- `10209c36` — 审计并接受当前 prompt/protocol 更新

### 相关计划

- `docs/superpowers/plans/2026-04-09-report-telegram.md`
- `docs/superpowers/plans/2026-04-09-report-telegram-route-unification.md`
- `docs/superpowers/plans/2026-04-10-prompt-line-limit-exemption-audit.md`

### 约束继承

- 当前链路已经证明 Telegram 真正的发信门槛在 daemon runtime，而不是 prompt 自身。
- `report_telegram` 已出过真实问题：最终汇报漏打字段就不会推送。
- 用户已经明确批准：删除该字段，改成 lead 全量推送。

## 方案对比

### 方案 A：保留字段，但由系统自动补 true

- 优点：改动小
- 缺点：协议里仍保留一个“看起来重要但实际上不该由模型控制”的字段，不符合用户要求

### 方案 B：忽略字段，但继续保留在 schema / prompt 里

- 优点：兼容旧协议
- 缺点：死字段会继续误导模型和维护者

### 方案 C（推荐）：彻底移除字段，路由层固定按 `from == "lead"` fan-out

- 优点：规则最简单，最符合用户要求
- 缺点：lead 发给 coder 的内部消息也会同步到 Telegram，但这是用户明确接受的行为

## 推荐设计

采用 **方案 C**。

### 1. Telegram 路由规则收口

把 `src-tauri/src/telegram/report.rs::should_send_telegram_report()` 改成：

- 只检查 `msg.from == "lead"`

并保留 `routing_dispatch.rs` 里的运行时发送能力检查：

- `telegram_notifications_enabled`
- `telegram_outbound_tx`
- `telegram_paired_chat_id`

### 2. 协议字段端到端删除

从以下位置移除 `report_telegram`：

- `BridgeMessage`（Rust / bridge / TS，如有暴露）
- bridge reply tool schema
- Codex output schema
- Claude / Codex prompt 文本
- 解析器与 handler 中的读写逻辑
- 对应测试

### 3. 保持格式化逻辑不变

Telegram report card 的 HTML 模板、chunk 拆分、task title/task id 展示无需变更。我们只改“是否发送”的决策，不改“如何发送”。

## 行为变化

### 改动前

- lead 只有显式携带 `report_telegram=true` 才会发 Telegram
- lead 漏打该字段时，即使是 final review / final acceptance，也不会推送

### 改动后

- lead 的所有消息都会发 Telegram
- coder / user 的消息不会发 Telegram
- 模型不再需要记忆或输出 `report_telegram`

## 文件计划

### 核心协议 / schema / prompt

- `src-tauri/src/daemon/types.rs`
- `bridge/src/types.rs`
- `src/types.ts`（如果当前前端类型仍暴露该字段）
- `bridge/src/tools.rs`
- `bridge/src/tools_tests.rs`
- `src-tauri/src/daemon/codex/handler.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/codex/structured_output.rs`
- `src-tauri/src/daemon/codex/structured_output_tests.rs`
- `src-tauri/src/daemon/role_config/roles.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`
- `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
- `bridge/src/mcp_protocol.rs`
- `bridge/src/mcp_protocol_tests.rs`

### Telegram routing

- `src-tauri/src/telegram/report.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`（如仅注释/命名需同步）

## 验证策略

- `cargo build --manifest-path bridge/Cargo.toml`
- `cargo test --manifest-path bridge/Cargo.toml tools`
- `cargo test --manifest-path bridge/Cargo.toml mcp_protocol`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::`
- `cargo test --manifest-path src-tauri/Cargo.toml telegram`
- `cargo test --manifest-path src-tauri/Cargo.toml codex::`
- `git diff --check`

## 验收标准

- `report_telegram` 不再出现在消息协议、tool schema、output schema、prompt 指引和相关测试中
- Telegram 只按 `from == "lead"` 进行 fan-out
- 非 lead 消息不会发 Telegram
- 相关测试与构建全部通过
