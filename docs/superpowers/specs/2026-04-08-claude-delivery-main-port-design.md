# Claude Delivery Main-Port Design

**Goal:** Port the verified Claude delivery-chain logic fixes from `plan/claude-delivery-chain` onto `main` without regressing the newer Claude stream UI work already merged on `main`.

## Context

- `main` is currently at `866ce9ec` (`merge: land claude stream ui updates`).
- The stream UI work already landed on `main` in:
  - `43009c80` `feat: stream Claude thinking/text/tool blocks to UI`
  - `ac717f1a` `fix: dedupe claude stream preview updates`
- The logic-fix branch is `plan/claude-delivery-chain`, whose relevant commits are:
  - `8320788e` `test: lock Claude delivery ownership regressions`
  - `cb7d298d` `fix: harden Claude delivery ownership and terminal ordering`
  - `a5ea5903` `test: lock Claude preview flush timing`
  - `100522a1` `fix: flush Claude preview before terminal events`
- In a fresh `main`-based worktree, baseline verification passed after the required Rust setup step:
  - `cargo build -p dimweave-bridge`
  - `cargo test --manifest-path src-tauri/Cargo.toml`
  - `bun test`

## What still needs porting onto `main`

### Backend logic

`main` still has the old delivery ownership behavior:

- `claim_claude_bridge_terminal_delivery()` does not latch `CompletedByBridge` in the `Inactive` branch.
- `claude_terminal_reply_claims_visible_result()` still claims ownership for any non-empty terminal bridge reply, even when `to != "user"`.
- Both the bridge terminal path and SDK `result` path still emit `ClaudeStreamPayload::Done` before the durable final message is routed.

### Frontend logic

`main` already has the new Claude stream block rendering, but the listener timing bug still exists:

- In `listener-setup.ts`, non-preview Claude stream events clear pending preview text before it is flushed into store state.
- This can still drop the last queued preview frame if `done`/`reset` lands inside the batching window.

### Documentation

`docs/agents/claude-message-delivery.md` still describes the pre-fix ownership rules and pre-fix event ordering.

## Approaches considered

### 1. Merge the whole delivery branch into `main`

**Rejected.**

That branch forked before the stream UI merge on `main`, so a wholesale merge would drag in stale UI-facing tests and branch-specific plan churn. The user explicitly asked to preserve `main`’s improved stream rendering and focus only on logic issues.

### 2. Cherry-pick only the production commits

**Rejected.**

Cherry-picking only `cb7d298d` and `100522a1` would miss the regression tests and main-specific test shape adjustments. We need the logic, the guards, and doc updates, but adapted to `main`’s current stream model.

### 3. Manually port only the applicable logic fixes onto a fresh `main` worktree

**Recommended.**

This keeps `main`’s UI rendering intact, ports only the verified ownership/flush logic, and lets us update tests to `main`’s current stream shape instead of replaying branch-specific UI assumptions.

## Design

### Backend

- Keep the existing Claude SDK + bridge dual-path architecture.
- Port the ownership rules from `plan/claude-delivery-chain`:
  - `Inactive` bridge terminal claims must latch `CompletedByBridge`
  - only terminal, non-empty, `to="user"` bridge replies may claim visible-result ownership
  - final durable messages must be routed before `ClaudeStreamPayload::Done`
- Keep `main`’s stream UI protocol unchanged; do not alter the `stream_event` shape or block rendering model.

### Frontend

- Keep `main`’s stream UI rendering as-is (`thinkingText`, block type, tool block state, etc.).
- Port only the batching/flush logic:
  - flush pending Claude preview text before clearing pending state on terminal/non-preview events
  - add a narrow helper only if it makes the ordering contract easier to test and reuse
- Do not change `ClaudeStreamIndicator.tsx`, block rendering, or layout behavior unless needed for tests to reflect the ported logic.

### Docs

- Update `docs/agents/claude-message-delivery.md` so it matches verified runtime behavior on `main` after the port:
  - bridge-first `Inactive` claims latch `CompletedByBridge`
  - only `to="user"` terminal replies claim visible-result ownership
  - preview is flushed before `done/reset`
  - durable final messages are routed before draft clear

## Non-goals

- No new Claude stream UI features
- No visual redesign of Message Panel components
- No rewrite of the stream-event reducer model
- No unrelated cleanup in `main`

## Validation plan

### Focused port checks

- `cargo test -q inactive_bridge_terminal_delivery_blocks_later_sdk_terminal_delivery --manifest-path src-tauri/Cargo.toml`
- `cargo test -q terminal_bridge_handoff_to_worker_does_not_claim_visible_result --manifest-path src-tauri/Cargo.toml`
- `cargo test -q sdk_terminal_delivery_claim_blocks_later_bridge_terminal_delivery --manifest-path src-tauri/Cargo.toml`
- `cargo test -q bridge_terminal_delivery_claim_blocks_later_sdk_terminal_delivery --manifest-path src-tauri/Cargo.toml`
- `bun test src/stores/bridge-store/listener-setup.test.ts src/components/MessagePanel/MessageList.test.tsx`

### Final verification

- `cargo build -p dimweave-bridge`
- `cargo test --manifest-path src-tauri/Cargo.toml`
- `bun test`
- `bun run build`
- `git status --short`

## Decision

Proceed with a manual logic-only port on a fresh `main` worktree, with separate backend and frontend verification tasks, and do not pull over any UI rendering changes that `main` already supersedes.
