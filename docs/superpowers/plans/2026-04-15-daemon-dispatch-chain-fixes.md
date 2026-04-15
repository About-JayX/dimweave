# Daemon Dispatch Chain Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repair the daemon dispatch chain so production launch/connect behavior matches the accepted `task_agents[]` + `agent_id` routing model, and restore a green verification baseline for the relevant daemon tests.

**Architecture:** First unblock daemon verification by fixing the pre-existing `TaskSnapshot` test fixtures that no longer compile. Then remove the live same-role conflict gate from official launch/connect paths, make task-scoped missing-role sends fail clearly, and finally rebuild global online-agent snapshots from concrete online `agent_id` instances instead of provider singletons.

**Tech Stack:** Rust 1.75+, Tokio, Tauri 2, Cargo test/check, existing daemon runtime/routing modules, git

---

## Memory

- Recent related commits:
  - `bb21affc` — removed provider-channel fallback and preserved inbound `sender_agent_id`
  - `590adb4e` — per-agent-id broadcast delivery and concrete `agent_id` resolution
  - `9da95457` — per-agent broadcast iteration and end-to-end `agent_id` resolution
  - `1dba6be6` — keyed daemon runtime ownership by `agent_id`
  - `e817eeaa` / `ccd79531` — added task-scoped per-agent runtime status, which introduced the current test-fixture compile blocker
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-14-task-agent-identity-role-broadcast.md`
  - `docs/superpowers/plans/2026-03-30-unified-online-agents-hook.md`
  - `docs/superpowers/plans/2026-04-07-shared-role-protocol-refactor.md`
- Relevant design constraints carried forward:
  - same-role broadcasts are a feature, not an edge case
  - missing-role sends fail clearly once task agents are authoritative
  - `task_agents[]` / `agent_id` remain the sole identity truth for runtime ownership

## Baseline

- Worktree: `.worktrees/daemon-dispatch-fixes`
- Baseline verification before changes:
  - `cargo build -p dimweave-bridge` ✅
  - `cargo check --manifest-path src-tauri/Cargo.toml` ✅
  - `bun run build` ✅
  - `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_shared_role_tests:: -- --nocapture` ❌ pre-existing compile failure: missing `agent_runtime_statuses` in `src-tauri/src/commands_artifact.rs` and `src-tauri/src/daemon/types_tests.rs`
  - `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_user_target_tests:: -- --nocapture` ❌ same pre-existing compile failure

## Task 1: Restore the daemon test baseline

**task_id:** `daemon-dispatch-baseline-unblock`

**allowed_files:**

- `src-tauri/src/commands_artifact.rs`
- `src-tauri/src/daemon/types_tests.rs`

**max_files_changed:** `2`
**max_added_loc:** `20`
**max_deleted_loc:** `4`

**acceptance criteria:**

- all `TaskSnapshot` test fixtures compile with the current `agent_runtime_statuses` field
- the two daemon routing test commands in the baseline compile and run again
- no daemon runtime behavior changes are introduced in this task

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_shared_role_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_user_target_tests:: -- --nocapture`
- `git diff --check`

## Task 2: Remove the live same-role conflict gate from official launch/connect paths

**task_id:** `daemon-live-shared-role-connect`

**allowed_files:**

- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`

**max_files_changed:** `5`
**max_added_loc:** `140`
**max_deleted_loc:** `70`

**acceptance criteria:**

- official Claude/Codex launch/connect paths no longer reject a second provider just because it shares the same role
- explicit-`agent_id` online no-op behavior stays intact
- focused tests prove shared-role live states can exist through production launch/connect code paths

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state_tests::online_role_conflict_only_blocks_live_other_agent -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_shared_role_tests:: -- --nocapture`
- `git diff --check`

## Task 3: Make task-scoped missing-role sends fail clearly

**task_id:** `daemon-missing-role-fails-clearly`

**allowed_files:**

- `src-tauri/src/daemon/routing.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`
- `src-tauri/src/daemon/routing_user_target_tests.rs`

**max_files_changed:** `3`
**max_added_loc:** `120`
**max_deleted_loc:** `40`

**acceptance criteria:**

