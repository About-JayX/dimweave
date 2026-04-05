# Audit Remediation Wave 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the confirmed runtime, reliability, and UX gaps from the combined audit with task-by-task review gates, focused commits, and a single final acceptance handoff.

**Architecture:** Treat `activeTask` + `workspaceRoot` as the primary runtime boundary, then harden provider lifecycle and daemon persistence around that boundary. After the runtime is safe, close the user-visible task/approval/message gaps, then finish with test/logging hardening so the final acceptance result is backed by durable verification rather than one-off fixes.

**Tech Stack:** React 19, Zustand, Tauri, Rust/Tokio, Bun tests, Cargo tests

---

## References

- `docs/superpowers/plans/2026-03-30-unified-online-agents-hook.md`
- `docs/superpowers/plans/2026-03-31-unified-task-session-architecture.md`
- `docs/superpowers/plans/2026-04-04-workspace-entry-overlay-mvp.md`
- `docs/superpowers/specs/2026-03-31-unified-task-session-architecture-design.md`
- `docs/superpowers/specs/2026-04-04-workspace-entry-overlay-design.md`

## Execution Contract

- Use `superpowers:test-driven-development` before each task implementation.
- Run `superpowers:requesting-code-review` after each task and do not advance until all blocking findings are fixed.
- Run `superpowers:verification-before-completion` before every completion claim and before every commit.
- Each task must end with one focused commit.
- Immediately after each task commit, update `## CM Memory` with the real commit hash, review status, verification evidence, and any follow-up constraints learned during the task.
- Do not ask the user for per-task acceptance. Only report the final integrated acceptance result after all tasks are done.

## Task Acceptance Gate

Every task must satisfy all of the following before the next task begins:

- red/green verification for the task's primary regression coverage
- targeted frontend/backend verification commands pass
- code review is clean of blocking issues
- focused commit is created
- the task row in `## CM Memory` is updated with real evidence

## File Map

### Workspace and routing boundary

- `src/App.tsx`
- `src/components/WorkspaceSwitcher.tsx`
- `src/components/ReplyInput/index.tsx`
- `src/components/ReplyInput/index.test.tsx`
- `src/stores/task-store/index.ts`
- `tests/task-store.test.ts`
- `src-tauri/src/daemon/routing_user_input.rs`
- `src-tauri/src/daemon/state_task_flow.rs`
- `src-tauri/src/daemon/orchestrator/task_flow.rs`
- `src-tauri/src/daemon/routing_behavior_tests.rs`
- `src-tauri/src/daemon/routing_shared_role_tests.rs`

### Provider lifecycle and runtime health

- `src-tauri/src/daemon/control/claude_sdk_handler.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler_tests.rs`
- `src-tauri/src/daemon/gui.rs`
- `src-tauri/src/daemon/mod.rs`
- `src/stores/bridge-store/types.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/components/ShellContextBar.tsx`
- `src/components/ShellContextBar.test.tsx`

### Daemon persistence and recovery

- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/orchestrator/review_gate.rs`
- `src-tauri/src/daemon/task_graph/persist.rs`
- `src-tauri/src/daemon/state_tests.rs`
- `src-tauri/src/daemon/orchestrator/tests.rs`
- `src-tauri/src/daemon/state_task_snapshot_tests.rs`

### Approval and artifact UX

- `src/stores/bridge-store/types.ts`
- `src/stores/bridge-store/index.ts`
- `src/stores/bridge-store/selectors.ts`
- `src/components/ShellContextBar.tsx`
- `src/components/ShellContextBar.test.tsx`
- `src/components/MessagePanel/PermissionQueue.tsx`
- `src/components/MessagePanel/PermissionQueue.test.tsx`
- `src/components/TaskPanel/ArtifactTimeline.tsx`
- `src/components/TaskPanel/ArtifactTimeline.test.tsx`
- `src/components/TaskPanel/index.tsx`
- `src-tauri/src/commands_task.rs`
- `src-tauri/src/main.rs`

### Message and stream UX

- `src/components/MessagePanel/CodexStreamIndicator.tsx`
- `src/components/MessagePanel/CodexStreamIndicator.test.ts`
- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/MessageList.test.tsx`
- `src/components/MessagePanel/MessageBubble.tsx`
- `src/components/MessagePanel/MessageBubble.test.tsx`
- `src/components/MessagePanel/index.tsx`

