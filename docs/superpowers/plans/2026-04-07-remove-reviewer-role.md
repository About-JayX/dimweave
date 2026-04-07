# Remove Reviewer Role Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the standalone `reviewer` role and the entire review-gate / review-status flow so Dimweave runs with only `user`, `lead`, and `coder`.

**Architecture:** Execute this as a four-part removal. First collapse role/routing surfaces so no backend or bridge path still recognizes `reviewer`. Then remove `ReviewStatus`, `ReviewGate`, approval commands, persistence, and task-flow state transitions from the daemon. After the backend contract is gone, remove the remaining frontend reviewer selectors, badges, and review-status store plumbing. Finish with a documentation/rule sweep plus full verification so no active runtime surface still references reviewer or review gate.

**Tech Stack:** Rust/Tokio, Tauri, bridge sidecar Rust crate, React 19, Zustand, Bun, Cargo, git

---

## Scope Clarifications

- This execution starts from a dirty worktree. The current uncommitted changes in:
  - `src-tauri/src/daemon/orchestrator/review_gate.rs`
  - `src-tauri/src/daemon/orchestrator/tests.rs`
  - `src-tauri/src/daemon/state_tests.rs`
  are inside the scope of this feature and should be overwritten as needed by the final "remove reviewer/review gate" design.
- Do not preserve partial reviewer/review-gate behavior just because those files are already edited.
- Historical docs can still mention reviewer as a past design. This plan only requires updating active runtime rules/docs, not rewriting archival notes.

## Execution Contract

- Use `superpowers:test-driven-development` before each implementation task.
- Each task must start by adding or updating failing tests that describe the reviewer-free behavior.
- Each task must end with one focused commit.
- After each task commit, update `## CM Memory` with the real commit hash, verification evidence, and any execution rule we learned.
- Do not move to the next task until:
  - the task-specific red/green loop is complete
  - targeted verification commands pass
  - `git diff --check` is clean
  - the task row in `## CM Memory` is updated

## File Map

### Role protocol, routing, and bridge allowlists

- `src-tauri/src/daemon/role_config/role_protocol.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`
- `src-tauri/src/daemon/role_config/roles.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`
- `src-tauri/src/daemon/cmd.rs`
- `src-tauri/src/daemon/codex/session_event.rs`
- `src-tauri/src/mcp_tests.rs`
- `src-tauri/src/daemon/claude_sdk/process_tests.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `bridge/src/tools.rs`
- `bridge/src/tools_tests.rs`
- `bridge/src/channel_state.rs`
- `bridge/src/mcp_protocol.rs`
- `bridge/src/main.rs`

### Review gate, review status, and daemon task flow

- `src-tauri/src/daemon/orchestrator/review_gate.rs`
- `src-tauri/src/daemon/orchestrator/task_flow.rs`
- `src-tauri/src/daemon/orchestrator/tests.rs`
- `src-tauri/src/daemon/orchestrator/mod.rs`
- `src-tauri/src/daemon/task_graph/types.rs`
- `src-tauri/src/daemon/task_graph/store.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/state_persistence.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/gui_task.rs`
- `src-tauri/src/daemon/routing_display.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/state_snapshot_tests.rs`
- `src-tauri/src/daemon/state_task_snapshot_tests.rs`
- `src-tauri/src/daemon/types_tests.rs`
- `src-tauri/src/commands_artifact.rs`

### Frontend reviewer/review-status UI

- `src/components/AgentStatus/RoleSelect.tsx`
- `src/components/ReplyInput/TargetPicker.tsx`
- `src/components/ReplyInput/Footer.tsx`
- `src/components/MessagePanel/SourceBadge.tsx`
- `src/components/MessagePanel/surface-styles.ts`
- `src/components/TaskPanel/ReviewGateBadge.tsx`
- `src/components/TaskPanel/TaskHeader.tsx`
- `src/components/TaskPanel/SessionTree.tsx`
- `src/components/TaskPanel/view-model.ts`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/events.ts`
- `src/components/ClaudePanel/launch-request.test.ts`

### Active docs and rules