- if a task has explicit agents and none match the requested role, routing returns a clear failure path instead of buffering
- buffering still works for tasks that do not yet own any agents
- focused routing tests cover both cases

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_shared_role_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_user_target_tests:: -- --nocapture`
- `git diff --check`

## Task 4: Report real online agent instances in global snapshots

**task_id:** `daemon-online-agents-snapshot`

**allowed_files:**

- `src-tauri/src/daemon/state_snapshot.rs`
- `src-tauri/src/daemon/state_snapshot_tests.rs`
- `src-tauri/src/daemon/control/handler.rs`

**max_files_changed:** `3`
**max_added_loc:** `140`
**max_deleted_loc:** `60`

**acceptance criteria:**

- `online_agents_snapshot()` enumerates real online `agent_id` instances instead of one row per provider family
- `get_online_agents()` / bridge status callers receive the expanded per-agent view without changing DTO shape
- focused tests cover multi-agent same-provider visibility

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::state_snapshot_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_shared_role_tests:: -- --nocapture`
- `git diff --check`

## Task 5: Final docs and CM close-out

**task_id:** `daemon-dispatch-finalize`

**allowed_files:**

- `docs/superpowers/specs/2026-04-15-daemon-dispatch-chain-fixes-design.md`
- `docs/superpowers/plans/2026-04-15-daemon-dispatch-chain-fixes.md`

**max_files_changed:** `2`
**max_added_loc:** `40`
**max_deleted_loc:** `20`

**acceptance criteria:**

- docs reflect the accepted daemon dispatch repair behavior
- CM record contains real commit hashes, verification evidence, and any pre-existing blocker notes encountered during baseline

**verification_commands:**

- `git diff --check -- docs/superpowers/specs/2026-04-15-daemon-dispatch-chain-fixes-design.md docs/superpowers/plans/2026-04-15-daemon-dispatch-chain-fixes.md`

## CM Record

| Task | Commit | Summary | Verification | Status |
| --- | --- | --- | --- | --- |
| Task 1 | `f9fc277d` | Added the missing `agent_runtime_statuses` field to the remaining `TaskSnapshot` test fixtures so the daemon routing test targets compile again without changing runtime behavior. | `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture` ✅ 11 passed; `cargo test --manifest-path src-tauri/Cargo.toml user_target_tests -- --nocapture` ✅ 13 passed; `git diff --check` ✅ | accepted |
| Task 2 | `776aa79c`, `d2fd48e5` | Removed the singleton-era `online_role_conflict` gate from official Claude/Codex live launch/connect paths while preserving explicit-`agent_id` duplicate no-op guards, then added focused tests proving same-role cross-provider coexistence through the production launch/connect chain. | `cargo test --manifest-path src-tauri/Cargo.toml online_role_conflict -- --nocapture` ✅ 1 passed; `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture` ✅ 11 passed; `git diff --check` ✅ | accepted |
| Task 3 | `21571244` | Narrowed task-scoped missing-role routing so tasks with explicit agents now drop unmatched-role sends instead of buffering forever, while keeping buffering for tasks that still have zero agents. | `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture` ✅ 12 passed; `cargo test --manifest-path src-tauri/Cargo.toml user_target_tests -- --nocapture` ✅ 13 passed; `git diff --check` ✅ | accepted |
| Task 4 | `894fdb35`, `293393a5` | Rebuilt `online_agents_snapshot()` to enumerate real per-`agent_id` online instances from task runtime slots, then fixed phantom legacy singleton rows by suppressing fallback emission whenever that provider family already had live per-agent slots. | `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot_tests -- --nocapture` ✅ 14 passed; `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture` ✅ 12 passed; `git diff --check` ✅ | accepted |
| Task 5 | `6314f8b7` | Marked the design accepted, recorded the final dispatch-chain outcomes, and closed out the CM record after the full targeted verification set passed. | `cargo test --manifest-path src-tauri/Cargo.toml state_snapshot_tests -- --nocapture` ✅ 14 passed; `cargo test --manifest-path src-tauri/Cargo.toml shared_role_tests -- --nocapture` ✅ 12 passed; `cargo test --manifest-path src-tauri/Cargo.toml user_target_tests -- --nocapture` ✅ 13 passed; `cargo check --manifest-path src-tauri/Cargo.toml` ✅; `bun run build` ✅; `git diff --check -- docs/superpowers/specs/2026-04-15-daemon-dispatch-chain-fixes-design.md docs/superpowers/plans/2026-04-15-daemon-dispatch-chain-fixes.md` ✅ | accepted |
