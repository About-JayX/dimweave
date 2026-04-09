# Codex Output Schema Hotfix

**日期**: 2026-04-09
**状态**: 已修复

## 目标

修复 Codex 在显示 `active` 后首条消息立即失败的问题，并记录导致事故的提交、根因、修复方式和验证证据。

## 现象

- Codex 连接和握手成功，UI 显示 `active`
- 用户发送消息后，turn 立刻失败，前端表现为 `error`
- transcript 中只有：
  - `task_started`
  - `user_message`
  - `task_complete(last_agent_message: null)`

## 根因分析

**根因**: `src-tauri/src/daemon/role_config/roles.rs` 里的 Codex `outputSchema` 在 `properties` 中新增了 `report_telegram`，但没有同步更新 `required`。daemon 仍会在每次 `turn/start` 时发送这份 schema，于是 Codex app-server 在运行时拒绝该请求。

**错误位置**:

- schema 定义: [src-tauri/src/daemon/role_config/roles.rs](/Users/jason/floder/agent-bridge/src-tauri/src/daemon/role_config/roles.rs)
- 发送位置: [src-tauri/src/daemon/codex/session.rs](/Users/jason/floder/agent-bridge/src-tauri/src/daemon/codex/session.rs)

**服务端实际报错**:

```text
invalid_json_schema:
Invalid schema for response_format 'codex_output_schema':
In context=(), 'required' is required to be supplied and to be an array
including every key in properties. Missing 'report_telegram'.
```

## 引入提交

**问题提交**: `3e2c95a7ef2e4084a24efd53091c29702d269967`

**提交标题**: `feat: add report_telegram to message protocol`

该提交做了两件直接导致事故的事情：

1. 在 `output_schema().properties` 中新增 `report_telegram`
2. 同时新增了错误测试假设，明确把 `report_telegram` 当成 schema 级 optional，而不是仅业务语义上的默认 `false`

## 修复

### 代码修复

将 `output_schema().required` 从：

```json
["message", "send_to", "status"]
```

改为：

```json
["message", "send_to", "status", "report_telegram"]
```

### 测试修复

把错误测试：

- `output_schema_allows_optional_report_telegram_boolean`

改为：

- `output_schema_requires_report_telegram_boolean`

并同步更新 `required` 断言。

## 语义说明

这里要区分两层：

1. **业务语义**
   - `report_telegram` 缺省时，系统仍按 `false` 理解
2. **Codex outputSchema 协议语义**
   - 当前 app-server 要求 `properties` 中出现的字段必须同时出现在 `required`

因此修复不是改变业务含义，而是把 schema 形式调整为 Codex 当前接受的合法格式。agent 仍应输出：

```json
{"message":"...", "send_to":"user", "status":"done", "report_telegram":false}
```

## 验证

运行：

```bash
cargo test --manifest-path src-tauri/Cargo.toml output_schema_ -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml role_config -- --nocapture
git diff --check -- src-tauri/src/daemon/role_config/roles.rs src-tauri/src/daemon/role_config/roles_tests.rs
```

结果：

- `output_schema_` 相关回归测试通过
- `role_config` 全量测试通过
- `git diff --check` 通过

## CM Record

- **CM:** `fix: require report_telegram in codex output schema`

