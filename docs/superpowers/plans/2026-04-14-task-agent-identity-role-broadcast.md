# Task Agent Identity And Role-Broadcast Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the singleton `lead/coder` task-agent model with `task_agents[]` as the sole source of truth, support extensible roles with broadcast-by-role routing, and update the task UI so tasks can own zero or more independently identified agents.

**Architecture:** First introduce `TaskAgent` plus migration from legacy singleton task fields, while keeping compatibility reads only where required. Then rewire routing/runtime/state snapshots around `agent_id` and role broadcasts. Finally update the frontend task pane, target picker, and task-agent management UI to consume the new model directly.

**Tech Stack:** Rust async daemon, Tauri 2 commands/events, React 19, Zustand 5, Bun test, Vite

---

## Memory

- Recent related commits:
  - `6938ba4d` — task runtime foundation with persisted `lead_provider` / `coder_provider`
  - `14dd2b70` and `0a661996` — task-scoped routing and provider-origin stamping
  - `629e711e` — task config contract built around singleton lead/coder bindings
  - `8a15a782` through `9a2232e2` — task-first task setup modal built on the singleton role-slot model
  - `24006491` — UI error log separation and manual retry; this work remains valid
  - `f7cb99ba` — current `main` baseline
- Superseded plan/spec:
  - `docs/superpowers/specs/2026-04-13-task-first-sidebar-and-ui-error-log-design.md`
  - `docs/superpowers/plans/2026-04-13-task-first-sidebar-and-ui-error-log.md`
- Lessons carried forward:
  - Do not keep two competing sources of truth for agent ownership.
  - Routing correctness must not depend on UI focus.
  - Temporary compatibility fields are acceptable only as migration inputs, not as long-term authoritative state.

## Scope Notes

- Single-workspace UX remains in place.
- The UI error log work remains valid and is not reimplemented here unless a direct agent-model dependency requires a narrow follow-up.
- The target design allows zero agents per task.
- Roles are arbitrary non-empty strings.
- Same-role broadcasts are a feature, not an edge case.

## Task 1: Introduce `TaskAgent` model and legacy migration

**task_id:** `task-agent-model-and-migration`

**Acceptance criteria:**

- A new persisted `TaskAgent` model exists with stable internal `agent_id`.
- Existing singleton task fields (`lead_provider`, `coder_provider`, `lead_session_id`, `current_coder_session_id`) are no longer the primary truth for new logic.
- Legacy tasks migrate deterministically into zero, one, or two initial `TaskAgent` records without duplication on repeated load.
- Migration preserves existing task/session relationships as far as the current data allows.
- Compatibility reads, if retained, are clearly transitional and do not drive new writes.

**allowed_files:**

