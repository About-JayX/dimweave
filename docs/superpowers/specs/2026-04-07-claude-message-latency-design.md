# Claude Message Latency Design

**Date:** 2026-04-07
**Status:** Drafted from verified code + runtime-source inspection

## Goal

Make Claude feel live in the chat timeline when `stream_event` data is already arriving, instead of making the user wait for the terminal `result` / bridge reply before the visible Claude message catches up.

## Verified Findings

### 1. Claude Code is already sending partial stream data

- `src-tauri/src/daemon/claude_sdk/process.rs` launches Claude with:
  - `--output-format stream-json`
  - `--verbose`
  - `--include-partial-messages`
- `src-tauri/src/daemon/claude_sdk/process_tests.rs` already asserts that `--include-partial-messages` is present because it is required for `stream_event`.
- The locally installed Claude Code runtime is `/Users/jason/.nvm/versions/node/v24.14.0/lib/node_modules/@anthropic-ai/claude-code/cli.js` (`claude --version` reports **2.1.89**).
- That runtime’s hybrid SDK transport batches `stream_event` before POSTing them to the host. The minified source shows:
  - a `streamEventBuffer`
  - a `streamEventTimer`
  - `TmY = 100`
  - `write()` buffering `stream_event` and flushing them on a 100 ms timer

### 2. Dimweave receives those partial deltas and batches them again

- `src-tauri/src/daemon/claude_sdk/event_handler_stream.rs`
  - handles `content_block_delta -> text_delta`
  - appends text into `claude_preview_buffer`
  - batches Rust-side preview emission with `CLAUDE_PREVIEW_BATCH_WINDOW_MS = 50`
- `src/stores/bridge-store/listener-setup.ts`
  - batches frontend preview updates again with `setTimeout(..., 32)`

### 3. The visible Claude *message* is intentionally delayed

- `src-tauri/src/daemon/claude_sdk/event_handler_delivery.rs`
  - `build_direct_sdk_gui_message()` returns `None` for non-terminal Claude SDK text
  - the comment explicitly says partial assistant chunks are suppressed to avoid duplicate / preview noise
- `src-tauri/src/daemon/claude_sdk/event_handler_tests.rs`
  - `in_progress_sdk_text_does_not_create_visible_gui_message()` locks in that policy
- Result: Claude can be actively streaming preview text while the actual chat `messages[]` list still does not gain a new Claude message bubble until turn completion.

### 4. The existing live text is rendered as a transient rail bubble, not a real timeline message

- `src/stores/bridge-store/stream-reducers.ts` stores Claude preview in `claudeStream.previewText`
- `src/components/MessagePanel/ClaudeStreamIndicator.tsx` renders that preview as a transient “working draft”
- `src/components/MessagePanel/MessageList.tsx` places Claude’s live state in the tail footer rail instead of the actual `messages` timeline items

## Root Cause

This is **not** primarily a “Claude stopped sending information” bug.

The verified root cause is:

1. Claude Code already provides partial stream data.
2. Dimweave already receives and stores that partial stream data.
3. The chat timeline still looks stale because partial Claude content is intentionally excluded from the real message path and only shown in a transient rail/footer presentation.

The current UX therefore makes a live stream look like a late message.

## Non-Goals

- Do not rewrite the Claude SDK transport.
- Do not change bridge-vs-SDK terminal ownership rules for final visible results.
- Do not add a new daemon protocol unless the existing preview channel proves insufficient.
- Do not remove batching inside Claude Code itself; that lives in upstream Claude Code.

## Approaches Considered

### Approach A — Reduce batching only

**What changes**
- Lower or remove the 50 ms Rust preview batch.
- Lower or remove the 32 ms frontend batch.

**Pros**
- Very small code change.
- Slightly improves preview freshness.

**Cons**
- Does **not** solve the main perception problem: the real Claude message bubble still appears only at turn completion.
- Upstream Claude Code still imposes its own 100 ms stream-event batch.

**Verdict**
- Useful as a tuning knob, but insufficient as the main fix.

### Approach B — Promote existing preview text into a real transient Claude draft bubble in the chat timeline

**What changes**
- Keep the existing `claudeStream` preview pipeline.
- Render Claude’s live preview as a transient draft bubble inside the chat timeline itself.
- Remove Claude from the footer-only rail presentation (Codex can stay on the rail for now).
- Keep terminal/final result ownership unchanged: when `done/reset` happens, the transient draft disappears and the real terminal Claude message remains the only persisted message.

**Pros**
- Uses already-verified available data.
- Fixes the user-visible symptom without daemon/protocol churn.
- Keeps final bridge/SDK ownership semantics intact.
- Low-to-medium implementation risk.

**Cons**
- The transient draft is still derived UI state, not a persisted `BridgeMessage`.
- There may be a brief frame where the draft disappears just before the final terminal message lands, depending on event ordering.

**Verdict**
- **Recommended.**

### Approach C — Introduce provider-level in-progress Claude messages with stable draft IDs

**What changes**
- Emit in-progress Claude messages into the normal `agent_message` path.
- Add stable turn-scoped IDs so the frontend can replace/update one live bubble instead of appending duplicates.

**Pros**
- Cleanest long-term message model.
- Makes Claude behave more like a true streaming chat provider.

**Cons**
- Requires wider daemon/UI contract changes.
- More risk around dedupe, turn ownership, persisted history, and routing semantics.

**Verdict**
- Good future cleanup, too invasive for the current fix.

## Recommended Design

### User-facing behavior

- When Claude starts thinking, the chat timeline shows a Claude draft bubble immediately.
- If no text has arrived yet, that bubble shows a lightweight `thinking…` placeholder.
- As `claudeStream.previewText` grows, the draft bubble updates in place.
- When Claude finishes:
  - the draft bubble is cleared
  - the final routed Claude message remains as the persisted chat bubble

### Architecture

- Keep backend transport exactly as-is for this fix.
- Treat `claudeStream` as the source of truth for **transient** Claude draft content.
- Stop treating Claude’s live draft as a footer-only rail concern.
- Move the Claude live draft into the chat timeline rendering path.

### Rendering model

- Synthesize one transient Claude draft view model from:
  - `claudeStream.thinking`
  - `claudeStream.previewText`
  - `claudeRole`
- Render that draft with normal message-bubble styling so the live Claude response visually behaves like a real message.
- Do **not** persist the draft into `messages[]`.
- Keep Codex stream UI unchanged in this change set.

### Data / state rules

- Draft visible if `claudeStream.thinking` is true.
- Draft text:
  - `previewText` tail when non-empty
  - otherwise `"thinking…"`
- Draft cleared on `done` / `reset` exactly as current stream state is cleared.
- No new Rust state fields required.

### Tests required

- View-model test for building a transient Claude draft item from stream state.
- Message list test proving:
  - Claude live draft renders inline when only stream state exists
  - no extra footer Claude indicator is used
  - final persisted messages still render normally

## Files Expected To Change

- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/ClaudeStreamIndicator.tsx` or replacement component
- `src/components/MessagePanel/view-model.ts`
- `src/components/MessagePanel/view-model.test.ts`
- `src/components/MessagePanel/MessageList.test.tsx`
- `CLAUDE.md` (current limitation text is now stale relative to the code and should be corrected while touching this area)

## Acceptance Criteria

1. Verified Claude preview text appears as a live draft bubble in the chat timeline.
2. The user no longer has to wait for the terminal `done` message before seeing Claude’s visible message body evolve.
3. Final Claude terminal message ownership semantics remain unchanged.
4. No duplicate persisted Claude messages are introduced.
5. Existing stream batching remains bounded and stable.
