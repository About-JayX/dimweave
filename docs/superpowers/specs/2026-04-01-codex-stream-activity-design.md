# Codex Stream Activity Design

## Summary

Dimweave will expose Codex work-in-progress state in the message panel so users can see what Codex is doing instead of a generic `thinking...` label. The stream indicator will show three kinds of live context: the current activity label, the current reasoning summary, and command stdout/stderr.

## Product Goal

- Replace opaque Codex "thinking" status with visible progress.
- Keep the UX lightweight: one transient indicator, not a full timeline UI.
- Match Codex's native feel closely enough that the user can infer whether the model is reasoning, running commands, editing files, or using tools.

## Scope

### In scope

- Rust daemon handling for:
  - `item/started`
  - `item/reasoning/summaryTextDelta`
  - `item/commandExecution/outputDelta`
- Frontend store support for:
  - current activity label
  - accumulated reasoning summary
  - accumulated command output
- `CodexStreamIndicator` rendering updates:
  - activity label text
  - reasoning summary block
  - command output block
  - pulse animation disabled once any meaningful content is visible

### Out of scope

- Full per-item timeline or inspector UI
- Persisting reasoning/output into chat history
- Rendering raw `item/reasoning/textDelta`
- Rendering `item/fileChange/outputDelta`
- Introducing a new frontend test framework

## Architecture Decision

Use the existing `codex_stream` transient event channel and extend it with richer payload kinds. The daemon remains responsible for translating app-server JSON-RPC into compact GUI events, while the frontend store remains responsible for accumulating and resetting transient stream state for the current turn.

This preserves the current architecture:

- Codex WS client handles JSON-RPC transport
- daemon session event layer maps WS events to GUI stream events
- Zustand store accumulates current-turn display state
- `CodexStreamIndicator` renders that transient state

## Backend Design

### Event mapping

`src-tauri/src/daemon/codex/session_event.rs` will translate:

- `item/started`
  - `commandExecution` -> `Activity { label: "Running: <command>" }`
  - `fileChange` -> `Activity { label: "File <kind>: <path>" }`
  - `mcpToolCall` -> `Activity { label: "MCP tool: <tool>" }`
  - `webSearch` -> `Activity { label: "Searching: <query>" }`
  - `reasoning` -> `Activity { label: "Reasoning…" }`
- `item/reasoning/summaryTextDelta`
  - append delta to the current reasoning buffer
  - emit full accumulated `Reasoning { text }`
- `item/commandExecution/outputDelta`
  - emit `CommandOutput { text: delta }`

### Stream state

`StreamPreviewState` remains the per-turn transient state holder. It will now own:

- agent message preview buffer
- reasoning summary buffer

Both buffers reset on `turn/started` and `turn/completed`.

### Label extraction

Activity label derivation should be a pure helper so the mapping can be unit-tested without a Tauri `AppHandle`.

## Frontend Design

### Store state

`codexStream` will hold:

- `thinking`
- `currentDelta`
- `lastMessage`
- `turnStatus`
- `activity`
- `reasoning`
- `commandOutput`

`handleCodexStreamEvent` will:

- clear transient fields on `thinking`
- update `activity` on `activity`
- replace reasoning text on `reasoning`
- append command output on `commandOutput`
- reset all transient fields on `turnDone`

### Indicator rendering

`CodexStreamIndicator` keeps the existing single-bubble layout, but the content priority becomes:

1. agent text delta
2. reasoning summary
3. command output

The header label becomes:

- `streaming…` when agent text is streaming
- current activity label when available
- `thinking…` otherwise

Pulse animation stops as soon as any meaningful content is present, including the activity label itself.

## Testing Strategy

### Rust

- unit tests for activity label extraction
- unit tests for reasoning buffer accumulation/reset behavior

### Frontend

- reducer-style tests for `handleCodexStreamEvent`
- component rendering test for `CodexStreamIndicator`:
  - activity label is shown
  - pulse is removed when only activity is present

### Verification

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::codex`
- `bun test` for the new frontend tests
- `npm run build`

## Acceptance Criteria

- When Codex starts a command, the indicator shows `Running: ...`
- When Codex emits reasoning summary deltas, the indicator shows accumulated reasoning text
- When Codex emits command output deltas, the indicator shows accumulated stdout/stderr in a monospace block
- When only an activity label is present, the indicator does not pulse
- All transient stream fields reset cleanly at turn boundaries
