# Claude Delivery Main-Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the verified Claude delivery ownership and preview-flush fixes onto `main` without overwriting `main`’s newer Claude stream UI rendering.

**Architecture:** Keep `main`’s current Claude stream UI model intact, but port the verified logic fixes from `plan/claude-delivery-chain`. Backend work tightens bridge/SDK ownership and event ordering. Frontend work only fixes pending-preview flush timing before terminal events clear draft state.

**Tech Stack:** Rust, Tauri, Axum WebSocket handlers, React, TypeScript, Zustand, Bun tests, Cargo tests

---

## Verified Main-Branch Context

- `main` already includes stream UI commits `43009c80` and `ac717f1a`.
- The relevant fixes to port come from `8320788e`, `cb7d298d`, `a5ea5903`, and `100522a1` on `plan/claude-delivery-chain`.
- Fresh `main` worktree baseline passed after Rust setup:
  - `cargo build -p dimweave-bridge`
  - `cargo test --manifest-path src-tauri/Cargo.toml`
  - `bun test`

## File Map

- Modify: `src-tauri/src/daemon/state_delivery.rs`
- Modify: `src-tauri/src/daemon/state_tests.rs`
- Modify: `src-tauri/src/daemon/control/handler.rs`
- Modify: `src-tauri/src/daemon/claude_sdk/event_handler.rs`
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/bridge-store/stream-batching.ts`
- Modify: `src/stores/bridge-store/listener-setup.test.ts`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`
- Modify: `docs/agents/claude-message-delivery.md`

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `70677d8a` | `manual diff review` | `cargo test -q inactive_bridge_terminal_delivery_blocks_later_sdk_terminal_delivery --manifest-path src-tauri/Cargo.toml` (FAIL as expected); `cargo test -q terminal_bridge_handoff_to_worker_does_not_claim_visible_result --manifest-path src-tauri/Cargo.toml` (FAIL as expected); `cargo test -q terminal_bridge_reply_to_user_claims_visible_result --manifest-path src-tauri/Cargo.toml` (PASS); `git diff --check` | Lock the missing backend ownership regressions on top of `main` before porting behavior. Keep Task 1 helper-level because `main` still uses the old `(status, content)` helper signature; Task 2 will refactor it to `BridgeMessage`. |
| Task 2 | `pending` | `pending` | `pending` | Port only backend logic and ordering; keep `main`’s stream UI protocol untouched. |
| Task 3 | `pending` | `pending` | `pending` | Reproduce the real frontend timing bug on `main`’s current stream state model before fixing it. |
| Task 4 | `pending` | `pending` | `pending` | Flush pending preview before terminal clear, then update docs to match verified behavior on `main`. |

## Baseline Verification

- Run: `cargo build -p dimweave-bridge`
- Run: `cargo test --manifest-path src-tauri/Cargo.toml`
- Run: `bun test`

Expected: PASS on the untouched `main`-based worktree before porting changes.

## Task 1: Lock backend ownership regressions on `main`

**Acceptance criteria:**
- A bridge-first terminal reply in the `Inactive` branch blocks the later SDK terminal claim.
- Internal bridge handoffs to `lead` or `coder` do not count as user-visible final-result ownership.
- Focused Rust tests fail before the implementation change.

**Files:**
- Modify: `src-tauri/src/daemon/state_tests.rs`
- Modify: `src-tauri/src/daemon/control/handler.rs`

**Planned CM:** `test: lock Claude delivery ownership regressions on main`

- [x] **Step 1: Add the missing state-machine regression test**

Append this test near the existing delivery-claim tests in `src-tauri/src/daemon/state_tests.rs`:

```rust
#[test]
fn inactive_bridge_terminal_delivery_blocks_later_sdk_terminal_delivery() {
    let mut s = DaemonState::new();

    assert!(s.claim_claude_bridge_terminal_delivery());
    assert!(!s.claim_claude_sdk_terminal_delivery());
}
```

- [x] **Step 2: Run the focused state test to verify RED**

Run:

```bash
cargo test -q inactive_bridge_terminal_delivery_blocks_later_sdk_terminal_delivery --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because `claim_claude_bridge_terminal_delivery()` still returns `true` in the `Inactive` branch without latching `CompletedByBridge`.

- [x] **Step 3: Add the control-handler ownership guard tests**

Because `main` still has the old helper signature, add these helper-level tests to `src-tauri/src/daemon/control/handler.rs`:

```rust
#[test]
fn terminal_bridge_reply_to_user_claims_visible_result() {
    assert!(claude_terminal_reply_claims_visible_result(
        MessageStatus::Done,
        "Final report",
    ));
}

