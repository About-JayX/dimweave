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
- `tests/e2e/shell-task-flow.spec.ts`
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
| Task 1 | `TBD` | Pending | Pending | Workspace switch must never reuse a live session from another task/workspace. |
| Task 2 | `TBD` | Pending | Pending | Claude reconnect and runtime degradation must become visible product state, not terminal-only state. |
| Task 3 | `TBD` | Pending | Pending | Review gate and buffered delivery must survive daemon restart. |
| Task 4 | `TBD` | Pending | Pending | Approval visibility and artifact interaction should be solved together so task context becomes actionable. |
| Task 5 | `TBD` | Pending | Pending | Long sessions need search, expandable previews, and image zoom without destabilizing virtualization. |
| Task 6 | `TBD` | Pending | Pending | Final acceptance requires automation depth and lower logging entropy, not just passing local smoke tests. |

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

- [ ] **Step 1: Write failing regressions for fresh-task switching and stale-session routing**

Cover both of these requirements:

- selecting the currently active workspace from the shell switcher still creates a fresh task context instead of returning early
- user input for a new active task must not route into an online provider session that belongs to another task/workspace

- [ ] **Step 2: Run the targeted regressions to prove the current behavior fails**

Run: `bun test src/components/ReplyInput/index.test.tsx src/components/WorkspaceSwitcher.test.tsx tests/task-store.test.ts`
Expected: FAIL on the fresh-task and mismatch-send regressions

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_behavior_tests daemon::routing_shared_role_tests`
Expected: FAIL on stale online-session delivery coverage

- [ ] **Step 3: Implement task-scoped send guards and routing constraints**

Implementation notes:

- remove the same-workspace no-op branch in `App.tsx`
- keep workspace switching task-scoped, but explicitly prevent message send when the active task does not own a compatible online session
- make the reply composer surface a visible mismatch state instead of silently allowing send
- ensure daemon-side task/user-input routing only uses sessions that belong to the active task boundary

- [ ] **Step 4: Re-run the targeted task-boundary verification**

Run: `bun test src/components/ReplyInput/index.test.tsx src/components/WorkspaceSwitcher.test.tsx tests/task-store.test.ts`
Expected: PASS

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::routing_behavior_tests daemon::routing_shared_role_tests`
Expected: PASS

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking review findings remain for Task 1.

- [ ] **Step 6: Commit Task 1**

```bash
git add src/App.tsx src/components/WorkspaceSwitcher.tsx src/components/ReplyInput/index.tsx src/components/ReplyInput/index.test.tsx src/stores/task-store/index.ts tests/task-store.test.ts src-tauri/src/daemon/routing_user_input.rs src-tauri/src/daemon/state_task_flow.rs src-tauri/src/daemon/orchestrator/task_flow.rs src-tauri/src/daemon/routing_behavior_tests.rs src-tauri/src/daemon/routing_shared_role_tests.rs docs/superpowers/plans/2026-04-04-audit-remediation-wave-1.md
git commit -m "fix: enforce task-scoped workspace routing boundaries"
```

- [ ] **Step 7: Update `## CM Memory`**

Replace the Task 1 placeholders with the real commit hash, review verdict, verification commands, and the learned routing invariant before starting Task 2.

### Task 2: Harden Claude reconnect and runtime degradation visibility

**Files:**
- Modify: `src-tauri/src/daemon/control/claude_sdk_handler.rs`
- Modify: `src-tauri/src/daemon/control/claude_sdk_handler_tests.rs`
- Modify: `src-tauri/src/daemon/gui.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src/stores/bridge-store/types.ts`
- Modify: `src/stores/bridge-store/index.ts`
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/components/ShellContextBar.tsx`
- Modify: `src/components/ShellContextBar.test.tsx`

- [ ] **Step 1: Write failing reconnection and runtime-health regressions**

Cover both of these requirements:

- Claude SDK WS disconnects should retry with bounded backoff instead of stopping at `disconnected`
- a runtime failure severe enough to break message handling must surface in shell UI, not only in terminal logs

- [ ] **Step 2: Run the focused regressions to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::control::claude_sdk_handler_tests`
Expected: FAIL on reconnect or degradation-event coverage

