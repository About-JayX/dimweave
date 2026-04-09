# Review Addendum — fix/task-graph

## Findings from Lead Review

### Finding 1 — Telegram invalid-token save is destructive [FIXED]

**Root cause:** `save_and_restart()` tore down the runtime before validating the new token.

**Fix:** Extracted `validate_bot_token()` as a pure function. Called before `handle.take()` in `save_and_restart()`. Invalid tokens now return `Err` immediately without touching the running runtime.

**Tests:** 6 unit tests in `telegram_lifecycle_tests.rs` covering valid/invalid token formats.

### Finding 2 — Task save status can report success when nothing was persisted [FIXED]

**Root cause:** Three issues compounded:
1. `create_and_select_task()` called `auto_save_task_graph()` (fire-and-forget), then `mod.rs` emitted unconditional success — double-save with unreliable status.
2. `select_task()`, `resume_session()`, and `attach_provider_history()` all emitted fake success events despite not persisting.
3. `SaveStatus` lacked `taskId`, making the indicator global.

**Fix (iterative, 3 rounds):**
1. Removed unconditional `emit_task_save_status` from SelectTask, ResumeSession, AttachProviderHistory handlers.
2. Added `taskId: String` to `TaskSaveStatusEvent` and `taskId: string` to frontend `SaveStatus`. Scoped `SaveIndicator` to only render when `lastSave.taskId === activeTaskId`.
3. Removed `auto_save_task_graph()` from `create_and_select_task()`. CreateTask handler now performs the single authoritative `save_task_graph()` and emits success/failure based on the real `Result`.

**Final design:**
- CreateTask: explicit `save_task_graph()` → conditional emit with real result + task_id
- SelectTask/ResumeSession/AttachProviderHistory: no save-status emit
- Frontend `SaveIndicator`: scoped by `taskId` match

**Tests:** 4 persistence contract tests in `state_persistence_tests.rs`:
- `create_task_does_not_auto_persist`
- `save_task_graph_returns_ok_on_success`
- `save_task_graph_returns_err_on_unwritable_path`
- `select_task_does_not_persist`

### Finding 3 — Artifact detail wrapping class typo [FIXED]

**Fix:** Replaced invalid `wrap-break-word` with `break-words` in `ArtifactTimeline.tsx`.

## Final Verification

```
cargo check                    → clean (1 pre-existing warning in api.rs)
cargo test telegram            → 19 passed, 0 failed
cargo test state_persistence   → 4 passed, 0 failed
bun run build                  → tsc + vite success (2117 modules)
git diff --check               → no whitespace errors
```