#[test]
fn terminal_bridge_handoff_to_worker_does_not_claim_visible_result() {
    assert!(!claude_terminal_reply_claims_visible_result(
        MessageStatus::Done,
        "Take over the next patch set",
    ));
}
```

- [x] **Step 4: Run the focused control-handler test to verify RED**

Run:

```bash
cargo test -q terminal_bridge_handoff_to_worker_does_not_claim_visible_result --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL because the helper still returns `true` for any non-empty terminal content.

- [x] **Step 5: Commit the failing backend tests**

```bash
git add src-tauri/src/daemon/state_tests.rs src-tauri/src/daemon/control/handler.rs
git commit -m "test: lock Claude delivery ownership regressions on main"
```

- [x] **Step 6: Update `## CM Memory`**

## Task 2: Port backend ownership and terminal-ordering logic onto `main`

**Acceptance criteria:**
- `Inactive` bridge claims latch `CompletedByBridge`.
- Only terminal, non-empty, `to="user"` bridge replies claim user-visible final-result ownership.
- Durable final messages are routed before `ClaudeStreamPayload::Done`.

**Files:**
- Modify: `src-tauri/src/daemon/state_delivery.rs`
- Modify: `src-tauri/src/daemon/control/handler.rs`
- Modify: `src-tauri/src/daemon/claude_sdk/event_handler.rs`

**Planned CM:** `fix: port Claude delivery ownership logic to main`

- [ ] **Step 1: Latch bridge ownership in both `Active` and `Inactive`**

Update `claim_claude_bridge_terminal_delivery()` in `src-tauri/src/daemon/state_delivery.rs`:

```rust
pub fn claim_claude_bridge_terminal_delivery(&mut self) -> bool {
    match self.claude_sdk_direct_text_state {
        ClaudeSdkDirectTextState::Active | ClaudeSdkDirectTextState::Inactive => {
            self.claude_sdk_direct_text_state = ClaudeSdkDirectTextState::CompletedByBridge;
            true
        }
        ClaudeSdkDirectTextState::CompletedBySdk
        | ClaudeSdkDirectTextState::CompletedByBridge => false,
    }
}
```

- [ ] **Step 2: Narrow visible-result ownership to user-visible terminal replies**

Refactor the helper in `src-tauri/src/daemon/control/handler.rs` to accept the full message:

```rust
fn claude_terminal_reply_claims_visible_result(
    message: &crate::daemon::types::BridgeMessage,
) -> bool {
    message.to == "user"
        && message.status.is_some_and(|s| s.is_terminal())
        && !message.content.trim().is_empty()
}
```

Also update the tests to construct `BridgeMessage` values and assert against the refactored helper.

- [ ] **Step 3: Emit `Done` only after bridge-routed final delivery**

Restructure the Claude terminal branch in `handle_connection()` so the durable final message routes before `ClaudeStreamPayload::Done`:

```rust
let mut bridge_claimed_delivery = false;

if id == "claude" && claude_terminal_reply_claims_visible_result(&message) {
    let should_route = state.write().await.claim_claude_bridge_terminal_delivery();
    if should_route {
        bridge_claimed_delivery = true;
    } else {
        suppress_message = true;
        state.write().await.finish_claude_sdk_direct_text_turn();
        gui::emit_system_log(
            &app,
            "info",
            "[Control] suppressed duplicate Claude terminal reply after SDK fallback",
        );
        gui::emit_claude_stream(&app, ClaudeStreamPayload::Done);
    }
}

if suppress_message || message.content.trim().is_empty() {
    if !suppress_message
        && agent_id.as_deref() == Some("claude")
        && message.status.is_some_and(|s| s.is_terminal())
    {
        gui::emit_claude_stream(&app, ClaudeStreamPayload::Done);
    }
    continue;
}

let is_claude_terminal = agent_id.as_deref() == Some("claude")
    && message.status.is_some_and(|s| s.is_terminal());
routing::route_message(&state, &app, message).await;
if bridge_claimed_delivery || is_claude_terminal {
    gui::emit_claude_stream(&app, ClaudeStreamPayload::Done);
}
```

- [ ] **Step 4: Emit SDK `Done` only after the durable SDK final message is routed**

Restructure `handle_result()` in `src-tauri/src/daemon/claude_sdk/event_handler.rs`:

```rust
flush_pending_preview_batch(state, app).await;

let text = event["result"]
    .as_str()
    .map(ToOwned::to_owned)
    .or_else(|| Some(extract_assistant_text(event)));

if let Some(text) = text.filter(|text| !text.is_empty()) {
    if !claim_sdk_terminal_delivery(state).await {
        gui::emit_system_log(
            app,
            "info",
            "[Claude SDK] suppressed duplicate terminal text; bridge owns visible result",
        );
        finish_sdk_direct_text_turn(state).await;
        gui::emit_claude_stream(app, ClaudeStreamPayload::Done);
        gui::emit_system_log(app, "info", "[Claude SDK] turn completed");
        return;
    }

    if let Some(msg) = build_direct_sdk_gui_message(role, &text, MessageStatus::Done) {
        routing::route_message(state, app, msg).await;
    }
}

gui::emit_claude_stream(app, ClaudeStreamPayload::Done);
gui::emit_system_log(app, "info", "[Claude SDK] turn completed");
```

