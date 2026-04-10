# Prompt 行数豁免审计设计

## 摘要

当前工作树里已经存在一组未提交的 prompt / protocol 改动，集中在 `role_config` 和 `bridge` 的协议提示词定义。审计结果显示，这组改动的目标测试全部通过，但其中 `src-tauri/src/daemon/role_config/role_protocol.rs` 从 `HEAD` 的 167 行增长到 347 行，触发了仓库里“每个源码文件最多 200 行”的硬性检查。

用户明确要求：**prompt 部分不计入 line 检查，并把这条规则写进文档；在此基础上，如果审计无其他问题，就提交当前改动并补 CM。**

因此，本次设计不再把 prompt 文件长度视为阻塞项，而是做两件事：

1. 在 `CLAUDE.md` 里把 prompt / 协议定义文件从 200 行限制中明确豁免出来；
2. 在保持现有 prompt 改动内容不变的前提下，完成审计、验证、提交与 CM 回填。

## 目标

- 让仓库文档明确声明：prompt / 协议定义源码文件不受 200 行限制。
- 仅对当前已存在的 prompt / protocol 改动做审计接受，不再额外扩展行为。
- 在验证通过后，把当前改动以受控 commit 形式落盘，并把 commit 记录回填到 plan。

## 范围

### 包含

- 修改 `CLAUDE.md` 中的源码文件长度约束，增加 prompt / 协议定义文件豁免说明。
- 审计并提交以下已有未提交改动：
  - `bridge/src/mcp_protocol.rs`
  - `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
  - `src-tauri/src/daemon/role_config/role_protocol.rs`
  - `src-tauri/src/daemon/role_config/roles_tests.rs`
- 补充本次工作的 spec / plan / CM 记录。

### 不包含

- 不重写 prompt 内容本身。
- 不做 `role_protocol.rs` 的模块拆分。
- 不扩展到其他非 prompt 源码文件的行数豁免。
- 不修改 Telegram、routing、Feishu 或前端逻辑。

## 相关记忆

### 直接相关提交

- `3e2c95a7` — 加入 `report_telegram` 协议字段
- `6a6ad203` — 定义 `report_telegram` prompt 合同
- `f42030b6` — Codex output schema 强制要求 `report_telegram`
- `fb7f8db9` — 强化 lead communication prompt contract
- `b9956525` — 抽出共享 role prompt protocol

### 相关计划 / 文档

- `docs/superpowers/plans/2026-04-09-report-telegram.md`
- `docs/agents/claude-chain.md`
- `docs/agents/codex-chain.md`
- `CLAUDE.md`

### 约束继承

- 这是一项 non-greenfield prompt/protocol 审计工作，必须保留 memory evidence。
- 只接受当前用户明确批准的规则变化：**prompt 不计入 line 检查**。
- 这次接受应尽量最小化，仅把豁免写入文档并提交现有改动，不顺带继续扩 scope。

## 方案对比

### 方案 A：继续把 prompt 文件纳入 200 行限制

- 优点：规则简单统一
- 缺点：与用户刚刚给出的明确指令冲突；会继续阻塞当前 prompt 改动提交

### 方案 B：仅豁免当前 `role_protocol.rs`

- 优点：范围最小
- 缺点：规则表达过窄；未来 `bridge/src/mcp_protocol.rs` 或其他 prompt 定义文件仍会重复触发相同争议

### 方案 C（推荐）：豁免 prompt / 协议定义源码文件，并写入 `CLAUDE.md`

- 优点：与用户指令一致，规则清晰；只放开 prompt / 协议定义文件，不扩展到普通业务代码
- 缺点：需要在文档里明确举例，避免被滥用

## 推荐设计

采用 **方案 C**。

在 `CLAUDE.md` 的两处行数约束说明中，把规则改成：

- 默认仍然要求普通源码文件 <= 200 行；
- 但**承载系统提示词、角色协议、channel 指令等长字符串定义的源码文件**不受此限制；
- 文档里明确举例：`src-tauri/src/daemon/role_config/**`、`bridge/src/mcp_protocol.rs`。

这样既满足用户要求，也把豁免边界控制在 prompt / 协议定义文件，而不是放大成“所有源码都可超长”。

## 验证策略

对当前 dirty 改动做受控验证：

- `cargo build --manifest-path bridge/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::`
- `cargo test --manifest-path bridge/Cargo.toml mcp_protocol`
- `git diff --check`

验收标准：

- `CLAUDE.md` 明确写入 prompt 行数豁免；
- 以上验证全部通过；
- 当前 prompt/protocol 变更无新增审计阻塞项；
- commit hash 回填到 plan 的 CM Memory。