Run: `bun test src/components/ShellContextBar.test.tsx`
Expected: FAIL because shell UI does not yet expose runtime degradation state

- [ ] **Step 3: Implement reconnect and health-surface behavior**

Implementation notes:

- add bounded automatic reconnect for Claude SDK WS using the existing session/epoch safety rules
- emit a dedicated GUI/runtime health signal when the daemon or provider connection becomes degraded
- expose that state in the bridge store and surface a compact shell-level warning affordance

- [ ] **Step 4: Re-run the focused verification**

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::control::claude_sdk_handler_tests`
Expected: PASS

Run: `bun test src/components/ShellContextBar.test.tsx`
Expected: PASS

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking review findings remain for Task 2.

- [ ] **Step 6: Commit Task 2**

```bash
git add src-tauri/src/daemon/control/claude_sdk_handler.rs src-tauri/src/daemon/control/claude_sdk_handler_tests.rs src-tauri/src/daemon/gui.rs src-tauri/src/daemon/mod.rs src/stores/bridge-store/types.ts src/stores/bridge-store/index.ts src/stores/bridge-store/listener-setup.ts src/components/ShellContextBar.tsx src/components/ShellContextBar.test.tsx docs/superpowers/plans/2026-04-04-audit-remediation-wave-1.md
git commit -m "fix: reconnect claude sdk and surface runtime degradation"
```

- [ ] **Step 7: Update `## CM Memory`**

Replace the Task 2 placeholders with the real commit hash, review verdict, verification commands, and the learned reconnect/degradation invariant before starting Task 3.

### Task 3: Persist volatile review and message-buffer state

**Files:**
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/daemon/state_delivery.rs`
- Modify: `src-tauri/src/daemon/orchestrator/review_gate.rs`
- Modify: `src-tauri/src/daemon/task_graph/persist.rs`
- Modify: `src-tauri/src/daemon/state_tests.rs`
- Modify: `src-tauri/src/daemon/orchestrator/tests.rs`
- Modify: `src-tauri/src/daemon/state_task_snapshot_tests.rs`

- [ ] **Step 1: Write failing persistence regressions**

Cover both of these requirements:

- pending lead approval state survives daemon restart
- buffered outgoing/in-flight messages survive daemon restart without cross-task leakage

- [ ] **Step 2: Run the focused regressions to verify they fail**

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::state_tests daemon::orchestrator::tests daemon::state_task_snapshot_tests`
Expected: FAIL on review-gate or buffered-message persistence coverage

- [ ] **Step 3: Persist and hydrate the volatile runtime state**

Implementation notes:

- extend daemon persistence so review-gate state and buffered delivery state are saved alongside task/session/artifact data
- ensure hydration preserves task boundaries and does not replay buffered messages into the wrong task/session
- keep persistence minimal and local; do not invent provider-owned storage for this wave

- [ ] **Step 4: Re-run the persistence verification**

Run: `cargo test --manifest-path src-tauri/Cargo.toml daemon::state_tests daemon::orchestrator::tests daemon::state_task_snapshot_tests`
Expected: PASS

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking review findings remain for Task 3.

- [ ] **Step 6: Commit Task 3**

```bash
git add src-tauri/src/daemon/state.rs src-tauri/src/daemon/state_delivery.rs src-tauri/src/daemon/orchestrator/review_gate.rs src-tauri/src/daemon/task_graph/persist.rs src-tauri/src/daemon/state_tests.rs src-tauri/src/daemon/orchestrator/tests.rs src-tauri/src/daemon/state_task_snapshot_tests.rs docs/superpowers/plans/2026-04-04-audit-remediation-wave-1.md
git commit -m "feat: persist review gate and buffered message state"
```

- [ ] **Step 7: Update `## CM Memory`**

Replace the Task 3 placeholders with the real commit hash, review verdict, verification commands, and the learned persistence invariant before starting Task 4.

### Task 4: Make approvals and artifacts actionable

