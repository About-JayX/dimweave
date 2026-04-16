# Codex Output Schema Strict API Compatibility Fix

> Hotfix plan — single-session repair, no worktree needed.

**Goal:** Restore Codex structured output communication which broke when commit `0499a7c8` migrated `send_to` string enum to nested `target` object without satisfying OpenAI strict JSON schema requirements.

**Root Cause:** The Codex app-server relays `outputSchema` to the upstream API (OpenAI) which enforces strict JSON schema rules. The nested `target` object introduced in `0499a7c8` violated three API requirements:
1. All `type: "object"` nodes must have `additionalProperties: false`
2. All keys in `properties` must appear in `required`
3. Root-level `required` must include all property keys

The API returned `invalid_json_schema` errors on every `turn/start`, but the daemon's `handle_codex_event` had no handler for the WS `error` method — errors were silently dropped by the `_ => {}` catch-all. From the user's perspective, Codex showed a thinking bubble that disappeared with no output.

**Architecture:** Fix the schema, add error visibility, update prompt/examples to match the strict schema contract, fix a pre-existing React infinite-loop bug exposed during testing.

**Tech Stack:** Rust daemon (role_config, session_event), React 19 (MessageList), Vite build config

---

## Memory

- Recent related commits:
  - `0499a7c8` — the commit that introduced the structured `target` object and broke the schema (Task 6 of `2026-04-16-agent-directed-routing-redesign.md`)
  - `3283dd1d` — migrated Codex output parsing to structured MessageTarget model
  - `70dabf89` — hard-cut BridgeMessage to structured source/target
  - `5817368d` — added silent turn fallback diagnostics (masked the real error)
- Relevant prior plans:
  - `docs/superpowers/plans/2026-04-16-agent-directed-routing-redesign.md` — Task 6 introduced the broken schema
  - `docs/superpowers/plans/2026-04-16-task-provider-binding-and-codex-final-message-fix.md` — silent turn diagnostics
- Lessons carried forward:
  - OpenAI strict JSON schema is far more restrictive than standard JSON Schema: ALL object properties must be required, ALL objects need `additionalProperties: false`
  - The old `send_to` string enum worked because flat types have no nested-object constraints
  - Silent error swallowing (`_ => {}` in event dispatch) must never be the default for protocol methods — unknown methods should at minimum log

## Baseline

- Branch: `main` at `5a0ca6f9`
- Before fix: every Codex `turn/start` with `outputSchema` returned API error `invalid_json_schema`, silently dropped
- Verified via diagnostic logging: `[Codex][ws] method=error` followed by `turn/completed` with no `item/agentMessage`

## File Map

| File | Change |
|------|--------|
| `src-tauri/src/daemon/role_config/roles.rs` | Schema fix + prompt update + role-specific examples |
| `src-tauri/src/daemon/role_config/roles_tests.rs` | Test updates for new required fields |
| `src-tauri/src/daemon/codex/session_event.rs` | Add `error` WS method handler |
| `src-tauri/src/daemon/codex/structured_output_tests.rs` | Add empty-field compatibility tests |
| `src/components/MessagePanel/MessageList.tsx` | Fix scrollerRef infinite setState loop |
| `vite.config.ts` | Disable minification for debuggable DMG builds |

---

## Tasks

### Task 1: Fix output schema for strict API compatibility

- `task_id`: schema_fix
- `allowed_files`: `src-tauri/src/daemon/role_config/roles.rs`, `src-tauri/src/daemon/role_config/roles_tests.rs`
- `max_files_changed`: 2
- acceptance criteria:
  - `target` object has `additionalProperties: false`
  - `target.required` = `["kind", "role", "agentId"]`
  - Root `required` = `["message", "target", "status"]`
  - `target.description` no longer says "Omit to stay silent"
  - All role_config tests pass
- verification: `cargo test -p dimweave role_config`

### Task 2: Add Codex WS error method handler

- `task_id`: error_handler
- `allowed_files`: `src-tauri/src/daemon/codex/session_event.rs`
- `max_files_changed`: 1
- acceptance criteria:
  - `"error"` method in `handle_codex_event` match arm
  - Error detail logged via `eprintln!` and `gui::emit_system_log`
  - Error message displayed to user via `gui::emit_agent_message`
  - `stream_preview.mark_durable_output()` called to prevent silent-turn fallback
