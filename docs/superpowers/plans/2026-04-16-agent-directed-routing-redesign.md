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

### Claude provider path

- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_tests.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`

### Core daemon routing

- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`

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

## Task 3: Upgrade Codex output and event handling to the new target model

**task_id:** `codex-agent-directed-output`

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

- Codex structured output no longer uses role-only `send_to`
- agent-targeted and role-targeted messages both parse and route correctly
- `replyTarget` is preserved through Codex event handling
- tests cover malformed target payloads, explicit agent targets, and explicit role broadcasts

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml codex::structured_output -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex::handler -- --nocapture`
- `git diff --check`

## Task 4: Upgrade Claude event delivery to the new target model

**task_id:** `claude-agent-directed-output`

**allowed_files:**

- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_tests.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`

**max_files_changed:** `4`
**max_added_loc:** `260`
**max_deleted_loc:** `120`

**acceptance criteria:**

- Claude structured replies build the same message model as Codex
- direct user-facing SDK fallback still works for user-targeted terminal text
- sender identity stays concrete and no role-only flattening remains
- tests cover user target, role target, and agent target variants

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk::event_handler -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk_handler_processing -- --nocapture`
- `git diff --check`

## Task 5: Rewrite daemon routing around structured agent-directed targets

**task_id:** `daemon-structured-target-routing`

**allowed_files:**

- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`

**max_files_changed:** `9`
**max_added_loc:** `520`
**max_deleted_loc:** `260`

**acceptance criteria:**

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
- `git diff --check`

## Task 6: Introduce default reply-target semantics for delegation/report chains

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

## Task 7: Remove legacy role-string message fields and old target assumptions

**task_id:** `agent-directed-hard-cut-cleanup`

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
- `src-tauri/src/daemon/role_config/roles.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`
- `src-tauri/src/daemon/role_config/claude_prompt_tests.rs`

**max_files_changed:** `31`
**max_added_loc:** `260`
**max_deleted_loc:** `420`

**acceptance criteria:**

- legacy role-string message fields are removed from the shared message contract
- no production path still depends on `from/to/display_source/sender_agent_id`
- prompt/tool guidance no longer tells providers to emit role-only `send_to`
- tests lock the final hard-cut contract

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::role_config -- --nocapture`
- `cargo test --manifest-path bridge/Cargo.toml -- --nocapture`
- `git diff --check`

## Task 8: Full communication regression sweep and close-out

**task_id:** `agent-directed-routing-communication-regression`

**allowed_files:**

- `docs/superpowers/specs/2026-04-16-agent-directed-routing-redesign-design.md`
- `docs/superpowers/plans/2026-04-16-agent-directed-routing-redesign.md`

**max_files_changed:** `2`
**max_added_loc:** `40`
**max_deleted_loc:** `20`

**acceptance criteria:**

- CM records contain the real accepted commit hashes and verification evidence
- full communication regression evidence is documented
- no old `to/send_to` assumptions remain in the accepted scope
- headless real-scenario runtime tests are executed after implementation, without relying on the frontend UI

**verification_commands:**

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
| Task 3 | not started | Upgrade Codex parsing and event handling to emit the new structured target model. | Not run yet. | planned |
| Task 4 | not started | Upgrade Claude event delivery and processing to the new structured target model. | Not run yet. | planned |
| Task 5 | not started | Rewrite daemon routing to resolve explicit agent targets before role broadcast and remove old sender/receiver assumptions. | Not run yet. | planned |
| Task 6 | not started | Introduce concrete `replyTarget` semantics for delegation and default report-back. | Not run yet. | planned |
| Task 7 | not started | Remove the legacy role-string message fields and finish the hard cut once all producers/consumers have moved to the new structured model. | Not run yet. | planned |
| Task 8 | not started | Run the full communication regression sweep and close out docs/CM with accepted hashes and evidence. | Not run yet. | planned |

## Plan Revision 1 — 2026-04-16

**Reason:** The original Task 1 LOC budget assumed a direct field swap. The approved staged migration requires a full parallel structured message type in both daemon and bridge plus focused serialization coverage in both crates. The first verified implementation came in at `+322` LOC across the three allowed files while staying within scope and passing all Task 1 verification commands.

**Revised Task 1 budget:**

- `max_added_loc: 340`