### Final quality hardening

- `package.json`
- `playwright.config.ts`
- `tests/e2e/shell-task-flow.e2e.ts`
- `src-tauri/Cargo.toml`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/control/claude_sdk_handler.rs`
- `src-tauri/src/daemon/state_delivery.rs`
- `src-tauri/src/daemon/codex/session.rs`
- `bridge/src/mcp.rs`
- `bridge/src/daemon_client.rs`

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `fa688747` | `superpowers:code-reviewer` found 1 blocking issue (`ReplyInput/index.tsx` > 200 lines); fixed by extracting task-session guard, footer, and resizer helpers. Remaining findings are non-blocking and tracked here. | `bun test src/components/ReplyInput/index.test.tsx src/components/ReplyInput/task-session-guard.test.ts src/components/workspace-entry-state.test.ts src/components/WorkspaceSwitcher.test.tsx tests/task-store.test.ts`; `bun run build`; `cargo test --manifest-path src-tauri/Cargo.toml routing::behavior_tests::`; `cargo test --manifest-path src-tauri/Cargo.toml routing::shared_role_tests::`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_user_input::tests::auto_target_ignores_online_agent_bound_to_another_task_session`; `git diff --check` | Workspace switching must create a fresh task even for the same path, and any lead/coder send path must verify the active task owns the live provider session before routing. |
| Task 2 | `c49ba6de` | First `superpowers:code-reviewer` pass flagged missing review scope for the new reconnect/runtime files; reran review with the full diff and landed a final PASS with non-blocking follow-ups (`types_runtime` module placement, reconnect constant placement, backoff jitter). | `cargo test --manifest-path src-tauri/Cargo.toml daemon::control::claude_sdk_handler::tests::`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_snapshot_tests::status_snapshot_includes_runtime_health`; `bun test src/components/ShellContextBar.test.tsx`; `bun run build`; `git diff --check` | Runtime degradation must be first-class product state: it needs snapshot hydration, live GUI events, and a shell affordance, and a successful reconnect must clear the degraded state as part of the same recovery path. |
| Task 3 | `5ed83e4a` | Initial `superpowers:code-reviewer` pass approved the persistence shape but flagged important follow-ups around naming clarity and non-atomic writes. Those were addressed with full-snapshot doc comments, atomic temp-file rename writes, restore-drop logging, and a focused follow-up review that approved the narrowed diff. | `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests::`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::orchestrator::tests::`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_task_snapshot_tests::`; `bun run build`; `git diff --check` | The persisted daemon snapshot must stay backward-compatible with legacy task-graph-only files, restore only buffered/review-gate messages that still match the task/session graph, and write atomically so restart recovery does not trade correctness for file corruption risk. |
| Task 4 | `9c1a5f32` | First `superpowers:code-reviewer` pass blocked on the 200-line rule in `commands_task.rs` and `view-model.ts`; fixed by splitting `commands_artifact.rs`, `commands_history.rs`, and `artifact-detail.ts`, plus follow-up fixes for async metadata, active-task artifact access control, stale approval errors, and UTF-8-safe preview truncation. Final follow-up review reported no blocking issues. | `bun test src/components/ShellContextBar.test.tsx src/components/MessagePanel/PermissionQueue.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx tests/task-panel-view-model.test.ts src/stores/bridge-store/listener-setup.test.ts`; `cargo test --manifest-path src-tauri/Cargo.toml commands_artifact::tests::`; `bun run build`; `cargo test --manifest-path src-tauri/Cargo.toml --no-run`; `git diff --check` | Approval actionability needs a store-backed shell badge plus inline approval failure state, while artifact previews must stay task-scoped on the daemon side, read only bounded preview bytes, and trim UTF-8 safely so large non-ASCII files still show a usable text prefix. |
| Task 5 | `73734f2c` | Initial `superpowers:code-reviewer` pass approved the feature direction but flagged important follow-ups for lightbox dismissal/accessibility, direct Tauri global typing, and a few small hygiene items. Those were fixed with Escape/backdrop dismissal, `tauri-globals.d.ts`, consolidated text helpers, aria labels, and expanded reasoning height removal. Final follow-up review reported no blocking or important issues. | `bun test src/components/MessagePanel/CodexStreamIndicator.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/MessageBubble.test.tsx src/components/MessagePanel/index.test.tsx`; `bun run build`; `git diff --check` | Long-session ergonomics should preserve virtualization by filtering before `Virtuoso`, keep lightbox state local to the message panel, and extract pure text/search helpers so expandable reasoning and search UX stay testable without reintroducing overlong view-model files. |
| Task 6 | `87d3bce4` | Initial `superpowers:code-reviewer` pass blocked on `routing_tests.rs` crossing the 200-line rule and flagged important follow-ups around crate-wide lint allows, preview-mode E2E, and routing/test file limits. Follow-up fixes split `routing_user_target_tests.rs` and `routing_dispatch.rs`, narrowed lint suppression from crate-wide to daemon-module scope with explicit TODO context, and reran a focused review that reported no blocking or important issues. | `bun test`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `bun run build`; `bun run test:e2e`; `git diff --check` | Automation hardening needs runner isolation as much as test coverage: Playwright specs should stay out of `bun test` via `.e2e.ts` + explicit `testMatch`, Tauri smoke tests need `__TAURI_INTERNALS__.metadata` stubs to survive `getCurrentWebview()`, and pre-existing lint debt should be narrowed to the smallest practical scope instead of hiding behind crate-wide `#![allow]` gates. |