- `CLAUDE.md`
- `.claude/rules/` relevant role/routing docs
- `docs/superpowers/plans/2026-04-07-remove-reviewer-role.md`

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `c102cd50` | `manual diff review` | `cargo test claude_prompt --manifest-path src-tauri/Cargo.toml`; `cargo test role_config::roles::tests --manifest-path src-tauri/Cargo.toml`; `cargo test valid_roles_accepted --manifest-path src-tauri/Cargo.toml`; `cargo test reviewer_target_is_dropped_not_buffered --manifest-path src-tauri/Cargo.toml`; `cargo test upsert_mcp_server_marks_changed_when_role_differs --manifest-path src-tauri/Cargo.toml`; `cargo test build_inline_mcp_config_serializes_dimweave_server --manifest-path src-tauri/Cargo.toml`; `cargo test claude_sdk::process::tests --manifest-path src-tauri/Cargo.toml`; `cargo test reply_schema_has_enum_constraint --manifest-path bridge/Cargo.toml`; `cargo test reviewer_sender_is_dropped --manifest-path bridge/Cargo.toml`; `git diff --check` | Remove reviewer from all role/routing allowlists before touching review-gate state, otherwise bridge/daemon/frontend contracts drift. |
| Task 2 | `6acb46de` | `manual diff review` | `cargo test task_snapshot_after_reload_omits_review_status --manifest-path src-tauri/Cargo.toml`; `cargo test build_task_change_events_emits_task_and_review_when_task_changes --manifest-path src-tauri/Cargo.toml`; `cargo test build_task_context_events_includes_task_and_review_updates --manifest-path src-tauri/Cargo.toml`; `cargo test task_snapshot_serializes_camel_case --manifest-path src-tauri/Cargo.toml`; `cargo test buffered_route_message_no_longer_mentions_review_gate --manifest-path src-tauri/Cargo.toml`; `cargo test daemon::orchestrator::tests:: --manifest-path src-tauri/Cargo.toml`; `cargo test daemon::state::state_tests:: --manifest-path src-tauri/Cargo.toml`; `cargo test daemon::state::state_task_snapshot_tests:: --manifest-path src-tauri/Cargo.toml`; `cargo test daemon::state::state_snapshot_tests:: --manifest-path src-tauri/Cargo.toml`; `cargo test daemon::types::tests:: --manifest-path src-tauri/Cargo.toml`; `cargo test daemon::gui_task::tests:: --manifest-path src-tauri/Cargo.toml`; `cargo test daemon::routing_display::tests:: --manifest-path src-tauri/Cargo.toml`; `cargo test commands_artifact::tests:: --manifest-path src-tauri/Cargo.toml`; `git diff --check` | Review gate removal is not just deleting one file; it must include task graph fields, persistence, daemon commands, UI events, and buffer reasons. |
| Task 3 | `e142a580` | `manual diff review` | `bun test tests/task-panel-view-model.test.ts`; `bun test tests/task-store.test.ts`; `bun test src/components/ClaudePanel/launch-request.test.ts`; `bun test src/components/TaskContextPopover.test.tsx src/components/ReplyInput/index.test.tsx src/components/ReplyInput/task-session-guard.test.ts tests/task-panel-view-model.test.ts tests/task-store.test.ts src/components/ClaudePanel/launch-request.test.ts`; `bun run build`; `git diff --check` | Frontend removal must include both role selectors and passive review-status surfaces such as badges, view-model helpers, store event reducers, and any test fixtures that still carry legacy `reviewStatus` fields. |
| Task 4 | `docs: finalize reviewer removal rollout` | `manual diff review` | `rg -n "reviewer|ReviewStatus|review_gate" src-tauri/src bridge/src src/components src/stores CLAUDE.md .claude/rules -g'*.rs' -g'*.ts' -g'*.tsx' -g'*.md'`; `cargo test --manifest-path src-tauri/Cargo.toml`; `cargo test --manifest-path bridge/Cargo.toml`; `bun run build`; `git diff --check` | Final acceptance requires zero active reviewer/review-gate references in executable code paths plus green Cargo/Bun verification. |

### Task 1: Collapse role and routing surfaces to `user` / `lead` / `coder`