- `src-tauri/src/daemon/task_graph/types.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/persist.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/state_snapshot_tests.rs`
- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/types_tests.rs`

**max_files_changed:** `10`
**max_added_loc:** `520`
**max_deleted_loc:** `180`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `git diff --check`

## Plan Revision 1 — 2026-04-14

**Reason:** Task 1 requires persisted `TaskAgent` records and deterministic migration on repeated load. The snapshot serialization/deserialization layer lives in `src-tauri/src/daemon/task_graph/persist.rs`, so migration cannot be implemented correctly without putting that file in scope.

**Added to Task 1 allowed_files:**

- `src-tauri/src/daemon/task_graph/persist.rs`

**Revised Task 1 budgets:**

- `max_files_changed: 10`
- `max_added_loc: 520`
- `max_deleted_loc: 180`

## Plan Revision 2 — 2026-04-14

**Reason:** Task 1 adds `task_agents` to the persisted `TaskSnapshot` DTO. `src-tauri/src/commands_artifact.rs` contains a test helper that constructs `TaskSnapshot` directly, so it needs a mechanical compile-fix update to include the new field. This does not expand runtime behavior; it only keeps the test helper aligned with the approved DTO change.

**Added to Task 1 allowed_files:**

- `src-tauri/src/commands_artifact.rs`

**Revised Task 1 budgets:**

- `max_files_changed: 11`
- `max_added_loc: 525`
- `max_deleted_loc: 180`

## Plan Revision 3 — 2026-04-14

**Reason:** The original Task 1 verification filter `types_tests` does not match the actual Rust test module name and can return zero executed tests while still exiting successfully. The task needs a precise filter that actually executes the DTO/type snapshot coverage.

**Revised Task 1 verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `git diff --check`

## Plan Revision 4 — 2026-04-14

**Reason:** Review of the first attempt at Task 2 proved that role-broadcast routing cannot be correct while runtime ownership still collapses to provider names (`"claude"` / `"codex"`). The remaining work must first establish backend agent runtime ownership by `agent_id`, then rewire routing/snapshots on top of that.

**Superseded tasks:**

- The original Task 2, Task 3, Task 4, and Task 5 sections below this point are superseded.
- Execute the revised Task 2 through revised Task 6 sections that follow.

## Revised Task 2: Introduce backend `agent_id` runtime ownership

**task_id:** `agent-runtime-ownership-by-id`

**Acceptance criteria:**

- Live provider/session ownership resolves to concrete `agent_id`, not singleton provider names.
- Session/runtime structures can represent multiple agents of the same provider inside one task.
- Normalized session ownership is persisted by `agent_id`, so launch/resume can preserve a stable agent identity.
- Launch/resume flows bind runtime state to a specific `agent_id`.
- Provider-originated events can resolve the owning `agent_id` without using singleton task slots as primary truth.
- Same-provider multi-agent scenarios no longer collapse into one ownership slot, including live daemon handle ownership.

**allowed_files:**

- `src-tauri/src/daemon/task_graph/types.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/task_graph/persist.rs`
- `src-tauri/src/daemon/task_graph/tests.rs`
- `src-tauri/src/daemon/task_runtime.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/launch_task_sync.rs`
- `src-tauri/src/daemon/claude_sdk/runtime.rs`
- `src-tauri/src/daemon/claude_sdk/mod.rs`
- `src-tauri/src/daemon/claude_sdk/reconnect.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler.rs`
- `src-tauri/src/daemon/codex/mod.rs`
- `src-tauri/src/daemon/codex/runtime.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/provider/claude.rs`
- `src-tauri/src/daemon/provider/codex.rs`
- `src-tauri/src/daemon/provider/shared.rs`
- `src-tauri/src/daemon/provider/claude_tests.rs`
- `src-tauri/src/daemon/provider/codex_tests.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_input_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/state_task_snapshot_tests.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/daemon/state_tests.rs`

**max_files_changed:** `30`
**max_added_loc:** `1200`
**max_deleted_loc:** `360`

**verification_commands:**

- `cargo check --manifest-path src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::claude_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml agent_runtime_ownership -- --nocapture`
- `git diff --check`

## Plan Revision 5 — 2026-04-14

**Reason:** Review of the first Task 2 attempts proved two additional requirements:

1. backend runtime ownership must include `SessionHandle.agent_id` and provider registration/resume paths, not just task runtime slots
2. compile-fix ripples in provider/test helper files are unavoidable once normalized session ownership carries `agent_id`

Task 2 also needs stronger verification because `cargo check --tests` caught compile drift that the narrower task filters missed.

**Added to revised Task 2 allowed_files:**

- `src-tauri/src/daemon/provider/shared.rs`
- `src-tauri/src/daemon/provider/claude_tests.rs`
- `src-tauri/src/daemon/provider/codex_tests.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_input_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/state_task_snapshot_tests.rs`
- `src-tauri/src/daemon/types_tests.rs`

**Revised Task 2 budgets:**

- `max_files_changed: 30`
- `max_added_loc: 1200`
- `max_deleted_loc: 360`

**Revised Task 2 verification_commands:**

- `cargo check --manifest-path src-tauri/Cargo.toml --tests`
- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::claude_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml agent_runtime_ownership -- --nocapture`
- `git diff --check`

## Revised Task 3: Route and snapshot by `agent_id` and role broadcast

**task_id:** `agent-id-routing-and-broadcast`

**Acceptance criteria:**

- Provider-originated messages, runtime summaries, and status events resolve ownership by `agent_id`, not singleton task slots.
- `target=<role>` resolves to all task agents in the active task with that role and broadcasts delivery to each.
- `auto` resolves to role `lead` if present; otherwise to the first ordered task role.
- Missing-role sends fail clearly instead of silently rerouting.
- Existing task focus does not affect agent ownership correctness.

**allowed_files:**

- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_processing.rs`
- `src-tauri/src/daemon/codex/handler.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`
- `src-tauri/src/daemon/types.rs`
- `src-tauri/src/daemon/types_dto.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_tests.rs`

**max_files_changed:** `25`
**max_added_loc:** `1040`
**max_deleted_loc:** `360`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing::shared_role_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing::user_target_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml agent_id_routing -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `git diff --check`

## Plan Revision 6 — 2026-04-14

**Reason:** The original Task 3 verification filters `daemon::routing_shared_role_tests::` and `daemon::routing_user_target_tests::` can match zero tests because the actual module paths are nested under `daemon::routing::...`. The verification commands must use the concrete module path so non-zero routing suites run during review.

**Revised Task 3 verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing::shared_role_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing::user_target_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml agent_id_routing -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture`
- `git diff --check`

## Revised Task 4: Replace frontend singleton task bindings with task-agent collections

**task_id:** `frontend-task-agents-state`

**Acceptance criteria:**

- Task store no longer treats `leadProvider/coderProvider` as the primary frontend truth.
- Task snapshots hydrate task-agent collections and per-agent runtime/config state.
- Dynamic target picker options come from the active task’s actual role set.
- Default target selection is `lead` if present, otherwise the first ordered role.
- No-task state remains valid and stable.

**allowed_files:**

- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/events.ts`
- `src/stores/task-store/selectors.ts`
- `src/stores/bridge-store/types.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/selectors.ts`
- `tests/task-store.test.ts`
- `src/components/ReplyInput/TargetPicker.tsx`
- `src/components/ReplyInput/index.tsx`
- `src/components/ReplyInput/index.test.tsx`
- `src/components/ReplyInput/task-session-guard.ts`

**max_files_changed:** `12`
**max_added_loc:** `560`
**max_deleted_loc:** `220`

**verification_commands:**

- `bun test tests/task-store.test.ts src/components/ReplyInput/index.test.tsx`
- `bun run build`
- `git diff --check`

## Plan Revision 7 — 2026-04-14

**Reason:** Task 3 review established that provider-originated ownership and task-scoped status still require the Claude SDK event-handler chain and a small `state.rs` export surface to move from provider-level ids to concrete `agent_id`s. These files are directly on the acceptance path for agent-id routing and stamping.

**Added to revised Task 3 allowed_files:**

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
- `src-tauri/src/daemon/claude_sdk/event_handler_tests.rs`

**Revised Task 3 budgets:**

- `max_files_changed: 25`
- `max_added_loc: 1040`
- `max_deleted_loc: 360`

## Revised Task 5: Rebuild task pane agent management around `task_agents[]`

**task_id:** `task-pane-agent-list-and-dialog`

**Acceptance criteria:**

- `New Task` can create an empty task without any agent records.
- The task pane shows the active task’s agent list, not singleton lead/coder bindings.
- Users can add multiple agents to the same task, including multiple agents with the same role.
- Agent rows are draggable and persisted in task-local order.
- `Edit Task` / `Add Agent` flows edit concrete task-agent records rather than task-level provider slots.
- Create/edit dialog uses role strings directly and does not expose the old singleton provider-slot UI.

**allowed_files:**

- `src/components/TaskContextPopover.tsx`
- `src/components/TaskContextPopover.test.tsx`
- `src/components/ShellContextBar.tsx`
- `src/components/ShellContextBar.test.tsx`
- `src/components/TaskPanel/index.tsx`
- `src/components/TaskPanel/view-model.ts`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/TaskHeader.test.tsx`
- `src/components/TaskPanel/TaskSetupDialog.tsx`
- `src/components/TaskPanel/TaskSetupDialog.test.tsx`
- `src/components/TaskPanel/use-artifact-detail.ts`
- `src/components/TaskPanel/TaskAgentList.tsx`
- `src/components/TaskPanel/TaskAgentList.test.tsx`
- `src/components/TaskPanel/TaskAgentEditor.tsx`
- `src/components/TaskPanel/TaskAgentEditor.test.tsx`
- `src/components/ClaudePanel/index.tsx`
- `src/components/ClaudePanel/connect-state.test.ts`
- `src/components/ClaudePanel/launch-request.ts`
- `src/components/ClaudePanel/launch-request.test.ts`
- `src/components/AgentStatus/index.tsx`
- `src/components/AgentStatus/RoleSelect.tsx`
- `src/components/AgentStatus/CodexHeader.tsx`
- `src/components/AgentStatus/CodexPanel.tsx`
- `src/components/AgentStatus/codex-launch-config.ts`
- `src/components/AgentStatus/codex-launch-config.test.ts`
- `src/components/AgentStatus/provider-session-view-model.ts`
- `src/components/ReplyInput/Footer.tsx`