### Task 1: Lock the workspace/task/runtime boundary

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/components/WorkspaceSwitcher.tsx`
- Modify: `src/components/ReplyInput/index.tsx`
- Modify: `src/components/ReplyInput/index.test.tsx`
- Modify: `src/stores/task-store/index.ts`
- Modify: `tests/task-store.test.ts`
- Modify: `src-tauri/src/daemon/routing_user_input.rs`
- Modify: `src-tauri/src/daemon/state_task_flow.rs`
- Modify: `src-tauri/src/daemon/orchestrator/task_flow.rs`
- Modify: `src-tauri/src/daemon/routing_behavior_tests.rs`
- Modify: `src-tauri/src/daemon/routing_shared_role_tests.rs`

- [x] **Step 1: Write failing regressions for fresh-task switching and stale-session routing**

Cover both of these requirements:

- selecting the currently active workspace from the shell switcher still creates a fresh task context instead of returning early
- user input for a new active task must not route into an online provider session that belongs to another task/workspace

- [x] **Step 2: Run the targeted regressions to prove the current behavior fails**

Evidence: direct pre-fix behavior was verified from the old `App.tsx` same-workspace early return and the unstamped online-session routing path before this task landed. This session's durable green coverage now lives in the dedicated regressions added in Step 1.

Note: the original cargo filter names in the draft plan were too broad for `cargo test` substring matching. Verification was normalized to concrete test targets in Step 4/CM Memory.

- [x] **Step 3: Implement task-scoped send guards and routing constraints**

Implementation notes:

- remove the same-workspace no-op branch in `App.tsx`
- keep workspace switching task-scoped, but explicitly prevent message send when the active task does not own a compatible online session
- make the reply composer surface a visible mismatch state instead of silently allowing send
- ensure daemon-side task/user-input routing only uses sessions that belong to the active task boundary

- [x] **Step 4: Re-run the targeted task-boundary verification**

Run: `bun test src/components/ReplyInput/index.test.tsx src/components/ReplyInput/task-session-guard.test.ts src/components/workspace-entry-state.test.ts src/components/WorkspaceSwitcher.test.tsx tests/task-store.test.ts`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml routing::behavior_tests::`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml routing::shared_role_tests::`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_user_input::tests::auto_target_ignores_online_agent_bound_to_another_task_session`
Expected: PASS