**Files:**
- Modify: `src/stores/bridge-store/types.ts`
- Modify: `src/stores/bridge-store/index.ts`
- Modify: `src/stores/bridge-store/selectors.ts`
- Modify: `src/components/ShellContextBar.tsx`
- Modify: `src/components/ShellContextBar.test.tsx`
- Modify: `src/components/MessagePanel/PermissionQueue.tsx`
- Create: `src/components/MessagePanel/PermissionQueue.test.tsx`
- Modify: `src/components/TaskPanel/ArtifactTimeline.tsx`
- Create: `src/components/TaskPanel/ArtifactTimeline.test.tsx`
- Modify: `src/components/TaskPanel/index.tsx`
- Modify: `src-tauri/src/commands_task.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing approval/artifact regressions**

Cover all of these requirements:

- approvals show a pending count in the shell rail
- approval failures surface inline in the approvals UI
- artifact cards are interactive and open a detail view instead of behaving like dead text

- [ ] **Step 2: Run the focused regressions to verify they fail**

Run: `bun test src/components/ShellContextBar.test.tsx tests/task-panel-view-model.test.ts`
Expected: FAIL because approvals and artifacts are not yet actionable

Run: `bun test src/components/MessagePanel/PermissionQueue.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx`
Expected: FAIL because inline approval errors and artifact detail interactions do not exist yet

- [ ] **Step 3: Implement approval badges, inline errors, and artifact detail affordances**

Implementation notes:

- wire the existing permission-count selector into the shell rail
- store the latest permission-resolution failure in UI state so the approvals pane can render it inline
- add clickable artifact cards and a detail surface; if `contentRef` resolves to a readable local file, show a preview, otherwise show structured metadata plus the underlying reference

- [ ] **Step 4: Re-run the focused verification**

Run: `bun test src/components/ShellContextBar.test.tsx tests/task-panel-view-model.test.ts`
Expected: PASS

Run: `bun test src/components/MessagePanel/PermissionQueue.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx`
Expected: PASS

Run: `bun test`
Expected: PASS for the task's new frontend regressions

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking review findings remain for Task 4.

- [ ] **Step 6: Commit Task 4**

```bash
git add src/stores/bridge-store/types.ts src/stores/bridge-store/index.ts src/stores/bridge-store/selectors.ts src/components/ShellContextBar.tsx src/components/ShellContextBar.test.tsx src/components/MessagePanel/PermissionQueue.tsx src/components/TaskPanel/ArtifactTimeline.tsx src/components/TaskPanel/index.tsx src-tauri/src/commands_task.rs src-tauri/src/main.rs docs/superpowers/plans/2026-04-04-audit-remediation-wave-1.md
git commit -m "feat: make approvals and artifacts actionable"
```

- [ ] **Step 7: Update `## CM Memory`**

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

- [ ] **Step 1: Write failing message/stream regressions**

Cover all of these requirements:

- Codex reasoning preview can expand beyond the truncated teaser
- message list supports search/filter for long sessions
- image attachments can open in a larger lightbox view

- [ ] **Step 2: Run the focused regressions to verify they fail**

Run: `bun test src/components/MessagePanel/CodexStreamIndicator.test.ts`
Expected: FAIL on expand/collapse coverage

Run: `bun test src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/MessageBubble.test.tsx`
Expected: FAIL on search/lightbox coverage

- [ ] **Step 3: Implement expandable reasoning, search, and lightbox support**

Implementation notes:

- keep message virtualization intact while filtering/searching
- keep the default stream rail compact, but add an explicit expansion path for reasoning text
- keep image zoom state local to the message panel and avoid introducing provider-specific image logic

- [ ] **Step 4: Re-run the focused verification**

Run: `bun test src/components/MessagePanel/CodexStreamIndicator.test.ts`
Expected: PASS

Run: `bun test src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/MessageBubble.test.tsx`
Expected: PASS

Run: `bun test`
Expected: PASS for the task's frontend regressions

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking review findings remain for Task 5.

- [ ] **Step 6: Commit Task 5**

```bash
git add src/components/MessagePanel/CodexStreamIndicator.tsx src/components/MessagePanel/CodexStreamIndicator.test.ts src/components/MessagePanel/MessageList.tsx src/components/MessagePanel/MessageBubble.tsx src/components/MessagePanel/index.tsx docs/superpowers/plans/2026-04-04-audit-remediation-wave-1.md
git commit -m "feat: improve message search and stream ergonomics"
```

- [ ] **Step 7: Update `## CM Memory`**

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