- verification: `cargo test -p dimweave session_event`

### Task 3: Update prompts and examples for strict schema

- `task_id`: prompt_update
- `allowed_files`: `src-tauri/src/daemon/role_config/roles.rs`, `src-tauri/src/daemon/role_config/roles_tests.rs`
- `max_files_changed`: 2
- acceptance criteria:
  - All target examples in prompt include all three required fields (`kind`, `role`, `agentId`)
  - Role-specific examples: lead shows delegation to coder, coder shows reporting to lead
  - `role_examples()` function provides per-role examples
  - Tests updated for new prompt content
- verification: `cargo test -p dimweave role_config`

### Task 4: Add parser compatibility tests for strict schema output

- `task_id`: parser_compat
- `allowed_files`: `src-tauri/src/daemon/codex/structured_output_tests.rs`
- `max_files_changed`: 1
- acceptance criteria:
  - Test: `{"kind":"user","role":"","agentId":""}` → `MessageTarget::User`
  - Test: `{"kind":"role","role":"coder","agentId":""}` → `MessageTarget::Role{role:"coder"}`
  - Test: `{"kind":"agent","role":"","agentId":"claude-1"}` → `MessageTarget::Agent{agent_id:"claude-1"}`
- verification: `cargo test -p dimweave structured_output`

### Task 5: Fix MessageList scrollerRef infinite loop

- `task_id`: scrollerref_fix
- `allowed_files`: `src/components/MessagePanel/MessageList.tsx`
- `max_files_changed`: 1
- acceptance criteria:
  - `setScrollerNode` uses functional updater `(prev) => prev === node ? prev : node`
  - No "Maximum update depth exceeded" error in dev or production
- verification: manual — open app, no React error in console

### Task 6: Enable debuggable DMG builds

- `task_id`: vite_debug
- `allowed_files`: `vite.config.ts`
- `max_files_changed`: 1
- acceptance criteria:
  - `minify: false` for readable source in DMG
  - `sourcemap: true` always enabled
  - File names use `[name].js` instead of `[name]-[hash].js`
- verification: `bun run build` produces readable asset names

---

## CM (Configuration Management)

### Task 1 CM: schema_fix
- **Status:** complete
- **Commit:** `b6958989`
- **Files:** `roles.rs`, `roles_tests.rs`
- **Verification:** `cargo test -p dimweave role_config` — 28 passed
- **Runtime:** Codex `turn/start` no longer returns `invalid_json_schema` error

### Task 2 CM: error_handler
- **Status:** complete
- **Commit:** `b6958989`
- **Files:** `session_event.rs`
- **Verification:** `cargo test -p dimweave session_event` — passed; runtime confirmed error messages reach GUI

### Task 3 CM: prompt_update
- **Status:** complete
- **Commit:** `b6958989`
- **Files:** `roles.rs`, `roles_tests.rs`
- **Verification:** `cargo test -p dimweave role_config` — 28 passed; lead examples show delegation to coder

### Task 4 CM: parser_compat
- **Status:** complete
- **Commit:** `b6958989`
- **Files:** `structured_output_tests.rs`
- **Verification:** `cargo test -p dimweave structured_output` — 36 passed (3 new empty-field tests)

### Task 5 CM: scrollerref_fix
- **Status:** complete
- **Commit:** `b6958989`
- **Files:** `MessageList.tsx`
- **Verification:** dev mode — no "Maximum update depth exceeded" error

### Task 6 CM: vite_debug
- **Status:** complete
- **Commit:** `b6958989`
- **Files:** `vite.config.ts`
- **Verification:** `bun run build` — assets named `index.js`, `markdown.js` (no hash)

---

## Post-Fix Notes

- The `reply` dynamic tool is intentionally NOT registered in `handshake.rs` `dynamicTools`. Structured output with `target` routing is the primary communication channel. The `reply` handler in `handler.rs` exists as a fallback but is not advertised to the model.
- The `get_status` tool description in `handshake.rs:12` has a minor JSON syntax error in its description string (`{"agentId", "role"}` instead of key-value pairs). Non-blocking — does not affect tool functionality.
- 2 pre-existing test failures in `state_persistence_tests` (filesystem permission issues in `/var/folders`) are unrelated to this fix.