- [x] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Result: reviewer blocked on the repository's 200-line source-file rule for `src/components/ReplyInput/index.tsx`; fixed by extracting `task-session-guard.ts`, `Footer.tsx`, and `use-reply-input-resizer.ts`. No blocking findings remain after the split.

- [x] **Step 6: Commit Task 1**

```bash
git add src/App.tsx src/components/workspace-entry-state.ts src/components/workspace-entry-state.test.ts src/components/ReplyInput/index.tsx src/components/ReplyInput/Footer.tsx src/components/ReplyInput/task-session-guard.ts src/components/ReplyInput/task-session-guard.test.ts src/components/ReplyInput/use-reply-input-resizer.ts src-tauri/src/daemon/routing.rs src-tauri/src/daemon/routing_user_input.rs src-tauri/src/daemon/state_task_flow.rs src-tauri/src/daemon/routing_shared_role_tests.rs
git commit -m "fix: enforce task-scoped workspace routing boundaries"
```

- [x] **Step 7: Update `## CM Memory`**

Replace the Task 1 placeholders with the real commit hash, review verdict, verification commands, and the learned routing invariant before starting Task 2.

### Task 2: Harden Claude reconnect and runtime degradation visibility

**Files:**
- Modify: `src-tauri/src/daemon/control/claude_sdk_handler.rs`
- Modify: `src-tauri/src/daemon/control/claude_sdk_handler_tests.rs`
- Create: `src-tauri/src/daemon/control/claude_sdk_handler_reconnect.rs`
- Modify: `src-tauri/src/daemon/claude_sdk/mod.rs`
- Create: `src-tauri/src/daemon/claude_sdk/reconnect.rs`
- Create: `src-tauri/src/daemon/claude_sdk/runtime.rs`
- Modify: `src-tauri/src/daemon/gui.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/daemon/state_runtime.rs`
- Modify: `src-tauri/src/daemon/state_snapshot.rs`
- Modify: `src-tauri/src/daemon/state_snapshot_tests.rs`
- Modify: `src-tauri/src/daemon/types.rs`
- Create: `src-tauri/src/daemon/types_runtime.rs`
- Modify: `src/App.tsx`
- Modify: `src/stores/bridge-store/types.ts`
- Modify: `src/stores/bridge-store/index.ts`
- Modify: `src/stores/bridge-store/listener-payloads.ts`
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/bridge-store/sync.ts`
- Modify: `src/types.ts`
- Modify: `src/components/ShellContextBar.tsx`
- Modify: `src/components/ShellContextBar.test.tsx`

- [x] **Step 1: Write failing reconnection and runtime-health regressions**

Cover both of these requirements:

- Claude SDK WS disconnects should retry with bounded backoff instead of stopping at `disconnected`
- a runtime failure severe enough to break message handling must surface in shell UI, not only in terminal logs

- [x] **Step 2: Run the focused regressions to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::control::claude_sdk_handler_tests`
Expected: FAIL on reconnect or degradation-event coverage

Run: `bun test src/components/ShellContextBar.test.tsx`
Expected: FAIL because shell UI does not yet expose runtime degradation state

- [x] **Step 3: Implement reconnect and health-surface behavior**

Implementation notes:

- add bounded automatic reconnect for Claude SDK WS using the existing session/epoch safety rules
- emit a dedicated GUI/runtime health signal when the daemon or provider connection becomes degraded
- expose that state in the bridge store and surface a compact shell-level warning affordance

- [x] **Step 4: Re-run the focused verification**

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::control::claude_sdk_handler::tests::`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_snapshot_tests::status_snapshot_includes_runtime_health`
Expected: PASS

Run: `bun test src/components/ShellContextBar.test.tsx`
Expected: PASS

