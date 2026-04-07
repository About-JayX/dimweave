# Reply Target, Terminal Scroll, and Shutdown Polish Design

## Summary

Dimweave has three small but user-visible interaction gaps:

1. The reply target picker in `ReplyInput` resets to `auto` whenever the shell view is remounted, even when the user expects that choice to remain stable for the current task.
2. The logs/terminal surface can render from the top when the user switches into it, instead of starting at the latest output.
3. App shutdown already asks the daemon to stop, but the shutdown path is not explicit enough about tearing down every live runtime/connection boundary before the process exits.

This design fixes all three in one pass without expanding the task graph schema.

## Product Goal

- Preserve the selected `To xxx` target per active task.
- Make the logs/terminal surface open at the bottom by default.
- Ensure application exit explicitly drains/tears down all tracked agent runtimes and live connections before quitting.

## Scope

### Included

- A task-scoped reply-target preference on the frontend.
- Log-panel auto-scroll behavior when entering the logs surface.
- Daemon/app shutdown hardening to stop Claude/Codex runtimes and disconnect bridge/runtime channels deterministically.
- Tests covering the three behaviors.

### Excluded

- Persisting reply-target preference across unrelated tasks.
- Reworking the entire bridge-store/task-store architecture.
- Replacing Virtuoso or redesigning the logs surface.
- Changing provider history semantics.

## Design

### 1. Reply target memory should be task-scoped

The current reply target is local component state in `ReplyInput`, so any remount resets it to `"auto"`.

The least risky design is:

- store a map of `{ [taskId]: Target }` in the frontend task store
- expose a selector/helper that resolves:
  - active task target if present
  - `"auto"` when no active task exists
- update the task-specific target when the picker changes
- avoid backend persistence for this preference

Why this design:

- the preference is UI-only, not daemon truth
- it survives tab/surface remounts because the store outlives the component
- it naturally scopes the setting to the current task
- it avoids polluting task graph persistence with per-user UI state

### 2. Logs surface should start at the bottom

The chat timeline already uses bottom-oriented scroll handling. The logs surface does not.

The fix is to give the logs Virtuoso the same kind of explicit bottom behavior:

- keep a ref to the logs Virtuoso/scroller
- when `surfaceMode` changes to `"logs"`:
  - if logs exist, scroll to the last item with `behavior: "auto"`
- if the user is already at the bottom, continue following output smoothly
- if the user has scrolled upward manually, do not force-scroll on every new line

This keeps the default view correct while preserving manual inspection.

### 3. Shutdown should be a real teardown barrier

Current shutdown already routes through `request_app_shutdown()` and `DaemonCmd::Shutdown`, but the exit contract should be made more explicit and more complete.

The shutdown path should:

- stop Codex app-server and clean its session home
- stop Claude SDK runtime
- clear/detach live bridge/runtime senders so no stale WS/runtime state survives inside the daemon
- stop any terminal/session managers owned by the app
- only then allow `app.exit(0)`

The daemon should remain the single shutdown coordinator; frontend should not try to shut down individual runtimes during app exit.

## File Map

### Reply target memory

- `src/components/ReplyInput/index.tsx`
- `src/components/ReplyInput/Footer.tsx`
- `src/components/ReplyInput/TargetPicker.tsx`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/selectors.ts`
- `tests/task-store.test.ts`

### Logs/terminal scroll

- `src/components/MessagePanel/index.tsx`
- `src/components/MessagePanel/MessageList.tsx`

### Shutdown hardening

- `src-tauri/src/main.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/session_manager.rs`
- `src-tauri/src/commands.rs`

## Testing Strategy

- Task-store tests for per-task reply target memory.
- ReplyInput tests verifying task-scoped target restoration.
- MessagePanel/MessageList tests for logs-surface scroll behavior.
- Rust daemon tests for shutdown teardown semantics.

## Acceptance Criteria

- When the user changes `To xxx`, leaves the chat surface, and returns within the same task, the selected target is preserved.
- Different tasks can hold different reply targets.
- The logs surface opens at the bottom when selected.
- New logs continue to follow output only while the viewer is at the bottom.
- App shutdown stops tracked Claude/Codex runtimes and clears live connection state before exit.