**max_files_changed:** `27`
**max_added_loc:** `1100`
**max_deleted_loc:** `420`

**verification_commands:**

- `bun test src/components/ShellContextBar.test.tsx src/components/TaskContextPopover.test.tsx`
- `bun test src/components/TaskPanel/TaskHeader.test.tsx src/components/TaskPanel/TaskSetupDialog.test.tsx src/components/TaskPanel/TaskAgentList.test.tsx src/components/TaskPanel/TaskAgentEditor.test.tsx`
- `bun test src/components/ClaudePanel/connect-state.test.ts src/components/ClaudePanel/launch-request.test.ts src/components/AgentStatus/codex-launch-config.test.ts`
- `bun test src/components/ReplyInput/index.test.tsx`
- `bun run build`
- `git diff --check`

## Revised Task 6: Final integration, supersession docs, and regression guard

**task_id:** `agent-identity-final-integration`

**Acceptance criteria:**

- Old singleton slot docs are marked superseded.
- Final spec/plan CM records are updated with accepted commits and verification evidence.
- Final integration tests confirm: empty-task creation, dynamic role targets, broadcast semantics, no-task stability, and preserved UI error-log behavior.
- The overall feature set is stage-complete on the new `task_agents[]` model.

**allowed_files:**

- `docs/superpowers/specs/2026-04-14-task-agent-identity-role-broadcast-design.md`
- `docs/superpowers/plans/2026-04-14-task-agent-identity-role-broadcast.md`
- `docs/superpowers/specs/2026-04-13-task-first-sidebar-and-ui-error-log-design.md`
- `docs/superpowers/plans/2026-04-13-task-first-sidebar-and-ui-error-log.md`
- `tests/task-store.test.ts`
- `src/components/ReplyInput/index.test.tsx`
- `src/components/ErrorBoundary.test.tsx`
- `src/components/ErrorLogDialog.test.tsx`

**max_files_changed:** `8`
**max_added_loc:** `220`
**max_deleted_loc:** `120`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture`
- `bun test tests/task-store.test.ts src/components/ReplyInput/index.test.tsx src/components/ErrorBoundary.test.tsx src/components/ErrorLogDialog.test.tsx`
- `bun run build`
- `git diff --check`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `caae718f`, `faa4d78f` | Introduced persisted `TaskAgent` with stable internal ids, added store CRUD plus deterministic legacy migration from singleton task slots, persisted `task_agents` in snapshots with backward-compatible load migration, and exposed `task_agents` on `TaskSnapshot` DTOs. Follow-up `faa4d78f` tightened migration so legacy tasks derive zero/one/two agents from actual session occupancy evidence instead of default slot fields. | `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture` ✅ 45 passed; `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot -- --nocapture` ✅ 10 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::types::tests -- --nocapture` ✅ 10 passed; `git diff --check` ✅ | accepted |
| Task 2 | `5d24a376`, `c046ba4b`, `1dba6be6`, `160d2a43`, `d55a3a56`, `8782d0f0` | Introduced `agent_id`-aware task runtime slots, keyed daemon live Claude/Codex handle registries by `agent_id`, preserved `SessionHandle.agent_id` through provider registration/resume, stopped same-provider same-role launches from collapsing to one identity, and fixed the Claude attach-provider-history path to bind the created `agent_id` into the normalized session. | `cargo check --manifest-path src-tauri/Cargo.toml --tests` ✅; `cargo test --manifest-path src-tauri/Cargo.toml task_graph:: -- --nocapture` ✅ 47 passed; `cargo test --manifest-path src-tauri/Cargo.toml claude_sdk:: -- --nocapture` ✅ 38 passed; `cargo test --manifest-path src-tauri/Cargo.toml codex:: -- --nocapture` ✅ 52 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::claude_tests:: -- --nocapture` ✅ 18 passed; `cargo test --manifest-path src-tauri/Cargo.toml daemon::provider::codex_tests:: -- --nocapture` ✅ 18 passed; `cargo test --manifest-path src-tauri/Cargo.toml agent_runtime_ownership -- --nocapture` ✅ 9 passed; `git diff --check` ✅ | accepted |
| Task 3 | _pending_ | _pending_ | _pending_ | pending |
| Task 4 | _pending_ | _pending_ | _pending_ | pending |
| Task 5 | _pending_ | _pending_ | _pending_ | pending |
| Task 6 | _pending_ | _pending_ | _pending_ | pending |