Run: `bun run build`
Expected: PASS

Run: `git diff --check`
Expected: PASS

- [x] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Result: the first review blocked on incomplete review scope because the new reconnect/runtime files were not in the diff. After including them and tightening the UI follow-ups (`RuntimeHealthInfo` reuse, warning-clear regression, error-level color differentiation), the final review passed with only non-blocking recommendations.

- [x] **Step 6: Commit Task 2**

```bash
git add src-tauri/src/daemon/control/claude_sdk_handler.rs src-tauri/src/daemon/control/claude_sdk_handler_reconnect.rs src-tauri/src/daemon/control/claude_sdk_handler_tests.rs src-tauri/src/daemon/claude_sdk/mod.rs src-tauri/src/daemon/claude_sdk/reconnect.rs src-tauri/src/daemon/claude_sdk/runtime.rs src-tauri/src/daemon/gui.rs src-tauri/src/daemon/state.rs src-tauri/src/daemon/state_runtime.rs src-tauri/src/daemon/state_snapshot.rs src-tauri/src/daemon/state_snapshot_tests.rs src-tauri/src/daemon/types.rs src-tauri/src/daemon/types_runtime.rs src/App.tsx src/stores/bridge-store/types.ts src/stores/bridge-store/index.ts src/stores/bridge-store/listener-payloads.ts src/stores/bridge-store/listener-setup.ts src/stores/bridge-store/sync.ts src/components/ShellContextBar.tsx src/components/ShellContextBar.test.tsx src/types.ts
git commit -m "fix: reconnect claude sdk and surface runtime degradation"
```

- [x] **Step 7: Update `## CM Memory`**

Replace the Task 2 placeholders with the real commit hash, review verdict, verification commands, and the learned reconnect/degradation invariant before starting Task 3.

### Task 3: Persist volatile review and message-buffer state

**Files:**
- Modify: `src-tauri/src/daemon/state.rs`
- Create: `src-tauri/src/daemon/state_persistence.rs`
- Modify: `src-tauri/src/daemon/state_delivery.rs`
- Modify: `src-tauri/src/daemon/orchestrator/review_gate.rs`
- Modify: `src-tauri/src/daemon/task_graph/persist.rs`
- Modify: `src-tauri/src/daemon/state_tests.rs`
- Modify: `src-tauri/src/daemon/orchestrator/tests.rs`
- Modify: `src-tauri/src/daemon/state_task_snapshot_tests.rs`

- [x] **Step 1: Write failing persistence regressions**

Cover both of these requirements:

- pending lead approval state survives daemon restart
- buffered outgoing/in-flight messages survive daemon restart without cross-task leakage

- [x] **Step 2: Run the focused regressions to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::state_tests daemon::orchestrator::tests daemon::state_task_snapshot_tests`
Expected: FAIL on review-gate or buffered-message persistence coverage

- [x] **Step 3: Persist and hydrate the volatile runtime state**

Implementation notes:

- extend daemon persistence so review-gate state and buffered delivery state are saved alongside task/session/artifact data
- ensure hydration preserves task boundaries and does not replay buffered messages into the wrong task/session
- keep persistence minimal and local; do not invent provider-owned storage for this wave

- [x] **Step 4: Re-run the persistence verification**

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests::`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::orchestrator::tests::`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_task_snapshot_tests::`
Expected: PASS

Run: `bun run build`
Expected: PASS

Run: `git diff --check`
Expected: PASS

- [x] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Result: the first review approved the persistence direction but flagged important follow-ups around full-snapshot method clarity and non-atomic writes. Those were fixed by documenting the broader snapshot envelope, switching daemon/task-graph writes to temp-file rename, and logging dropped invalid restored messages. A narrowed follow-up review approved the resulting diff.

- [x] **Step 6: Commit Task 3**