**Files:**
- Modify: `src-tauri/src/daemon/role_config/role_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt.rs`
- Modify: `src-tauri/src/daemon/role_config/roles.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/codex/session_event.rs`
- Modify: `src-tauri/src/mcp_tests.rs`
- Modify: `src-tauri/src/daemon/claude_sdk/process_tests.rs`
- Modify: `src-tauri/src/daemon/routing_behavior_tests.rs`
- Modify: `bridge/src/tools.rs`
- Modify: `bridge/src/tools_tests.rs`
- Modify: `bridge/src/channel_state.rs`
- Modify: `bridge/src/mcp_protocol.rs`
- Modify: `bridge/src/main.rs`

- [x] **Step 1: Write failing role/routing regressions**

Add or update tests so they require all of the following:

- `get_role("reviewer")` returns `None`
- `output_schema()` no longer includes `"reviewer"`
- daemon `AGENT_ROLES` excludes reviewer
- bridge reply targets and sender allowlists exclude reviewer
- role prompts no longer mention reviewer as a valid runtime role

- [x] **Step 2: Run the targeted regressions and confirm failure**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml role_config::roles::tests
cargo test --manifest-path src-tauri/Cargo.toml routing_behavior_tests
cargo test --manifest-path src-tauri/Cargo.toml mcp_tests
cargo test --manifest-path src-tauri/Cargo.toml claude_sdk::process::tests
cargo test --manifest-path bridge/Cargo.toml tools_tests
```

Expected: FAIL before implementation because reviewer is still a valid role/target.

- [x] **Step 3: Remove reviewer from backend/bridge role surfaces**

Implementation notes:

- remove reviewer branches from `role_protocol.rs`, `roles.rs`, and prompt examples
- shrink daemon role enums/allowlists to `lead` and `coder`
- remove reviewer from bridge send/target validation and MCP prompt text
- update tests to reflect the new three-role model

- [x] **Step 4: Re-run the targeted verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml role_config::roles::tests
cargo test --manifest-path src-tauri/Cargo.toml routing_behavior_tests
cargo test --manifest-path src-tauri/Cargo.toml mcp_tests
cargo test --manifest-path src-tauri/Cargo.toml claude_sdk::process::tests
cargo test --manifest-path bridge/Cargo.toml tools_tests
git diff --check
```

Expected: PASS.

- [x] **Step 5: Commit Task 1**

```bash
git add src-tauri/src/daemon/role_config/role_protocol.rs src-tauri/src/daemon/role_config/claude_prompt.rs src-tauri/src/daemon/role_config/roles.rs src-tauri/src/daemon/role_config/roles_tests.rs src-tauri/src/daemon/cmd.rs src-tauri/src/daemon/codex/session_event.rs src-tauri/src/mcp_tests.rs src-tauri/src/daemon/claude_sdk/process_tests.rs src-tauri/src/daemon/routing_behavior_tests.rs bridge/src/tools.rs bridge/src/tools_tests.rs bridge/src/channel_state.rs bridge/src/mcp_protocol.rs bridge/src/main.rs
git commit -m "refactor: remove reviewer role from routing surfaces"
```

- [x] **Step 6: Update `## CM Memory`**

Replace the Task 1 placeholders with the real commit hash, verification commands, and the learned routing invariant.

### Task 2: Remove review gate and review-status state from the daemon

**Files:**
- Delete: `src-tauri/src/daemon/orchestrator/review_gate.rs`
- Modify: `src-tauri/src/daemon/orchestrator/task_flow.rs`
- Modify: `src-tauri/src/daemon/orchestrator/tests.rs`
- Modify: `src-tauri/src/daemon/orchestrator/mod.rs`
- Modify: `src-tauri/src/daemon/task_graph/types.rs`
- Modify: `src-tauri/src/daemon/task_graph/store.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/daemon/state_task_flow.rs`
- Modify: `src-tauri/src/daemon/state_persistence.rs`
- Modify: `src-tauri/src/daemon/state_delivery.rs`
- Modify: `src-tauri/src/daemon/gui_task.rs`
- Modify: `src-tauri/src/daemon/routing_display.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/state_tests.rs`
- Modify: `src-tauri/src/daemon/state_snapshot_tests.rs`
- Modify: `src-tauri/src/daemon/state_task_snapshot_tests.rs`
- Modify: `src-tauri/src/daemon/types_tests.rs`
- Modify: `src-tauri/src/commands_artifact.rs`

