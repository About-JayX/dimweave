# Shared Role Protocol Design

## Summary

Dimweave's role protocol currently lives in two separate prompt builders: Claude append-system-prompt text and Codex base instructions. The content has already drifted. `lead/coder` behavior is mostly aligned, but `reviewer` permissions and the "significant factual error" exception are not expressed consistently, which makes the current prompt logic harder to trust and maintain.

This design introduces a shared role-protocol layer under `src-tauri/src/daemon/role_config/`. Shared role facts are defined once, then rendered into Claude-specific and Codex-specific prompt text. The provider launch/runtime code stays unchanged for this refactor.

## Product Goal

- Make Claude and Codex consume one shared source of role behavior.
- Keep `lead` high-permission and non-implementing.
- Make `reviewer` explicitly read-only in prompt logic, even when tooling is permissive.
- Clarify that significant factual-error correction still follows routing policy by default.
- Reduce future drift between provider prompt templates.

## Scope

### Included

- A new shared role-protocol module in `src-tauri/src/daemon/role_config/`.
- Rebuilding Claude prompt text from shared role fragments.
- Rebuilding Codex base instructions from shared role fragments.
- Prompt/test updates for:
  - reviewer read-only behavior
  - lead high-permission behavior
  - factual-error correction staying inside routing policy unless the role was explicitly asked to answer
- Plan/CM documentation for this work.

### Excluded

- Changing Claude launch flags or Codex resume behavior.
- Reworking daemon routing implementation itself.
- Replacing the current output schema or reply protocol.
- Any provider/runtime sandbox changes.

## Architecture

### Shared protocol layer

Add a new module, `role_protocol.rs`, that owns:

- role descriptors used by both providers
- shared subject-matter authority text
- shared security-research policy text
- role-specific rule blocks for `user`, `lead`, `coder`, `reviewer`
- normalized correction/routing language

The module should stay string-based and lightweight. This is a prompt-composition refactor, not a new type system.

### Provider renderers

- `claude_prompt.rs` remains the Claude renderer.
  - It keeps Claude-specific sections like `reply(to, text, status)` and `get_online_agents()`.
  - It pulls common role descriptors and role-specific rule blocks from the shared protocol layer.
- `roles.rs` remains the Codex renderer.
  - It keeps Codex-specific sections like JSON `send_to` output and `get_status()`.
  - It also pulls common role descriptors and rule blocks from the shared protocol layer.

### Permission semantics

- `lead`: explicitly high-permission, but still forbidden from acting as the primary implementer.
- `coder`: implementation-capable and locked to the approved plan.
- `reviewer`: explicitly forbidden from modifying files or acting as an implementer, even if the environment could technically allow it.

The goal is role-policy consistency, not hard sandbox enforcement.

## Behavior Changes

### Reviewer

Both providers should say the same thing:

- reviewer analyzes code, runs tests, verifies behavior
- reviewer must not modify files
- reviewer must not act as the primary implementer
- reviewer routes findings to `coder` or `lead` according to context

### Significant factual error correction

Both providers should clarify:

- correcting a significant factual error is allowed
- this does not create a free pass to answer the human user directly
- non-lead roles still follow routing policy unless the user explicitly asked that role to answer

## File Plan

### New file

- `src-tauri/src/daemon/role_config/role_protocol.rs`

### Modified files

- `src-tauri/src/daemon/role_config/mod.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`
- `src-tauri/src/daemon/role_config/roles.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`

## Testing Strategy

- Add focused prompt regression tests before implementation:
  - Claude reviewer prompt includes explicit read-only / non-implementer rules
  - Codex reviewer prompt includes the same constraints
  - factual-error correction language mentions routing policy
  - lead prompt still states full permissions
- Re-run the existing role-config and Claude prompt tests after the refactor.
- Re-run the Claude launch prompt-argument test to make sure the prompt injection path still receives non-empty role text.

## Acceptance Criteria

- Claude and Codex prompts read shared role-policy content from one module.
- `reviewer` constraints are explicit and consistent across both providers.
- `lead` still has full permissions in prompt wording.
- factual-error correction language no longer implies that non-target roles can bypass routing.
- role-config tests pass with the refactored prompt builders.