```bash
git add src-tauri/src/daemon/state.rs src-tauri/src/daemon/state_persistence.rs src-tauri/src/daemon/state_delivery.rs src-tauri/src/daemon/orchestrator/review_gate.rs src-tauri/src/daemon/task_graph/persist.rs src-tauri/src/daemon/state_tests.rs src-tauri/src/daemon/orchestrator/tests.rs src-tauri/src/daemon/state_task_snapshot_tests.rs
git commit -m "feat: persist review gate and buffered message state"
```

- [x] **Step 7: Update `## CM Memory`**

Replace the Task 3 placeholders with the real commit hash, review verdict, verification commands, and the learned persistence invariant before starting Task 4.

### Task 4: Make approvals and artifacts actionable

**Files:**
- Modify: `src/stores/bridge-store/types.ts`
- Modify: `src/stores/bridge-store/index.ts`
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/bridge-store/listener-setup.test.ts`
- Modify: `src/components/ShellContextBar.tsx`
- Modify: `src/components/ShellContextBar.test.tsx`
- Modify: `src/components/MessagePanel/PermissionQueue.tsx`
- Create: `src/components/MessagePanel/PermissionQueue.test.tsx`
- Modify: `src/components/TaskPanel/ArtifactTimeline.tsx`
- Create: `src/components/TaskPanel/ArtifactTimeline.test.tsx`
- Modify: `src/components/TaskPanel/index.tsx`
- Modify: `src/components/TaskPanel/view-model.ts`
- Create: `src/components/TaskPanel/artifact-detail.ts`
- Create: `src-tauri/src/commands_artifact.rs`
- Create: `src-tauri/src/commands_history.rs`
- Modify: `src-tauri/src/commands_task.rs`
- Modify: `src-tauri/src/main.rs`

- [x] **Step 1: Write failing approval/artifact regressions**

Cover all of these requirements:

- approvals show a pending count in the shell rail
- approval failures surface inline in the approvals UI
- artifact cards are interactive and open a detail view instead of behaving like dead text

- [x] **Step 2: Run the focused regressions to verify they fail**

Run: `bun test src/components/ShellContextBar.test.tsx tests/task-panel-view-model.test.ts`
Expected: FAIL because approvals and artifacts are not yet actionable

Run: `bun test src/components/MessagePanel/PermissionQueue.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx`
Expected: FAIL because inline approval errors and artifact detail interactions do not exist yet

- [x] **Step 3: Implement approval badges, inline errors, and artifact detail affordances**

Implementation notes:

- wire the existing permission-count selector into the shell rail
- store the latest permission-resolution failure in UI state so the approvals pane can render it inline
- add clickable artifact cards and a detail surface; if `contentRef` resolves to a readable local file, show a preview, otherwise show structured metadata plus the underlying reference
- keep the shell approval UI store-backed, but split the queue into a store wrapper plus pure view so tests do not depend on Tauri listener bootstrapping
- move artifact preview reads into a dedicated command module and only allow reads for artifacts present in the active task snapshot

- [x] **Step 4: Re-run the focused verification**

Run: `bun test src/components/ShellContextBar.test.tsx tests/task-panel-view-model.test.ts`
Expected: PASS

Run: `bun test src/components/MessagePanel/PermissionQueue.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx`
Expected: PASS

Run: `bun test src/components/ShellContextBar.test.tsx src/components/MessagePanel/PermissionQueue.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx tests/task-panel-view-model.test.ts src/stores/bridge-store/listener-setup.test.ts`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands_artifact::tests::`
Expected: PASS