- [x] **Step 1: Write failing daemon regressions for reviewer-free task flow**

Add or update tests so they require all of the following:

- task graph `Task` no longer stores `review_status`
- daemon state no longer exposes `active_review_gate()` or `lead_approve_review()`
- `DaemonCmd::ApproveReview` path is removed or rejected
- task UI events no longer emit `review_gate_changed`
- routing no longer returns `"review_gate"` as a buffer reason

- [x] **Step 2: Run the focused daemon regressions and confirm failure**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml daemon::orchestrator::tests::
cargo test --manifest-path src-tauri/Cargo.toml daemon::state_tests::
cargo test --manifest-path src-tauri/Cargo.toml daemon::state_task_snapshot_tests::
cargo test --manifest-path src-tauri/Cargo.toml daemon::state_snapshot_tests::
cargo test --manifest-path src-tauri/Cargo.toml daemon::types_tests::
```

Expected: FAIL before implementation because review gate and review-status state still exist.

- [x] **Step 3: Remove review gate, review status, and approval flow**

Implementation notes:

- delete `review_gate.rs` and remove its module import chain
- remove `ReviewStatus` from task graph types, store mutations, snapshots, and persistence
- remove `ActiveReviewGate`, `lead_approve_review`, `ApproveReview`, and `review_gate` buffer handling
- simplify `task_flow.rs` so lead and coder messages no longer pass through reviewer-based state transitions
- update daemon tests to assert direct lead/coder flow without review approval state

- [x] **Step 4: Re-run the daemon verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml daemon::orchestrator::tests::
cargo test --manifest-path src-tauri/Cargo.toml daemon::state_tests::
cargo test --manifest-path src-tauri/Cargo.toml daemon::state_task_snapshot_tests::
cargo test --manifest-path src-tauri/Cargo.toml daemon::state_snapshot_tests::
cargo test --manifest-path src-tauri/Cargo.toml daemon::types_tests::
git diff --check
```

Expected: PASS.

- [x] **Step 5: Commit Task 2**

```bash
git add src-tauri/src/daemon/orchestrator/task_flow.rs src-tauri/src/daemon/orchestrator/tests.rs src-tauri/src/daemon/orchestrator/mod.rs src-tauri/src/daemon/task_graph/types.rs src-tauri/src/daemon/task_graph/store.rs src-tauri/src/daemon/state.rs src-tauri/src/daemon/state_task_flow.rs src-tauri/src/daemon/state_persistence.rs src-tauri/src/daemon/state_delivery.rs src-tauri/src/daemon/gui_task.rs src-tauri/src/daemon/routing_display.rs src-tauri/src/daemon/mod.rs src-tauri/src/daemon/state_tests.rs src-tauri/src/daemon/state_snapshot_tests.rs src-tauri/src/daemon/state_task_snapshot_tests.rs src-tauri/src/daemon/types_tests.rs src-tauri/src/commands_artifact.rs
git rm src-tauri/src/daemon/orchestrator/review_gate.rs
git commit -m "refactor: remove review gate state machine"
```

- [x] **Step 6: Update `## CM Memory`**

Replace the Task 2 placeholders with the real commit hash, verification commands, and the learned daemon-state invariant.

### Task 3: Remove reviewer and review-status UI from the frontend

**Files:**
- Modify: `src/components/AgentStatus/RoleSelect.tsx`
- Modify: `src/components/ReplyInput/TargetPicker.tsx`
- Modify: `src/components/ReplyInput/Footer.tsx`
- Modify: `src/components/MessagePanel/SourceBadge.tsx`
- Modify: `src/components/MessagePanel/surface-styles.ts`
- Delete: `src/components/TaskPanel/ReviewGateBadge.tsx`
- Modify: `src/components/TaskPanel/TaskHeader.tsx`
- Modify: `src/components/TaskPanel/SessionTree.tsx`
- Modify: `src/components/TaskPanel/view-model.ts`
- Modify: `src/stores/task-store/types.ts`
- Modify: `src/stores/task-store/events.ts`
- Modify: `src/components/ClaudePanel/launch-request.test.ts`

