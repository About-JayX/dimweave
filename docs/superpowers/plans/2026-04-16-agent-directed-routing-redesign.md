# Agent-Directed Routing Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace role-string message targeting with a structured agent-directed routing protocol so the daemon can support multiple same-role agents without report-chain ambiguity.

**Architecture:** The final merged state is a hard cut to `source/target/replyTarget`, but implementation will proceed in a staged migration so each task still compiles and can be verified. First add the new structured message types in parallel, then migrate bridge/Claude/Codex/daemon boundaries onto them, then remove the legacy role-string message fields in one dedicated cleanup task. Communication tests are mandatory at every boundary, and final acceptance also requires headless real-scenario runtime checks against live Codex/Claude providers.

**Tech Stack:** Rust 1.75+, Tokio, Tauri 2, bridge crate, Claude SDK event chain, Codex app-server session handling, Cargo test/check, Bun build, git

---

## Memory

- Recent related commits:
  - `293393a5` / `894fdb35` — repaired global online-agent snapshots to enumerate real per-agent instances
  - `21571244` — missing-role task sends now fail clearly once task agents are authoritative
  - `776aa79c` / `d2fd48e5` — removed same-role live connect conflict gates and proved production launch/connect coexistence
  - `bb21affc`, `590adb4e`, `9da95457`, `5fc23821` — recent `agent_id` routing fixes and no-fallback cleanup
  - `1dba6be6`, `c046ba4b`, `8a53a8b8`, `caae718f` — the original `task_agents[]` / per-agent runtime identity transition
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-14-task-agent-identity-role-broadcast.md`
  - `docs/superpowers/plans/2026-04-15-daemon-dispatch-chain-fixes.md`
  - `docs/superpowers/plans/2026-03-30-unified-online-agents-hook.md`
- Constraints carried forward:
  - `task_agents[]` and concrete `agent_id` remain the sole runtime identity truth
  - explicit role-broadcast remains a supported feature
  - communication tests are mandatory because this is the core daemon chain
- the final merged state is a hard cut, but the feature branch may carry a short-lived migration bridge so each task remains verifiable

## Baseline

- Worktree: `.worktrees/agent-directed-routing-design`
- Baseline verification before implementation planning:
  - current daemon dispatch chain is green on the targeted suites after `afda1f7f`
  - no implementation has started in this worktree yet

## File Map

### Shared protocol / DTO

- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `bridge/src/types.rs`

### Bridge tool / MCP contract

- `bridge/src/tools.rs`
- `bridge/src/tools_tests.rs`
- `bridge/src/mcp_io.rs`
- `bridge/src/mcp_protocol_tests.rs`
- `bridge/src/channel_state.rs`
- `bridge/src/main.rs`

### Codex provider path

- `src-tauri/src/daemon/codex/structured_output.rs`
- `src-tauri/src/daemon/codex/structured_output_tests.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/codex/handler.rs`

### Claude / daemon routing kernel path

- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_tests.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`

### Backend hard-cut cleanup paths

- `src-tauri/src/daemon/routing_dispatch.rs`
- `src-tauri/src/daemon/routing_display.rs`
- `src-tauri/src/daemon/routing_format.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/state_persistence.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`
- `src-tauri/src/daemon/orchestrator/task_flow.rs`
- `src-tauri/src/telegram/report.rs`
- `src-tauri/src/telegram/report_tests.rs`

### Frontend display / bridge consumers

- `src/types.ts`
- `src/stores/bridge-store/types.ts`
- `src/stores/bridge-store/listener-payloads.ts`
- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/MessageBubble.tsx`
- `src/components/MessagePanel/view-model.ts`
- `src/components/MessagePanel/text-tools.ts`
- `src/lib/message-payload.test.ts`

### Prompt / protocol docs

- `src-tauri/src/daemon/role_config/roles.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`
- `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
- `docs/superpowers/specs/2026-04-16-agent-directed-routing-redesign-design.md`
- `docs/superpowers/plans/2026-04-16-agent-directed-routing-redesign.md`

## Task 1: Introduce structured message types without breaking the crate

**task_id:** `directed-message-types`

**allowed_files:**

- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `bridge/src/types.rs`

**max_files_changed:** `3`
**max_added_loc:** `340`
**max_deleted_loc:** `120`