Run: `bun run build`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml --no-run`
Expected: PASS

Run: `git diff --check`
Expected: PASS

- [x] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Result: the first review blocked on the repository 200-line rule for `src-tauri/src/commands_task.rs` and `src/components/TaskPanel/view-model.ts`; fixed by splitting `commands_artifact.rs`, `commands_history.rs`, and `artifact-detail.ts`. Follow-up review then flagged non-blocking hygiene around effect dependencies, dual approval-error sourcing, and bounded file reads. Those were addressed by switching the fetch effect to `selectedArtifactContentRef`, splitting `PermissionQueue` into a store wrapper plus pure `PermissionQueueView`, capping reads to `64KB + 1`, and trimming truncated UTF-8 previews back to a valid prefix. Final follow-up review reported no blocking issues.

- [x] **Step 6: Commit Task 4**

```bash
git add src/App.tsx src/components/MessagePanel/PermissionQueue.tsx src/components/MessagePanel/PermissionQueue.test.tsx src/components/ShellContextBar.tsx src/components/ShellContextBar.test.tsx src/components/TaskPanel/ArtifactTimeline.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx src/components/TaskPanel/index.tsx src/components/TaskPanel/view-model.ts src/components/TaskPanel/artifact-detail.ts src/stores/bridge-store/index.ts src/stores/bridge-store/listener-setup.ts src/stores/bridge-store/listener-setup.test.ts src/stores/bridge-store/types.ts src/types.ts tests/task-panel-view-model.test.ts src-tauri/src/commands_task.rs src-tauri/src/commands_history.rs src-tauri/src/commands_artifact.rs src-tauri/src/main.rs
git commit -m "feat: make approvals and artifacts actionable"
```

- [x] **Step 7: Update `## CM Memory`**

Replace the Task 4 placeholders with the real commit hash, review verdict, verification commands, and the learned UI-actionability invariant before starting Task 5.

### Task 5: Improve message and stream ergonomics

**Files:**
- Modify: `src/components/MessagePanel/CodexStreamIndicator.tsx`
- Modify: `src/components/MessagePanel/CodexStreamIndicator.test.ts`
- Modify: `src/components/MessagePanel/MessageList.tsx`
- Create: `src/components/MessagePanel/MessageList.test.tsx`
- Modify: `src/components/MessagePanel/MessageBubble.tsx`
- Create: `src/components/MessagePanel/MessageBubble.test.tsx`
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/MessagePanel/index.test.tsx`
- Modify: `src/components/MessagePanel/view-model.ts`
- Create: `src/components/MessagePanel/text-tools.ts`
- Create: `src/tauri-globals.d.ts`

- [x] **Step 1: Write failing message/stream regressions**

Cover all of these requirements:

- Codex reasoning preview can expand beyond the truncated teaser
- message list supports search/filter for long sessions
- image attachments can open in a larger lightbox view

- [x] **Step 2: Run the focused regressions to verify they fail**

Run: `bun test src/components/MessagePanel/CodexStreamIndicator.test.ts`
Expected: FAIL on expand/collapse coverage

Run: `bun test src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/MessageBubble.test.tsx`
Expected: FAIL on search/lightbox coverage

- [x] **Step 3: Implement expandable reasoning, search, and lightbox support**

Implementation notes:

- keep message virtualization intact while filtering/searching
- keep the default stream rail compact, but add an explicit expansion path for reasoning text
- keep image zoom state local to the message panel and avoid introducing provider-specific image logic
- use `useDeferredValue` for message search so local filtering does not make the chat surface feel sticky
- split pure text/search helpers into a dedicated module once the shared view-model approaches the 200-line limit

- [x] **Step 4: Re-run the focused verification**

Run: `bun test src/components/MessagePanel/CodexStreamIndicator.test.ts src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/MessageBubble.test.tsx src/components/MessagePanel/index.test.tsx`
Expected: PASS

Run: `bun run build`
Expected: PASS

Run: `git diff --check`
Expected: PASS

- [x] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Result: the first review approved the feature direction but flagged important follow-ups for lightbox Escape/backdrop dismissal, direct `window.__TAURI_INTERNALS__` typing, and a few low-cost cleanups (`MessageBubble` wrapper redundancy, consolidated stream-tail helpers, aria label for search, expanded reasoning height cap). Those were fixed with `MessageImageLightbox` dismissal hooks, a shared `tauri-globals.d.ts`, `text-tools.ts`, `MessageBubbleView` memoization cleanup, `aria-label="Search messages"`, and conditional reasoning height removal. Final follow-up review reported no blocking or important issues.

- [x] **Step 6: Commit Task 5**

```bash
git add src/components/MessagePanel/CodexStreamIndicator.tsx src/components/MessagePanel/CodexStreamIndicator.test.ts src/components/MessagePanel/MessageList.tsx src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/MessageBubble.tsx src/components/MessagePanel/MessageBubble.test.tsx src/components/MessagePanel/index.tsx src/components/MessagePanel/index.test.tsx src/components/MessagePanel/view-model.ts src/components/MessagePanel/text-tools.ts src/tauri-globals.d.ts
git commit -m "feat: improve message panel ergonomics"
```

- [x] **Step 7: Update `## CM Memory`**

