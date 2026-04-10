# Lead Telegram 全量转发实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 移除 `report_telegram` 协议字段，并把 Telegram fan-out 规则改成“所有 `from == lead` 的消息都自动发送到 Telegram”。

**Architecture:** 删除 `report_telegram` 的端到端协议面（message model、tool schema、output schema、prompt、tests），把 Telegram 的业务 gate 收口到 daemon 路由层：只判断 `msg.from == "lead"`。继续保留 Telegram runtime 的启用状态、outbound sender 与 paired chat 等发送能力检查。

**Tech Stack:** Rust（Tauri daemon + bridge）、TypeScript 类型同步、Cargo tests、Git。

---

## Baseline Evidence

- 隔离 worktree：`.worktrees/2026-04-10-telegram-lead-fanout`
- baseline 验证已完成：
  - `cargo build --manifest-path bridge/Cargo.toml`
  - `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::`
  - `cargo test --manifest-path bridge/Cargo.toml mcp_protocol`
  - `cargo test --manifest-path src-tauri/Cargo.toml telegram`
  - `git diff --check`
- baseline 结果：通过（role_config 30 pass，bridge mcp_protocol 9 pass，telegram 34 pass）

## Project Memory

### Recent related commits

- `3e2c95a7` — `feat: add report_telegram to message protocol`
- `6a6ad203` — `docs: define report_telegram prompt contract`
- `fa5f16e4` — `feat: route report_telegram messages to telegram`
- `d5e76ef5` — `fix: add notifications_enabled hard gate to report_telegram routing`
- `85ea11ad` — `fix: enforce lead-only report_telegram at ingress`
- `10209c36` — `docs: audit and accept prompt protocol updates`

### Relevant prior plans

- `docs/superpowers/plans/2026-04-09-report-telegram.md`
- `docs/superpowers/plans/2026-04-09-report-telegram-route-unification.md`
- `docs/superpowers/plans/2026-04-10-prompt-line-limit-exemption-audit.md`

### Lessons carried forward

- `report_telegram` 已经被证明是一个容易漏打、容易误导的模型控制字段。
- 真正需要保留的 gate 是发送能力（enabled / paired chat / outbound tx），不是模型输出意图。
- 当前用户已明确批准：所有 lead 消息统一推 Telegram，内部消息也不例外。

## File Map

### Protocol / types / handlers

- Modify: `src-tauri/src/daemon/types.rs`
- Modify: `bridge/src/types.rs`
- Modify: `src/types.ts`
- Modify: `bridge/src/tools.rs`
- Modify: `bridge/src/tools_tests.rs`
- Modify: `src-tauri/src/daemon/codex/handler.rs`
- Modify: `src-tauri/src/daemon/codex/session_event.rs`
- Modify: `src-tauri/src/daemon/codex/structured_output.rs`
- Modify: `src-tauri/src/daemon/codex/structured_output_tests.rs`

### Prompt / schema surfaces

