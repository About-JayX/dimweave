# Prompt 行数豁免审计实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 prompt / 协议定义源码文件从 200 行限制中明确豁免，并在验证通过后提交当前已存在的 prompt/protocol 改动。

**Architecture:** 保持现有 dirty prompt 改动内容不变，只在 `CLAUDE.md` 写入 prompt 行数豁免说明，然后对当前改动执行定向审计与验证。通过后提交受控 commit，并把真实 commit 记录回填到 CM Memory。

**Tech Stack:** Markdown 文档、Rust 测试、Cargo、Git。

---

## Baseline Evidence

- 当前用户明确批准的新规则：**prompt 不计入 line 检查，并写入文档**。
- 直接相关 dirty 文件：
  - `bridge/src/mcp_protocol.rs`
  - `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
  - `src-tauri/src/daemon/role_config/role_protocol.rs`
  - `src-tauri/src/daemon/role_config/roles_tests.rs`
- baseline（干净 worktree）验证：
  - `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::` → 30 pass
  - `cargo test --manifest-path bridge/Cargo.toml mcp_protocol` → 9 pass
- dirty working tree 验证：
  - `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::` → 30 pass
  - `cargo test --manifest-path bridge/Cargo.toml mcp_protocol` → 9 pass

## Project Memory

### Recent related commits

- `3e2c95a7` — `feat: add report_telegram to message protocol`
- `6a6ad203` — `docs: define report_telegram prompt contract`
- `f42030b6` — `fix: require report_telegram in codex output schema`
- `fb7f8db9` — `refactor: harden lead communication prompt contract`
- `b9956525` — `refactor: share dimweave role prompt protocol`

### Relevant prior plan links

- `docs/superpowers/plans/2026-04-09-report-telegram.md`

### Relevant chain / hotfix references

- `docs/agents/claude-chain.md`
- `docs/agents/codex-chain.md`

### Lessons carried forward

- prompt/protocol 变更必须用运行时/测试结果说话，不能只凭肉眼判断；
- 规则变更必须先写入 Source-of-Truth 文档，再据此接受改动；
- 本次仅按用户批准范围调整文档，不扩展到普通源码文件的行数限制。

## File Map

### Documentation / audit trail

- Modify: `CLAUDE.md`
- Create: `docs/superpowers/specs/2026-04-10-prompt-line-limit-exemption-audit-design.md`
- Create: `docs/superpowers/plans/2026-04-10-prompt-line-limit-exemption-audit.md`

### Audited prompt/protocol files

- Modify: `bridge/src/mcp_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
- Modify: `src-tauri/src/daemon/role_config/role_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `docs: exempt prompt protocol files from line limit` | `git diff --check` | 用户于 2026-04-10 明确批准 prompt 不参与 200 行限制，并要求把规则写入文档。 |
| Task 2 | `docs: audit and accept prompt protocol updates` | `cargo build --manifest-path bridge/Cargo.toml`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::`; `cargo test --manifest-path bridge/Cargo.toml mcp_protocol`; `git diff --check` | 相关 prompt/protocol 改动来自 `3e2c95a7`、`6a6ad203`、`f42030b6`、`fb7f8db9`、`b9956525` 这条演进链，baseline 与 dirty 验证均为绿色。 |

---

### Task 1: 把 prompt 行数豁免写入文档

**task_id:** `prompt-line-limit-doc`

**Acceptance criteria:**

- `CLAUDE.md` 明确说明：prompt / 协议定义源码文件不计入 200 行限制
- 文档豁免范围足够具体，避免被解释成所有源码文件都可超长
- `git diff --check` 通过

**allowed_files:**

- `CLAUDE.md`

**max_files_changed:** `1`

**max_added_loc:** `12`

**max_deleted_loc:** `4`

**verification_commands:**

- `git diff --check -- CLAUDE.md`

### Task 2: 接受并提交当前 prompt/protocol 改动

**task_id:** `prompt-protocol-cm`

**Acceptance criteria:**

- 当前 dirty prompt/protocol 变更在新文档规则下无剩余阻塞项
- 定向验证全部通过
- 真实 commit 产生并回填到 `CM Memory`

**allowed_files:**

- `bridge/src/mcp_protocol.rs`
- `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
- `src-tauri/src/daemon/role_config/role_protocol.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`
- `CLAUDE.md`
- `docs/superpowers/specs/2026-04-10-prompt-line-limit-exemption-audit-design.md`
- `docs/superpowers/plans/2026-04-10-prompt-line-limit-exemption-audit.md`

**max_files_changed:** `7`

**max_added_loc:** `460`

**max_deleted_loc:** `150`

**verification_commands:**

- `cargo build --manifest-path bridge/Cargo.toml`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::`
- `cargo test --manifest-path bridge/Cargo.toml mcp_protocol`
- `git diff --check`