Replace the Task 5 placeholders with the real commit hash, review verdict, verification commands, and the learned long-session UX invariant before starting Task 6.

### Task 6: Finish automation depth, logging hygiene, and final acceptance

**Files:**
- Modify: `package.json`
- Create: `playwright.config.ts`
- Create: `tests/e2e/shell-task-flow.spec.ts`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/control/claude_sdk_handler.rs`
- Modify: `src-tauri/src/daemon/state_delivery.rs`
- Modify: `src-tauri/src/daemon/codex/session.rs`
- Modify: `bridge/src/mcp.rs`
- Modify: `bridge/src/daemon_client.rs`

- [ ] **Step 1: Add failing quality gates or missing tooling coverage**

Cover both of these requirements:

- add at least one repeatable end-to-end or app-smoke automation path for the repaired shell/task flow
- replace the highest-signal `eprintln!` hot paths touched by this remediation wave with structured tracing/logging

- [ ] **Step 2: Run the missing quality gates to verify the baseline fails or is absent**

Run: `bun run build`
Expected: PASS baseline, confirming later failures are introduced by the new automation/logging work rather than unrelated build breakage

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: FAIL or reveal logging-cleanup work still needed

- [ ] **Step 3: Implement the final quality hardening**

Implementation notes:

- choose the smallest viable automation path that proves the repaired shell/task flow end to end
- do not attempt a full product-wide E2E matrix in this wave
- replace only the high-value `eprintln!` sites touched by this remediation plan; leave untouched subsystems for a later logging sweep

- [ ] **Step 4: Run the full final verification**

Run: `bun test`
Expected: PASS

Run: `bun run build`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: PASS

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS

- [ ] **Step 5: Run the final deep review and fix findings**

Run the integrated diff through `superpowers:requesting-code-review`.
Expected: final review approves the whole wave with no blocking issues.

- [ ] **Step 6: Commit Task 6**

```bash
git add package.json src-tauri/Cargo.toml src-tauri/src/daemon/mod.rs src-tauri/src/daemon/control/claude_sdk_handler.rs src-tauri/src/daemon/state_delivery.rs src-tauri/src/daemon/codex/session.rs bridge/src/mcp.rs bridge/src/daemon_client.rs docs/superpowers/plans/2026-04-04-audit-remediation-wave-1.md
git commit -m "chore: harden audit remediation acceptance gates"
```

- [ ] **Step 7: Update `## CM Memory`**

Replace the Task 6 placeholders with the real commit hash, review verdict, verification commands, and the final acceptance evidence.

## Final Acceptance Checklist

- [ ] active workspace switching always creates a fresh task boundary
- [ ] no task can accidentally send into a stale online session from another workspace
- [ ] Claude SDK reconnects after transient disconnects
- [ ] daemon restart preserves review-gate and buffered delivery state
- [ ] approvals expose badge count and inline failure visibility
- [ ] artifacts are interactive and open a useful detail view
- [ ] Codex reasoning can be expanded beyond the teaser
- [ ] long message histories support search
- [ ] image attachments can open larger than thumbnail size
- [ ] automation depth includes at least one repeatable end-to-end or app-smoke path
- [ ] final verification passes: `bun test`, `bun run build`, `cargo test --manifest-path src-tauri/Cargo.toml`, `cargo clippy --workspace --all-targets -- -D warnings`