- [x] **Step 1: Write failing frontend regressions**

Add or update tests so they require all of the following:

- reviewer no longer appears in role or target pickers
- source badge / surface styles no longer define reviewer presentation
- task-store types/events no longer carry `ReviewStatus` or `ReviewGateChangedPayload`
- Task panel and reply footer no longer import or render `ReviewGateBadge`

- [x] **Step 2: Run the focused frontend regressions and confirm failure**

Run:

```bash
bun test src/components/ClaudePanel/launch-request.test.ts
bun test tests/task-panel-view-model.test.ts
bun test src/stores/task-store/listener-setup.test.ts
```

Expected: FAIL before implementation because reviewer and review-status UI/state still exist.

- [x] **Step 3: Remove frontend reviewer/review-status surfaces**

Implementation notes:

- shrink role/target selectors to `auto`, `lead`, `coder` where applicable
- remove reviewer-specific labels and styles from message UI
- delete `ReviewGateBadge` and remove all imports/usages
- remove review-status fields from task-store types and reducers
- update tests to reflect the simplified role model

- [x] **Step 4: Re-run the frontend verification**

Run:

```bash
bun test src/components/ClaudePanel/launch-request.test.ts
bun test tests/task-panel-view-model.test.ts
bun test src/stores/task-store/listener-setup.test.ts
bun run build
git diff --check
```

Expected: PASS.

- [x] **Step 5: Commit Task 3**

```bash
git add src/components/AgentStatus/RoleSelect.tsx src/components/ReplyInput/TargetPicker.tsx src/components/ReplyInput/Footer.tsx src/components/MessagePanel/SourceBadge.tsx src/components/MessagePanel/surface-styles.ts src/components/TaskPanel/TaskHeader.tsx src/components/TaskPanel/SessionTree.tsx src/components/TaskPanel/view-model.ts src/stores/task-store/types.ts src/stores/task-store/events.ts src/components/ClaudePanel/launch-request.test.ts
git rm src/components/TaskPanel/ReviewGateBadge.tsx
git commit -m "refactor: remove reviewer UI surfaces"
```

- [x] **Step 6: Update `## CM Memory`**

Replace the Task 3 placeholders with the real commit hash, verification commands, and the learned frontend invariant.

### Task 4: Sweep active docs/rules and run final acceptance verification

**Files:**
- Modify: `CLAUDE.md`
- Modify: `.claude/rules/` relevant role/routing files
- Modify: `docs/superpowers/plans/2026-04-07-remove-reviewer-role.md`

- [x] **Step 1: Write failing/strict sweep checks**

Before editing docs, run a repository sweep and capture every active executable/runtime reference to:

- `reviewer`
- `ReviewStatus`
- `review_gate`

Ignore archival/historical docs only if they are clearly not active runtime instructions.

- [x] **Step 2: Remove active reviewer/review-gate docs/rules references**

Update active runtime docs/rules so they describe the final three-role model and no longer instruct the system to route through reviewer or review gate.

- [x] **Step 3: Run final acceptance verification**

Run:

```bash
rg -n "reviewer|ReviewStatus|review_gate" src-tauri/src bridge/src src/components src/stores CLAUDE.md .claude/rules
cargo test --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path bridge/Cargo.toml
bun run build
git diff --check
```

Expected:

- `rg` returns no active runtime references to reviewer/review-gate symbols
- Rust and bridge tests pass
- frontend build passes
- diff formatting is clean

- [x] **Step 4: Commit Task 4**

```bash
git add CLAUDE.md .claude/rules docs/superpowers/plans/2026-04-07-remove-reviewer-role.md
git commit -m "docs: finalize reviewer removal rollout"
```

- [x] **Step 5: Update `## CM Memory`**

Replace the Task 4 placeholders with the real commit hash, verification commands, and the learned rollout invariant.
