# Historical Provider Session Convergence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Claude and Codex reliably converge on the same historical task/session context from the provider panels, not just when starting a new session.

**Architecture:** Treat already-normalized history items as canonical task-session resumes instead of raw provider launches. Keep raw external-id launch only for provider history entries that are not yet normalized. Separately, stop task-context refreshes from mutating the frontend active task unless the backend active task truly changed, so reconnect/disconnect churn cannot bounce the UI back to an old task.

**Tech Stack:** Rust, TypeScript, React, Zustand, Tauri, cargo test, bun test

---

## Root Cause Summary

1. **Provider panels bypass canonical normalized resume.**
   - `src/components/ClaudePanel/index.tsx` and `src/components/AgentStatus/CodexPanel.tsx` always launch using raw `externalId`.
   - They ignore `normalizedSessionId`, even though the daemon already has a dedicated `ResumeSession` path that restores the correct task and stored role.
   - Result: historical resume depends on the current UI-selected role and current task instead of the canonical normalized session/task.

2. **Task-context refresh emits fake task selection changes.**
   - `src-tauri/src/daemon/gui_task.rs` always emits `ActiveTaskChanged(task_id)` inside `build_task_context_events(...)`.
   - Call sites use this helper even when merely refreshing a non-active task (for example, disconnecting the old Codex/Claude session during a move to a historical task).
   - Result: the frontend can be forced back onto the old task during reconnect/disconnect churn.

3. **If canonical resume is used, bridge role state must stay in sync.**
   - The daemon updates `claude_role` / `codex_role` during real resume, but the frontend bridge store only refreshes roles from the initial status snapshot or explicit role-change actions.
   - Result: after a historical resume, the UI role selector can drift from the daemon’s actual runtime role.

---

## File Map

### Modify

- `src/components/ClaudePanel/index.tsx`
- `src/components/AgentStatus/CodexPanel.tsx`
- `src/components/AgentStatus/provider-session-view-model.ts`
- `tests/provider-session-view-model.test.ts`
- `src-tauri/src/daemon/gui_task.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/control/handler.rs`
- `src-tauri/src/daemon/gui.rs`
- `src/stores/bridge-store/listener-payloads.ts`
- `src/stores/bridge-store/listener-setup.ts`
- `src/stores/bridge-store/listener-setup.test.ts`

---

### Task 1: Route normalized provider history through canonical session resume

**Files:**
- Modify: `src/components/AgentStatus/provider-session-view-model.ts`
- Modify: `tests/provider-session-view-model.test.ts`
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/AgentStatus/CodexPanel.tsx`

- [ ] **Step 1: Add failing decision-logic tests**

Add pure tests for a helper that resolves provider-panel history actions:

```ts
expect(
  resolveProviderHistoryAction({
    externalId: "claude_hist_1",
    normalizedSessionId: "sess_123",
  }),
).toEqual({ kind: "resumeNormalized", sessionId: "sess_123" });

expect(
  resolveProviderHistoryAction({
    externalId: "thread_hist_1",
    normalizedSessionId: null,
  }),
).toEqual({ kind: "resumeExternal", externalId: "thread_hist_1" });

expect(resolveProviderHistoryAction(null)).toEqual({ kind: "new" });
```

- [ ] **Step 2: Implement the helper**

In `src/components/AgentStatus/provider-session-view-model.ts`, add a helper that returns one of:

```ts
{ kind: "new" }
{ kind: "resumeNormalized"; sessionId: string }
{ kind: "resumeExternal"; externalId: string }
```

Rule:
- `normalizedSessionId` present → `resumeNormalized`
- only `externalId` present → `resumeExternal`
- no selected history → `new`

- [ ] **Step 3: Update Claude/Codex panel launch behavior**

Use the helper in both panels:

- For `resumeNormalized`, call `taskStore.resumeSession(sessionId)`
- For `resumeExternal`, keep the current provider-native launch path
- For `new`, keep the current new-session launch path

Important:
- Do **not** require the current UI-selected role for `resumeNormalized`
- Keep model/effort inputs only for `new` / raw external resume launches

- [ ] **Step 4: Verify targeted frontend tests**

Run:

```bash
bun test tests/provider-session-view-model.test.ts
```

Expected: new helper tests pass and existing provider-session view-model tests still pass.

---

### Task 2: Stop non-active task refreshes from changing frontend selection

**Files:**
- Modify: `src-tauri/src/daemon/gui_task.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/control/handler.rs`

- [ ] **Step 1: Add failing Rust regression coverage**

Add a test proving that refreshing a non-active task does **not** emit `ActiveTaskChanged`:

```rust
let events = build_task_context_events(
    Some(&task_a),
    &task_a.task_id,
    &sessions,
    &artifacts,
    Some("task_b"),
);

assert!(!events.iter().any(|event| matches!(
    event,
    TaskUiEvent::ActiveTaskChanged { task_id } if task_id.as_deref() == Some("task_a")
)));
```

Also keep a positive test proving the active task still emits `ActiveTaskChanged`.

- [ ] **Step 2: Refactor task-context event building**

Refactor `build_task_context_events(...)` so active-task selection is emitted only when the task being refreshed matches the backend active task (or when the caller explicitly requests it).

Implementation rule:
- Session/artifact/task refreshes for non-active tasks should emit:
  - `TaskUpdated`
  - `ReviewGateChanged`
  - `SessionTreeChanged`
  - `ArtifactsChanged`
- They should **not** emit `ActiveTaskChanged`

- [ ] **Step 3: Update call sites**

Update `emit_task_context_events(...)` in `src-tauri/src/daemon/mod.rs` and manual call sites in `src-tauri/src/daemon/control/handler.rs` to pass the backend active task context into the builder/refactored helper.

- [ ] **Step 4: Verify targeted Rust tests**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml gui_task -- --nocapture
```