- [ ] **Step 5: Run the focused backend regression suite to verify GREEN**

Run:

```bash
cargo test -q inactive_bridge_terminal_delivery_blocks_later_sdk_terminal_delivery --manifest-path src-tauri/Cargo.toml
cargo test -q terminal_bridge_handoff_to_worker_does_not_claim_visible_result --manifest-path src-tauri/Cargo.toml
cargo test -q terminal_bridge_reply_to_user_claims_visible_result --manifest-path src-tauri/Cargo.toml
cargo test -q sdk_terminal_delivery_claim_blocks_later_bridge_terminal_delivery --manifest-path src-tauri/Cargo.toml
cargo test -q bridge_terminal_delivery_claim_blocks_later_sdk_terminal_delivery --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 6: Commit the backend port**

```bash
git add src-tauri/src/daemon/state_delivery.rs src-tauri/src/daemon/state_tests.rs src-tauri/src/daemon/control/handler.rs src-tauri/src/daemon/claude_sdk/event_handler.rs
git commit -m "fix: port Claude delivery ownership logic to main"
```

- [ ] **Step 7: Update `## CM Memory`**

## Task 3: Lock frontend preview-flush timing on `main`’s current stream model

**Acceptance criteria:**
- A queued Claude preview chunk is dropped by the current listener order before the fix.
- The draft-to-final handoff gap is expressed against `main`’s current MessageList test harness.
- Focused Bun tests fail before the implementation change.

**Files:**
- Modify: `src/stores/bridge-store/listener-setup.test.ts`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`

**Planned CM:** `test: lock Claude preview flush timing on main`

- [ ] **Step 1: Add the listener-order red test**

Add the real bad-ordering regression to `src/stores/bridge-store/listener-setup.test.ts`:

```ts
import {
  clearPendingClaudePreview,
  createPendingStreamUpdates,
  flushPendingStreamUpdates,
  queueClaudePreviewUpdate,
} from "./stream-batching";

test("flushes queued Claude preview before terminal done clears the draft", () => {
  const pending = createPendingStreamUpdates();
  queueClaudePreviewUpdate(pending, { kind: "preview", text: "final streamed sentence" });

  const state = baseState();
  clearPendingClaudePreview(pending);

  const partial = flushPendingStreamUpdates(state, pending);
  const finalState = {
    ...state,
    ...partial,
    claudeStream: partial.claudeStream ?? state.claudeStream,
  };

  expect(finalState.claudeStream.previewText).toBe("final streamed sentence");
});
```

- [ ] **Step 2: Run the focused listener test to verify RED**

Run:

```bash
bun test src/stores/bridge-store/listener-setup.test.ts
```

Expected: FAIL because the current listener clears pending Claude preview before it flushes.

- [ ] **Step 3: Add the draft-to-final red scenario in the main MessageList test**

Append this test to `src/components/MessagePanel/MessageList.test.tsx`:

```tsx
test("renders the final Claude bubble after the draft row clears", async () => {
  installTauriStub();
  const [{ MessageList }, { useBridgeStore }] = await Promise.all([
    import("./MessageList"),
    import("@/stores/bridge-store"),
  ]);

  useBridgeStore.setState((state) => ({
    ...state,
    claudeStream: {
      thinking: false,
      previewText: "",
      thinkingText: "",
      blockType: "idle" as const,
      toolName: "",
      lastUpdatedAt: 2,
    },
  }));

  const html = renderToStaticMarkup(<MessageList messages={[]} />);

  expect(html).toContain("Final report delivered to the user.");
  expect(html).not.toContain("writing");
});
```

- [ ] **Step 4: Run the focused MessageList test to verify RED**

Run:

```bash
bun test src/components/MessagePanel/MessageList.test.tsx
```

Expected: FAIL because the empty-state gap is still reachable before the ported ordering fix.

- [ ] **Step 5: Commit the red frontend tests**

```bash
git add src/stores/bridge-store/listener-setup.test.ts src/components/MessagePanel/MessageList.test.tsx
git commit -m "test: lock Claude preview flush timing on main"
```

- [ ] **Step 6: Update `## CM Memory`**

## Task 4: Port preview flush ordering and update the delivery doc on `main`

**Acceptance criteria:**
- Non-preview Claude stream events flush pending preview state before clearing the draft.
- The updated tests now assert the fixed ordering on `main`’s current stream model.
- The delivery-chain document matches verified runtime behavior on `main`.

