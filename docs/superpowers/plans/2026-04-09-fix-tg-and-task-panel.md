# Fix Task Graph Save Status + Telegram Pairing + UI Polish

**Goal:** Fix task graph save with no status feedback, Telegram pairing race condition, and ugly TaskPanel styling.

**Root Causes:**
1. `auto_save_task_graph()` silently swallows errors, no event to frontend
2. Telegram runtime loads config once at startup; `generate_pair()` writes to disk but runtime never sees update → `/pair` always fails
3. TaskPanel styling uses extreme low opacity (10-35%), tiny fonts (10px), no visual hierarchy
4. Telegram panel has no auto-start, no skeleton loading, buttons clutter the header

---

## File Map

### Task Graph Save Status (final design)
- `src-tauri/src/daemon/gui.rs` — `TaskSaveStatusEvent` includes `task_id: String`; `emit_task_save_status()` takes `task_id: &str`
- `src-tauri/src/daemon/gui_tests.rs` — extracted tests (200-line split)
- `src-tauri/src/daemon/state_snapshot.rs` — `create_and_select_task()` does NOT auto-persist; callers must call `save_task_graph()` explicitly
- `src-tauri/src/daemon/state_persistence.rs` — `auto_save_task_graph()` is fire-and-forget for non-create paths; `save_task_graph()` returns `Result` for authoritative callers
- `src-tauri/src/daemon/state_persistence_tests.rs` — 4 tests: no-auto-persist, save-ok, save-err-unwritable, select-no-persist
- `src-tauri/src/daemon/mod.rs` — CreateTask: single authoritative `save_task_graph()` + conditional `emit_task_save_status(success/failure, task_id)`; SelectTask/ResumeSession/AttachProviderHistory: NO save-status emit
- `src/stores/task-store/types.ts` — `SaveStatus` interface with `taskId: string`
- `src/stores/task-store/events.ts` — `task_save_status` listener
- `src/stores/task-store/index.ts` — `lastSave: null` initial state
- `src/components/TaskPanel/TaskHeader.tsx` — `SaveIndicator` scoped by `lastSave.taskId === activeTaskId`

### Telegram Pairing Fix
- `src-tauri/src/telegram/runtime.rs` — `config_tx` channel on `TelegramHandle`; drain config updates in poll loop
- `src-tauri/src/telegram/runtime_handlers.rs` — extracted `handle_update` + `handle_pair` (200-line split)
- `src-tauri/src/telegram/types.rs` — `bot_username` field on `TelegramConfig`, `from_config` reads it
- `src-tauri/src/telegram/config.rs` — empty file fallback, `bot_username` in test struct
- `src-tauri/src/daemon/telegram_lifecycle.rs` — `auto_start()`, push config via `config_tx` in `generate_pair`/`clear_pair`, token format validation
- `src-tauri/src/daemon/mod.rs` — auto-start telegram on daemon boot
- `src/stores/telegram-store.ts` — `clearPairing` auto-generates new pair code

### TaskPanel Styling
- `src/components/TaskPanel/TaskHeader.tsx` — status badges 50% opacity + 15% bg, text-[11px]
- `src/components/TaskPanel/SessionTree.tsx` — card borders 50%, status colors stronger, resume button states
- `src/components/TaskPanel/ArtifactTimeline.tsx` — per-kind colors, detail section improved
- `src/components/TaskPanel/index.tsx` — container borders/bg bumped

### Telegram Panel UI
- `src/components/AgentStatus/TelegramPanel.tsx` — skeleton loading, ActionMenu dropdown, PairCodeRow with auto-generate + refresh icon, bot username display
- `src/components/AgentStatus/ActionMenu.tsx` — extracted reusable dropdown (200-line split)

### Dev Scripts
- `package.json` — `dev:alt` (port 2420/5502/5500), `dev:tg` (port 3420/6502/6500)
- `vite.config.ts` — `DIMWEAVE_VITE_PORT` env var support

---

### Telegram Validation
- `src-tauri/src/daemon/telegram_lifecycle.rs` — `validate_bot_token()` extracted as pure function; called before runtime teardown in `save_and_restart()`
- `src-tauri/src/daemon/telegram_lifecycle_tests.rs` — 6 tests covering valid/invalid token formats

### ArtifactTimeline Fix
- `src/components/TaskPanel/ArtifactTimeline.tsx` — replaced invalid `wrap-break-word` with `break-words`

---

## Verification

```
cargo check --manifest-path src-tauri/Cargo.toml  → clean (1 pre-existing warning in api.rs)
cargo test telegram                                → 19 passed, 0 failed
cargo test state_persistence_tests                 → 4 passed, 0 failed
bun run build                                      → tsc + vite success (2117 modules)
git diff --check                                   → no whitespace errors
```

## CM Memory

| Task | Commit | Verification | Notes |
|------|--------|--------------|-------|
| All | pending | See verification section above | Single branch `fix/task-graph` in worktree |