**acceptance criteria:**

- daemon and bridge define shared structured types for:
  - `MessageSource`
  - `MessageTarget`
  - a new structured message type (for example `DirectedBridgeMessage`)
- the repository still compiles after this task
- serialization tests cover user, role-targeted, and agent-targeted messages on the new structured message type
- legacy `BridgeMessage` remains temporarily untouched in this task so downstream consumers can still compile

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `cargo test --manifest-path bridge/Cargo.toml types -- --nocapture`
- `git diff --check`

## Task 2: Rebuild bridge tool and runtime contracts around structured targets

**task_id:** `bridge-structured-target-contract`

**allowed_files:**

- `bridge/src/types.rs`
- `bridge/src/tools.rs`
- `bridge/src/tools_tests.rs`
- `bridge/src/mcp_io.rs`
- `bridge/src/daemon_client_io.rs`
- `bridge/src/mcp_protocol_tests.rs`
- `bridge/src/channel_state.rs`
- `bridge/src/main.rs`

**max_files_changed:** `8`
**max_added_loc:** `420`
**max_deleted_loc:** `220`

**acceptance criteria:**

- bridge reply tool accepts structured `target`
- bridge outbound runtime path emits the new structured message type instead of immediately down-converting back to legacy role-string messages
- `user|lead|coder` hard-coded target enums are removed from the reply schema
- bridge startup no longer coerces unknown roles to `lead`
- bridge channel sender validation stays consistent with arbitrary-role support and no longer drops valid non-`lead`/`coder` roles by legacy allowlist assumptions
- tests prove invalid target objects fail clearly

**verification_commands:**

- `cargo test --manifest-path bridge/Cargo.toml tools -- --nocapture`
- `cargo test --manifest-path bridge/Cargo.toml mcp_protocol -- --nocapture`
- `cargo test --manifest-path bridge/Cargo.toml channel_state -- --nocapture`
- `git diff --check`

## Plan Revision 2 — 2026-04-16

**Reason:** Task 2 review proved the original scope was too narrow for the approved acceptance criteria. `bridge/src/types.rs` also has to move so the bridge runtime can carry the structured message shape end-to-end, `bridge/src/daemon_client_io.rs` must move because it serializes `BridgeOutbound::AgentReply(...)` at the wire boundary, and Task 2 must explicitly cover the arbitrary-role sender validation gap in `channel_state.rs`.

**Revised Task 2 scope:**

- add `bridge/src/types.rs` to `allowed_files`
- add `bridge/src/daemon_client_io.rs` to `allowed_files`
- require bridge outbound runtime emission of the structured message type until the wire serialization seam
- require channel sender validation to align with arbitrary-role support

## Task 3: Upgrade Codex parsing/building to the new target model

**task_id:** `codex-target-parsing`

**allowed_files:**

- `src-tauri/src/daemon/codex/structured_output.rs`
- `src-tauri/src/daemon/codex/structured_output_tests.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/codex/handler.rs`

**max_files_changed:** `5`
**max_added_loc:** `340`
**max_deleted_loc:** `180`

**acceptance criteria:**

- Codex structured output is no longer limited to role-only `send_to` at the parser/build layer
- parser accepts both structured target objects and legacy `send_to` strings during migration
- hard-coded `user|lead|coder` filtering is removed from Codex output parsing/building
- `replyTarget` is parsed and preserved in the Codex-side intermediate output model
- tests cover malformed target payloads, explicit agent targets, explicit role broadcasts, and replyTarget parsing
- true agent-targeted routing activation is explicitly deferred to the routing-kernel task

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml codex::structured_output -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture`
- `git diff --check`

## Task 4: Activate the structured routing kernel across Claude and daemon routing

**task_id:** `directed-routing-kernel`

**allowed_files:**

- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_tests.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`

**max_files_changed:** `13`
**max_added_loc:** `700`
**max_deleted_loc:** `340`

**acceptance criteria:**