**Files:**
- Modify: `src/stores/bridge-store/listener-setup.ts`
- Modify: `src/stores/bridge-store/stream-batching.ts`
- Modify: `src/stores/bridge-store/listener-setup.test.ts`
- Modify: `src/components/MessagePanel/MessageList.test.tsx`
- Modify: `docs/agents/claude-message-delivery.md`

**Planned CM:** `fix: port Claude preview flush logic to main`

- [ ] **Step 1: Flush pending Claude preview before clearing pending state**

Update the Claude listener path in `src/stores/bridge-store/listener-setup.ts`:

```ts
listen<ClaudeStreamPayload>("claude_stream", (e) => {
  if (queueClaudePreviewUpdate(pendingStreamUpdates, e.payload)) {
    schedulePendingFlush();
    return;
  }

  flushPendingStreams();
  clearPendingClaudePreview(pendingStreamUpdates);
  if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
    cancelPendingFlush();
  }

  set((s) => handleClaudeStreamEvent(s, e.payload));
});
```

- [ ] **Step 2: Add the narrow preview-flush helper if it makes the test contract clearer**

If helpful, add this to `src/stores/bridge-store/stream-batching.ts`:

```ts
export function flushClaudePreviewIfPending(
  state: BridgeState,
  pending: PendingStreamUpdates,
): Partial<BridgeState> {
  if (!pending.claudePreviewText) {
    return {};
  }
  return flushPendingStreamUpdates(state, pending);
}
```

- [ ] **Step 3: Rewrite the frontend timing tests to the fixed ordering**

Update `src/stores/bridge-store/listener-setup.test.ts` to assert:

```ts
const flushed = flushClaudePreviewIfPending(state, pending);
const stateAfterFlush = {
  ...state,
  ...flushed,
  claudeStream: flushed.claudeStream ?? state.claudeStream,
};

expect(stateAfterFlush.claudeStream.previewText).toBe("final streamed sentence");
expect(pending.claudePreviewText).toBe("");

const donePartial = handleClaudeStreamEvent(stateAfterFlush, { kind: "done" });
expect(donePartial.claudeStream?.previewText).toBe("");
```

And update `src/components/MessagePanel/MessageList.test.tsx` to the fixed post-state:

```tsx
const finalMessage = {
  id: "msg_final",
  from: "claude",
  to: "user",
  content: "Final report delivered to the user.",
  timestamp: 2,
};

const html = renderToStaticMarkup(<MessageList messages={[finalMessage]} />);
expect(html).toContain("Final report delivered to the user.");
expect(html).not.toContain("writing");
```

- [ ] **Step 4: Update `docs/agents/claude-message-delivery.md`**

Revise the document so it states:

```md
- `CompletedByBridge` is latched for bridge-first terminal ownership, including the `Inactive` branch.
- Only terminal bridge replies that target `user` claim the user-visible final result.
- SDK `result` remains the fallback path when Claude does not emit a user-visible bridge reply.
- Claude preview text is flushed before `done/reset`, and the durable final message is routed before the draft is cleared.
```

Also rewrite the `reply(to="coder")` note so it matches the verified `to=="user"` ownership gate.

- [ ] **Step 5: Run focused verification**

Run:

```bash
bun test src/stores/bridge-store/listener-setup.test.ts src/components/MessagePanel/MessageList.test.tsx
cargo test -q inactive_bridge_terminal_delivery_blocks_later_sdk_terminal_delivery --manifest-path src-tauri/Cargo.toml
cargo test -q terminal_bridge_handoff_to_worker_does_not_claim_visible_result --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: PASS.

- [ ] **Step 6: Commit the frontend/doc port**

```bash
git add src/stores/bridge-store/listener-setup.ts src/stores/bridge-store/stream-batching.ts src/stores/bridge-store/listener-setup.test.ts src/components/MessagePanel/MessageList.test.tsx docs/agents/claude-message-delivery.md
git commit -m "fix: port Claude preview flush logic to main"
```

- [ ] **Step 7: Update `## CM Memory`**

## Final Verification

- Run: `cargo build -p dimweave-bridge`
- Run: `cargo test --manifest-path src-tauri/Cargo.toml`
- Run: `bun test`
- Run: `bun run build`
- Run: `git status --short`

Expected:

- Rust setup/build: succeeds
- Rust tests: all pass
- Bun tests: all pass
- Build: succeeds
- Git status: clean

## Execution Handoff

- Preferred execution mode: `superpowers:subagent-driven-development`
- Suggested implementation order: Task 1 -> Task 2 -> Task 3 -> Task 4
- If full-suite fallout appears that is not covered by these tasks, stop, document the exact failing command/output, and extend the plan before implementing additional fixes.