Expected: the new non-active refresh regression test passes and existing gui_task tests remain green.

---

### Task 3: Keep bridge role state aligned with daemon runtime role after resume

**Files:**
- Modify: `src-tauri/src/daemon/gui.rs`
- Modify: `src/stores/bridge-store/listener-payloads.ts`
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/bridge-store/listener-setup.test.ts`

- [ ] **Step 1: Add failing reducer coverage**

Add a bridge-listener test proving that when an agent-status event arrives with a role, the local bridge store updates that role:

```ts
const next = reduceAgentStatus(state, {
  agent: "codex",
  online: true,
  role: "lead",
});

expect(next.codexRole).toBe("lead");
```

- [ ] **Step 2: Extend agent-status payload**

Include optional `role` in daemon `agent_status` events and wire it through the frontend payload typings.

Emit the role whenever Claude or Codex transitions online after launch/resume.

- [ ] **Step 3: Update bridge listener reducer**

Make the `agent_status` reducer update:
- `claudeRole` when `agent === "claude"` and `role` is present
- `codexRole` when `agent === "codex"` and `role` is present

- [ ] **Step 4: Verify targeted frontend tests**

Run:

```bash
bun test src/stores/bridge-store/listener-setup.test.ts
```

Expected: agent-status role-sync coverage passes and existing bridge listener tests stay green.

---

### Task 4: Regression verification for historical convergence

- [x] **Step 1: Run focused verification**

```bash
cargo test --manifest-path src-tauri/Cargo.toml gui_task -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml sync_claude_launch -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml sync_codex_launch -- --nocapture
bun test tests/provider-session-view-model.test.ts
bun test src/stores/bridge-store/listener-setup.test.ts
bun test tests/task-panel-view-model.test.ts
```

Expected: all targeted regressions pass.

- [x] **Step 2: Run broader verification**

```bash
cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture
bun test
```

Expected: full Rust and frontend suites pass.

- [ ] **Step 3: Manual behavior checklist**

Verify:
- Claude provider panel selecting a normalized historical entry resumes the canonical task/session, not the currently selected task.
- Codex provider panel selecting the matching normalized historical entry converges into the same task/session context.
- Moving one provider from an old task to a historical task does not bounce the UI back to the old task during disconnect/reconnect.
- After historical resume, the visible provider role matches the daemon’s actual runtime role.

---

## Implementation Record

**Status:** Automated implementation committed. Manual runtime checklist still pending.

## Commit Record

| Commit | Scope | Verification |
| --- | --- | --- |
| `97d0e2fb` | Provider panels now canonicalize normalized history items through `resumeSession`, non-active task refreshes no longer emit stale `ActiveTaskChanged`, bridge agent-status events now sync runtime roles, and the earlier launch-sync / Session Tree fixes remain covered by regression tests. | `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture`; `bun test` |

### Changes made in `97d0e2fb`

**Task 1 — Canonical resume routing:**
- `src/components/AgentStatus/provider-session-view-model.ts` — added `ProviderHistoryAction` type + `resolveProviderHistoryAction()` helper
- `src/components/ClaudePanel/index.tsx` — `doLaunch` now calls `resumeSession(sessionId)` for normalized entries
- `src/components/AgentStatus/CodexPanel.tsx` — `handleConnect` now calls `resumeSession(sessionId)` for normalized entries
- `tests/provider-session-view-model.test.ts` — 3 new `resolveProviderHistoryAction` tests (6 total, all pass)

**Task 2 — Non-active task refresh no longer emits `ActiveTaskChanged`:**
- `src-tauri/src/daemon/gui_task.rs` — `build_task_context_events` gains `active_task_id: Option<&str>` param; `ActiveTaskChanged` only emitted when task matches; `emit_task_context_events` reads active_task_id from state; 3 new regression tests (9 total, all pass)
- `src-tauri/src/daemon/control/handler.rs` — direct `build_task_context_events` call passes `active_task_id.as_deref()`

**Task 3 — Bridge role state syncs from daemon on resume:**
- `src-tauri/src/daemon/gui.rs` — `AgentStatusEvent` gains optional `role` field; new `emit_agent_status_online()` emits role
- `src-tauri/src/daemon/claude_sdk/runtime.rs` — online transition emits role via `emit_agent_status_online`
- `src-tauri/src/daemon/codex/mod.rs` — online transition emits role via `emit_agent_status_online`
- `src/stores/bridge-store/listener-payloads.ts` — `AgentStatusPayload` gains optional `role?: string`
- `src/stores/bridge-store/listener-setup.ts` — extracted `reduceAgentStatus()` helper; syncs `claudeRole`/`codexRole` on online status
- `src/stores/bridge-store/listener-setup.test.ts` — 4 new role-sync tests (7 total, all pass)

**Verification results:**
- `cargo test`: 308 passed, 0 failed
- `bun test`: 170 passed, 0 failed
- Pre-existing TS errors (3) confirmed unrelated to this change