- Claude event delivery builds the structured target model instead of flattening replies into role-only strings
- `route_message(BridgeMessage)` remains callable as a compatibility shim during migration
- internal routing resolves structured targets with priority `target.agent` → `target.role` → `target.user`
- routing priority is `target.agent` → `target.role` → `target.user`
- agent-targeted delivery reaches exactly one validated concrete agent
- role-targeted delivery still broadcasts to all matching task agents
- sender validation no longer falls back to “first online slot”
- user-input auto target resolution produces structured targets instead of role strings

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml user_target_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk::event_handler -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk_handler_processing -- --nocapture`
- `git diff --check`

## Task 5: Introduce default reply-target semantics for delegation/report chains

**task_id:** `daemon-reply-target-flow`

**allowed_files:**

- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`

**max_files_changed:** `6`
**max_added_loc:** `320`
**max_deleted_loc:** `120`

**acceptance criteria:**

- delegating messages stamp a concrete `replyTarget`
- worker replies default to that `replyTarget` unless explicitly overridden
- two same-role leads in one task can maintain separate coder report chains without cross-talk
- tests prove report-back is agent-directed rather than role-directed

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml routing_behavior_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture`
- `git diff --check`

## Task 6: Backend compatibility cutover and prompt/schema cleanup

**task_id:** `agent-directed-backend-compat-cutover`

**allowed_files:**

- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `bridge/src/types.rs`
- `bridge/src/tools.rs`
- `bridge/src/tools_tests.rs`
- `bridge/src/mcp_io.rs`
- `bridge/src/mcp_protocol_tests.rs`
- `bridge/src/channel_state.rs`
- `bridge/src/main.rs`
- `src-tauri/src/daemon/codex/structured_output.rs`
- `src-tauri/src/daemon/codex/structured_output_tests.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/codex/handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_tests.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`
- `src-tauri/src/daemon/routing_display.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/state_persistence.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`
- `src-tauri/src/daemon/orchestrator/task_flow.rs`
- `src-tauri/src/telegram/report.rs`
- `src-tauri/src/telegram/report_tests.rs`
- `src-tauri/src/daemon/role_config/roles.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`
- `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`
- `src-tauri/src/daemon/feishu_project_task_link_tests.rs`
- `src-tauri/src/feishu_project/task_link_tests.rs`

**max_files_changed:** `41`
**max_added_loc:** `420`
**max_deleted_loc:** `700`

**acceptance criteria:**

- no backend production path still depends on raw `from/to/display_source/sender_agent_id` field reads
- backend consumers use accessors / structured helpers instead of direct legacy field access
- prompt/tool guidance no longer tells providers to emit role-only `send_to`
- Codex output schema is upgraded from `send_to` to structured `target`
- tests lock the backend compatibility cutover
- shared message contract field deletion is explicitly deferred to Task 7

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config -- --nocapture`
- `cargo test --manifest-path bridge/Cargo.toml -- --nocapture`
- `git diff --check`

## Task 7: Final shared-contract hard cut, frontend alignment, and final regression

**task_id:** `agent-directed-final-hard-cut-and-regression`

**allowed_files:**

- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `bridge/src/types.rs`
- `bridge/src/daemon_client_io.rs`
- `bridge/src/channel_state.rs`
- `bridge/src/mcp_io.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`
- `src-tauri/src/telegram/runtime_handlers.rs`
- `src-tauri/src/daemon/state_persistence.rs`
- `src-tauri/src/daemon/routing_format.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/orchestrator/tests.rs`
- `src-tauri/src/feishu_project/task_link_tests.rs`
- `src-tauri/src/daemon/feishu_project_task_link_tests.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/codex/handler.rs`
- `src-tauri/src/daemon/codex/mod.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`
- `src-tauri/src/daemon/routing_display.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/routing_target_session.rs`
- `src-tauri/src/daemon/orchestrator/task_flow.rs`
- `src-tauri/src/telegram/report.rs`
- `src/types.ts`
- `src/stores/bridge-store/types.ts`
- `src/stores/bridge-store/listener-payloads.ts`
- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/MessageList.test.tsx`
- `src/components/MessagePanel/MessageBubble.tsx`
- `src/components/MessagePanel/MessageBubble.test.tsx`
- `src/components/MessagePanel/view-model.ts`
- `src/components/MessagePanel/text-tools.ts`
- `src/lib/message-payload.test.ts`
- `tests/message-panel-view-model.test.ts`
- `tests/message-render-performance.test.ts`
- `docs/superpowers/specs/2026-04-16-agent-directed-routing-redesign-design.md`
- `docs/superpowers/plans/2026-04-16-agent-directed-routing-redesign.md`

**max_files_changed:** `46`
**max_added_loc:** `700`
**max_deleted_loc:** `800`

**acceptance criteria:**

- legacy role-string fields are removed from the shared message contract itself
- bridge/daemon wire protocol no longer depends on `from/to/display_source/sender_agent_id`
- frontend bridge/message display types align with the final structured message protocol
- no frontend production path still depends on the removed legacy routing fields
- automated communication regression evidence is documented

**verification_commands:**

- `bun test src/lib/message-payload.test.ts`
- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml user_target_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config -- --nocapture`
- `cargo test --manifest-path bridge/Cargo.toml -- --nocapture`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo build -p dimweave-bridge`
- `bun run build`
- `git diff --check`

## Task 8: Headless live runtime harness and real-scenario validation

**task_id:** `agent-directed-headless-live-validation`

**allowed_files:**

- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/control/server.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/codex/mod.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/claude_sdk/mod.rs`
- `src-tauri/src/daemon/claude_sdk/runtime.rs`
- `src-tauri/src/daemon/launch_task_sync.rs`
- `src-tauri/src/daemon/live_runtime_validation_tests.rs`
- `docs/superpowers/specs/2026-04-16-agent-directed-routing-redesign-design.md`
- `docs/superpowers/plans/2026-04-16-agent-directed-routing-redesign.md`

