# Telegram Route Loop Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the occasional Telegram-triggered route replay loop that re-injects old inbound messages and floods the app with repeated `[Route] coder → lead delivered` logs.

**Architecture:** Treat this as a layered ingress correctness bug. First close the update replay window by persisting `last_update_id` after each handled update. Then block bot-self messages from re-entering the route pipeline. Finally add a lightweight in-memory idempotency guard for recently seen Telegram updates as a second safety net.

**Tech Stack:** Rust, Tauri daemon, Telegram Bot API polling, Cargo.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/telegram-route-loop-fix` on branch `fix/telegram-route-loop-fix`
- Root-cause evidence from read pass:
  - `src-tauri/src/telegram/runtime.rs` updates `cfg.last_update_id` in memory per update but only writes config once after the whole batch finishes.
  - If the runtime dies in that window, Telegram may replay the batch on next start.
  - `src-tauri/src/telegram/runtime_handlers.rs` routes every paired-chat text message to `user -> lead` with no `update_id`/`message_id` dedupe and no bot-self filter.
  - Replayed inbound messages re-trigger the full lead/coder workflow, surfacing as repeated `[Route] coder → lead delivered` logs.
- User symptom to preserve as acceptance target:
  - Telegram occasionally floods route logs after an inbound message
  - app-originated messages do not show the same behavior

## Project Memory

### Recent related commits

- `edd1770d` — initial Telegram backend introduced
- `2738bce8` — Telegram HTML/unicode fix
- `c3b18acc` — Telegram pairing/runtime stabilization
- `3cb840f1` — report_telegram removed; lead messages now route through Telegram hook

### Lessons that constrain this plan

- Fix ingress reliability first; do not start with UI/log suppression.
- Preserve current paired-chat behavior for real user messages.
- Telegram runtime bugs can be intermittent because they depend on restart timing; tests must target the replay window explicitly.

## File Map

- Modify: `src-tauri/src/telegram/runtime.rs`
- Modify: `src-tauri/src/telegram/runtime_handlers.rs`
- Modify: `src-tauri/src/telegram/types.rs`
- Modify: `src-tauri/src/telegram/config.rs` (only if bot id persistence needs serde/default handling)
- Modify: `src-tauri/src/daemon/telegram_lifecycle_tests.rs`
- Optionally add: focused telegram runtime unit tests inline under `runtime.rs` / `runtime_handlers.rs` if that is the smallest clean path

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: persist telegram update cursor after each message` | `cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::telegram_lifecycle_tests:: -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture` | Shrink the Telegram replay window by persisting `last_update_id` after each handled update instead of after the full batch. |
| Task 2 | `fix: ignore telegram bot self-messages` | same as above | Prevent bot-authored messages from being routed back in as user input. |
| Task 3 | `fix: dedupe recent telegram updates in runtime` | same as above | Add an in-memory guard against immediate replay of recently seen update ids. |

---

### Task 1: Persist Telegram update cursor after each handled update

**task_id:** `telegram-route-loop-fix-cursor`

**Acceptance criteria:**
- `last_update_id` is saved immediately after each successfully handled update.
- A mid-batch crash/restart no longer causes the already-handled earlier updates in that batch to be replayed.
- No behavior change for normal polling batches beyond more frequent config writes.

**allowed_files:**
- `src-tauri/src/telegram/runtime.rs`
- relevant test file(s) only within approved map

**max_files_changed:** `2`
**max_added_loc:** `90`
**max_deleted_loc:** `40`

**verification_commands:**
- `cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::telegram_lifecycle_tests:: -- --nocapture`

---

### Task 2: Ignore Telegram bot self-messages

**task_id:** `telegram-route-loop-fix-botfilter`

**Acceptance criteria:**
- Runtime stores or passes bot identity from `getMe`.
- Inbound handler drops messages where `message.from.id == bot_id`.
- Existing paired-chat user messages still route normally.

**allowed_files:**
- `src-tauri/src/telegram/runtime.rs`
- `src-tauri/src/telegram/runtime_handlers.rs`
- `src-tauri/src/telegram/types.rs`
- `src-tauri/src/telegram/config.rs` only if required
- relevant test file(s) only within approved map

**max_files_changed:** `5`
**max_added_loc:** `120`
**max_deleted_loc:** `40`

**verification_commands:**
- `cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::telegram_lifecycle_tests:: -- --nocapture`

---

### Task 3: Add a recent-update idempotency guard

**task_id:** `telegram-route-loop-fix-dedupe`

**Acceptance criteria:**
- Runtime suppresses immediate duplicates for recently seen Telegram `update_id`s.
- Duplicate deliveries from short replay windows do not re-enter routing.
- Guard is bounded (no unbounded memory growth).

**allowed_files:**
- `src-tauri/src/telegram/runtime.rs`
- `src-tauri/src/telegram/runtime_handlers.rs` if needed
- relevant test file(s) only within approved map

**max_files_changed:** `3`
**max_added_loc:** `100`
**max_deleted_loc:** `30`

**verification_commands:**
- `cargo test --manifest-path src-tauri/Cargo.toml telegram -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::telegram_lifecycle_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml routing_ -- --nocapture`