- Modify: `src-tauri/src/daemon/role_config/roles.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
- Modify: `bridge/src/mcp_protocol.rs`
- Modify: `bridge/src/mcp_protocol_tests.rs`

### Telegram routing

- Modify: `src-tauri/src/telegram/report.rs`
- Modify: `src-tauri/src/daemon/routing_dispatch.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `refactor: remove report_telegram from prompt protocol` | `cargo build --manifest-path bridge/Cargo.toml`; `cargo test --manifest-path bridge/Cargo.toml tools`; `cargo test --manifest-path bridge/Cargo.toml mcp_protocol`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::`; `cargo test --manifest-path src-tauri/Cargo.toml codex::`; `cargo test --manifest-path src-tauri/Cargo.toml telegram`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::`; `git diff --check` | `3cb840f1` — removed `report_telegram` from message models, schemas, prompts, handlers, and test helpers. Direct verification passed. Full `daemon::` still had 6 pre-existing failures, accepted via Plan Revision 3 because they reproduced identically on baseline `main`. |
| Task 2 | `feat: route all lead messages to telegram` | `cargo test --manifest-path src-tauri/Cargo.toml telegram`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::`; `git diff --check` | `3cb840f1` — merged into Task 1 by Plan Revision 2. `src-tauri/src/telegram/report.rs` now routes all `from == "lead"` messages to Telegram while preserving runtime delivery gates. |

---

## Plan Revision 1 — 2026-04-10

**Approved by user:** yes (`批准`)

**Reason:** Task 1 scope originally covered the primary protocol surfaces but missed several compile- and test-reachable `BridgeMessage` construction sites plus the shared `role_protocol.rs` Telegram-reporting text. Removing `report_telegram` from the message model cannot pass the required verification set without updating those files too.

**Added to Task 1 allowed_files:**

- `bridge/src/channel_state.rs`
- `src-tauri/src/daemon/role_config/role_protocol.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`
- `src-tauri/src/telegram/runtime_handlers.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`
- `src-tauri/src/daemon/orchestrator/tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/feishu_project/task_link_tests.rs`

**Revised Task 1 budgets:**

- `max_files_changed: 28`
- `max_added_loc: 220`
- `max_deleted_loc: 340`

## Plan Revision 2 — 2026-04-10

**Approved by user:** yes (`批准`)

**Reason:** Revision 1 fixed most compile/test-reachable `report_telegram` references, but Task 1 and Task 2 still had a structural conflict: removing `report_telegram` from `BridgeMessage` makes `src-tauri/src/telegram/report.rs` uncompilable unless that file is updated in the same task. Therefore the protocol-field removal and the lead-only Telegram routing change cannot be executed as two separately verifiable tasks.

**Task merge:** Task 1 and Task 2 are merged into one execution boundary for implementation purposes.

**Added to merged task allowed_files:**

- `src-tauri/src/telegram/report.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`

**Revised merged-task budgets:**

- `max_files_changed: 30`
- `max_added_loc: 140`
- `max_deleted_loc: 360`

### Task 1: 删除 `report_telegram` 协议面

**task_id:** `remove-report-telegram-protocol`

**Acceptance criteria:**

- `report_telegram` 不再出现在 Rust / bridge / TS 消息模型中
- bridge `reply()` schema 不再声明 `report_telegram`
- Codex output schema 不再声明或要求 `report_telegram`
- Claude / Codex prompt 文本不再提及 `report_telegram`
- 相关解析器、handler 和测试全部更新为无该字段版本

**allowed_files:**

- `src-tauri/src/daemon/types.rs`
- `bridge/src/types.rs`
- `src/types.ts`
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
- `bridge/src/channel_state.rs`
- `src-tauri/src/daemon/role_config/role_protocol.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`
- `src-tauri/src/telegram/runtime_handlers.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`
- `src-tauri/src/daemon/orchestrator/tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/feishu_project/task_link_tests.rs`
- `src-tauri/src/telegram/report.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`

**max_files_changed:** `30`

**max_added_loc:** `140`

**max_deleted_loc:** `360`

**verification_commands:**

- `cargo build --manifest-path bridge/Cargo.toml`
- `cargo test --manifest-path bridge/Cargo.toml tools`
- `cargo test --manifest-path bridge/Cargo.toml mcp_protocol`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config::`
- `cargo test --manifest-path src-tauri/Cargo.toml codex::`
- `cargo test --manifest-path src-tauri/Cargo.toml telegram`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::`
- `git diff --check`

### Task 2: 把 Telegram gate 改成 lead 全量转发

**Status:** merged into Task 1 by Plan Revision 2. Do not execute as a separate task.

**task_id:** `lead-telegram-fanout`

**Acceptance criteria:**

- `should_send_telegram_report()` 仅按 `msg.from == "lead"` 判断
- 非 lead 消息不会发 Telegram
- 继续保留 `telegram_notifications_enabled`、`telegram_outbound_tx`、`paired_chat_id` 这些运行时发送能力 gate
- Telegram HTML 报文格式和 chunk 逻辑保持不变
- 对应 Telegram 测试更新为新规则

**allowed_files:**

- `src-tauri/src/telegram/report.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`

**max_files_changed:** `2`

**max_added_loc:** `40`

**max_deleted_loc:** `50`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml telegram`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::`
- `git diff --check`

## Plan Revision 3 — 2026-04-10

**Approved by user:** yes (`批准最终验收`)

**Reason:** The merged task's full `cargo test --manifest-path src-tauri/Cargo.toml daemon::` verification still reports 6 failures, but lead review reproduced the exact same 6 failing tests on baseline `main` before merge. These are therefore `pre_existing`, not introduced by this plan, and must not block acceptance of the Telegram fan-out change.

**Recorded pre-existing failures:**

- `daemon::routing::behavior_tests::auto_fanout_delivers_to_both_agents`
- `daemon::routing::shared_role_tests::stale_online_agent_reports_task_session_mismatch_reason`
- `daemon::routing::tests::route_to_claude_from_unknown_sender_drops`
- `daemon::routing::user_target_tests::auto_keeps_preferred_task_role_first_but_still_fanouts`
- `daemon::routing::user_target_tests::auto_prefers_bound_claude_coder_for_active_task`
- `daemon::state::state_tests::online_role_conflict_only_blocks_live_other_agent`

**Acceptance adjustment:**

- Direct task verification must pass in full
- Full `daemon::` suite must show **no new failures versus baseline main**
- The six failures above are treated as known pre-existing issues outside this plan scope