**max_files_changed:** `12`
**max_added_loc:** `500`
**max_deleted_loc:** `120`

**acceptance criteria:**

- a code-level headless validation harness exists for orchestrating daemon/provider scenarios without using the frontend UI
- the harness can create/select a task, attach task agents, launch Codex/Claude, and observe routed messages through code-level entrypoints
- the following live scenarios are executed and verified in this environment:
  - `Codex=lead / Claude=coder`
  - `Claude=lead / Codex=coder`
  - at least one multi-agent task with a same-role case
- CM records contain the real accepted commit hashes and live validation evidence

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml live_runtime_validation -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml user_target_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot_tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config -- --nocapture`
- `cargo test --manifest-path bridge/Cargo.toml -- --nocapture`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo build -p dimweave-bridge`
- `bun run build`
- `git diff --check`

## Communication Test Matrix

The implementation is not complete until these scenarios are covered by automated tests:

1. `user -> role(coder)` broadcast to multiple coders in one task
2. `lead(agent-1) -> coder(agent-2)` exact delegation
3. `coder(agent-2) -> replyTarget(agent-1)` default report-back
4. two same-role leads + two coders in one task with independent report chains
5. same-provider same-role agents receiving different targeted replies
6. invalid `target.agentId` rejected clearly
7. target agent in another task rejected clearly
8. reconnect/resume preserves concrete sender and reply target
9. user-targeted terminal replies still surface correctly
10. explicit role broadcast remains supported after agent-targeted routing lands
11. headless live scenario: Codex as lead, Claude as coder
12. headless live scenario: Claude as lead, Codex as coder
13. headless live scenario: multi-agent task with at least one same-role case

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `03c5d526` | Introduced the shared structured routing types (`MessageSource`, `MessageTarget`, and `DirectedBridgeMessage`) in both daemon and bridge while leaving the legacy `BridgeMessage` untouched so downstream consumers still compile during migration. | `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture` ✅ 13 passed; `cargo test --manifest-path bridge/Cargo.toml types -- --nocapture` ✅ 3 passed; `git diff --check` ✅ | accepted |
| Task 2 | `82e2a433` | Rebuilt the bridge tool/runtime boundary around structured targets: `BridgeOutbound::AgentReply` now carries the structured reply type through the bridge runtime, the legacy conversion moved to `daemon_client_io.rs` at the wire seam, structured `target` parsing replaced the old `to` schema, arbitrary role startup is preserved, and channel sender validation no longer drops valid non-`lead`/`coder` roles. | `cargo test --manifest-path bridge/Cargo.toml tools -- --nocapture` ✅ 21 passed; `cargo test --manifest-path bridge/Cargo.toml mcp_protocol -- --nocapture` ✅ 9 passed; `cargo test --manifest-path bridge/Cargo.toml channel_state -- --nocapture` ✅ 6 passed; `git diff --check` ✅ | accepted |
| Task 3 | `3283dd1d` | Upgraded Codex structured-output parsing/building to use `MessageTarget` / parsed `replyTarget` in its intermediate model, removed the hard-coded known-role filter, and explicitly left true agent-targeted routing activation for the routing-kernel task. | `cargo test --manifest-path src-tauri/Cargo.toml codex::structured_output -- --nocapture` ✅ 26 passed; `cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture` ✅ 6 passed; `git diff --check` ✅ | accepted |
| Task 4 | `d5db14b9` | Activated the directed-routing kernel while keeping `route_message(BridgeMessage)` callable as a compatibility shim: task and non-task paths now resolve concrete agent targets before role broadcast, user-input internally resolves structured targets before flattening at the compatibility boundary, Claude SDK direct delivery now builds a structured user target internally, and sender identity no longer falls back to the first online slot when multiple same-provider agents are live. | `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture` ✅ 41 passed; `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture` ✅ 15 passed; `cargo test --manifest-path src-tauri/Cargo.toml user_target_tests -- --nocapture` ✅ 13 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests:: -- --nocapture` ✅ 94 passed; `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk::event_handler -- --nocapture` ✅ 10 passed; `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk_handler_processing -- --nocapture` ✅ 0 matched tests; `git diff --check` ✅ | accepted |
| Task 5 | `30b7d6fd` | Added runtime reply-target tracking so agent-targeted delegations record a one-way default report-back target, role-targeted replies from the delegated worker redirect to the delegating agent, explicit agent overrides still win, and redirected replies no longer create reciprocal sticky mappings. | `cargo test --manifest-path src-tauri/Cargo.toml reply_target -- --nocapture` ✅ 8 passed; `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture` ✅ 17 passed; `git diff --check` ✅ | accepted |
| Task 6 | `0499a7c8` | Converted backend production consumers from raw legacy field reads to accessors/structured helpers, cut prompt/tool/schema guidance over to structured `target`, and locked the backend-compatible staged state while explicitly leaving shared-contract field deletion for the final task. | `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture` ✅ 41 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture` ✅ 13 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config -- --nocapture` ✅ 28 passed; `cargo test --manifest-path bridge/Cargo.toml -- --nocapture` ✅ 41 passed; `git diff --check` ✅ | accepted |
| Task 7 | `70dabf89` | Removed the legacy flat fields from the shared `BridgeMessage` contract itself, aligned bridge/daemon wire consumers and frontend message/display types to the structured `source/target/replyTarget` model, and brought the automated regression suites back to green. Headless live provider scenarios remain pending as the last acceptance step. | `bun test src/lib/message-payload.test.ts` ✅ 2 passed; `bun test src/components/MessagePanel/MessageBubble.test.tsx src/components/MessagePanel/MessageList.test.tsx tests/message-panel-view-model.test.ts tests/message-render-performance.test.ts src/lib/message-payload.test.ts` ✅ 34 passed; `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture` ✅ 41 passed; `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture` ✅ 17 passed; `cargo test --manifest-path src-tauri/Cargo.toml user_target_tests -- --nocapture` ✅ 13 passed; `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot_tests -- --nocapture` ✅ 14 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture` ✅ 13 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config -- --nocapture` ✅ 28 passed; `cargo test --manifest-path bridge/Cargo.toml -- --nocapture` ✅ 41 passed; `cargo check --manifest-path src-tauri/Cargo.toml` ✅; `cargo build -p dimweave-bridge` ✅; `bun run build` ✅; `git diff --check` ✅ | accepted |
| Task 8 | not started | Add a code-level headless validation harness and execute the required live Codex/Claude scenarios without using the frontend UI, then record the final acceptance evidence. | Not run yet. | planned |

## Plan Revision 3 — 2026-04-16

**Reason:** Joint lead+c coder audit of the remaining tasks after Task 2 found three plan-level issues: (1) old Task 3 overclaimed true agent-targeted routing even though daemon routing still consumed legacy `msg.to` role strings; (2) old Task 4 was too thin and its code-path changes were inseparable from the routing-kernel rewrite; (3) the old hard-cut cleanup task omitted backend side-effect paths and frontend consumers that still depend on legacy `BridgeMessage` fields.

**Revised remaining-task sequence:**

- Task 3: Codex parsing/building only
- Task 4: Claude + daemon routing-kernel activation
- Task 5: replyTarget flow
- Task 6: backend hard-cut cleanup
- Task 7: frontend alignment + final automated/headless regression

## Plan Revision 4 — 2026-04-16

**Reason:** Task 6 implementation review proved the prior Task 6/Task 7 split was still too aggressive. Backend code can be migrated away from *reading* raw legacy fields before the shared contract fields themselves are deleted, and Task 6 also needed two Feishu test files already touched by the candidate. The final field deletion must move into the last hard-cut + regression task where bridge wire consumers and frontend display types are updated together.

**Revised final sequence:**

- Task 6: backend compatibility cutover + prompt/schema cleanup
- Task 7: shared-contract field deletion + bridge/frontend final alignment + full regression

## Plan Revision 5 — 2026-04-16

**Reason:** Final feasibility audit after the Task 6 review found that Task 7 still omitted a set of backend and test files that construct `BridgeMessage` directly. Once the legacy shared-contract fields are deleted, those construction sites will fail to compile unless they are included in the final hard-cut task.

**Revised Task 7 scope:**

- add backend construction/wire files:
  - `src-tauri/src/daemon/routing.rs`
  - `src-tauri/src/daemon/routing_user_input.rs`
  - `src-tauri/src/daemon/feishu_project_task_link.rs`
  - `src-tauri/src/telegram/runtime_handlers.rs`
  - `src-tauri/src/daemon/state_persistence.rs`
  - `src-tauri/src/daemon/routing_format.rs`
- add backend/test construction sites:
  - `src-tauri/src/daemon/state_tests.rs`
  - `src-tauri/src/daemon/routing_tests.rs`
  - `src-tauri/src/daemon/routing_behavior_tests.rs`
  - `src-tauri/src/daemon/routing_shared_role_tests.rs`
  - `src-tauri/src/daemon/orchestrator/tests.rs`
  - `src-tauri/src/feishu_project/task_link_tests.rs`
  - `src-tauri/src/daemon/feishu_project_task_link_tests.rs`
- increase Task 7 budget to `max_files_changed: 41`, `max_added_loc: 600`, `max_deleted_loc: 700`

## Plan Revision 6 — 2026-04-16

**Reason:** Task 6 review showed one remaining scope hole: `src-tauri/src/daemon/routing_format.rs` is a backend production consumer that still read legacy `BridgeMessage` fields and was correctly updated by the candidate, but it was omitted from Task 6 `allowed_files`.

**Revised Task 6 scope:**

- add `src-tauri/src/daemon/routing_format.rs` to `allowed_files`
- increase Task 6 budget to `max_files_changed: 41`

## Plan Revision 7 — 2026-04-16

**Reason:** Task 7 review showed the final hard-cut file list still missed one backend production construction site (`src-tauri/src/daemon/codex/mod.rs`) plus four frontend regression test files and one frontend render test file that must be updated once the shared `BridgeMessage` contract deletes the legacy flat fields.

**Revised Task 7 scope:**

- add backend production file:
  - `src-tauri/src/daemon/codex/mod.rs`
- add frontend/test files:
  - `src/components/MessagePanel/MessageList.test.tsx`
  - `src/components/MessagePanel/MessageBubble.test.tsx`
  - `tests/message-panel-view-model.test.ts`
  - `tests/message-render-performance.test.ts`
- increase Task 7 budget to `max_files_changed: 46`, `max_added_loc: 700`, `max_deleted_loc: 800`

## Plan Revision 8 — 2026-04-16

**Reason:** Final Task 7 review showed one remaining backend consumer omission: `src-tauri/src/daemon/state_delivery.rs` still mutates or inspects message target state and is touched by the final hard-cut candidate, so it must be included in the Task 7 scope.

**Revised Task 7 scope:**

- add `src-tauri/src/daemon/state_delivery.rs` to `allowed_files`

## Plan Revision 1 — 2026-04-16

**Reason:** The original Task 1 LOC budget assumed a direct field swap. The approved staged migration requires a full parallel structured message type in both daemon and bridge plus focused serialization coverage in both crates. The first verified implementation came in at `+322` LOC across the three allowed files while staying within scope and passing all Task 1 verification commands.

**Revised Task 1 budget:**

- `max_added_loc: 340`
