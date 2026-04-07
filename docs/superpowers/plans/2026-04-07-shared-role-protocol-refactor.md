# Shared Role Protocol Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor Dimweave role prompts so Claude and Codex share one source of role-policy truth, while tightening reviewer read-only wording and clarifying factual-error correction routing.

**Architecture:** Add a lightweight shared role-protocol module under `src-tauri/src/daemon/role_config/`, then make `claude_prompt.rs` and `roles.rs` render provider-specific prompt structure from those shared fragments. Keep launch/runtime behavior unchanged.

**Tech Stack:** Rust, cargo test, existing daemon role_config module, git

---

## File Map

### New files

- `src-tauri/src/daemon/role_config/role_protocol.rs`

### Modified files

- `src-tauri/src/daemon/role_config/mod.rs`
- `src-tauri/src/daemon/role_config/claude_prompt.rs`
- `src-tauri/src/daemon/role_config/roles.rs`
- `src-tauri/src/daemon/role_config/roles_tests.rs`
- `docs/superpowers/specs/2026-04-07-shared-role-protocol-design.md`

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `PENDING` | `self-review` | `git diff --check -- docs/superpowers/specs/2026-04-07-shared-role-protocol-design.md docs/superpowers/plans/2026-04-07-shared-role-protocol-refactor.md` | Shared role-policy changes must be documented before prompt text starts drifting again. |
| Task 2 | `PENDING` | `PENDING` | `PENDING` | Shared role fragments must keep provider-specific transport/tool instructions separate from role-policy text. |

### Task 1: Record the approved design and execution contract

**Files:**
- Create: `docs/superpowers/specs/2026-04-07-shared-role-protocol-design.md`
- Create: `docs/superpowers/plans/2026-04-07-shared-role-protocol-refactor.md`

- [ ] **Step 1: Write the approved design spec**

Document:

- why Claude/Codex prompt drift is a maintenance risk
- the new shared `role_protocol.rs` module
- reviewer read-only wording
- factual-error correction routing rule
- explicit scope exclusions

- [ ] **Step 2: Write the implementation plan with CM tracking**

The plan must include:

- exact file paths
- task-by-task execution
- `## CM Memory`
- verification commands for each task

- [ ] **Step 3: Verify doc formatting**

Run:

```bash
git diff --check -- docs/superpowers/specs/2026-04-07-shared-role-protocol-design.md docs/superpowers/plans/2026-04-07-shared-role-protocol-refactor.md
```

Expected: no whitespace or patch-format issues.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-04-07-shared-role-protocol-design.md docs/superpowers/plans/2026-04-07-shared-role-protocol-refactor.md
git commit -m "docs: record shared role protocol refactor plan"
```

- [ ] **Step 5: Update `## CM Memory`**

Replace Task 1 placeholders with the real commit hash and verification evidence before starting Task 2.

### Task 2: Rebuild provider prompts from a shared role-protocol layer

**Files:**
- Create: `src-tauri/src/daemon/role_config/role_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/mod.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt.rs`
- Modify: `src-tauri/src/daemon/role_config/roles.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`

- [ ] **Step 1: Write failing tests for the missing shared reviewer/correction invariants**

Add focused regressions that require:

- Claude reviewer prompt to state that reviewer must not modify files or act as the primary implementer
- Codex reviewer prompt to state the same restriction
- factual-error correction language to mention routing policy
- lead prompt to retain "full permissions"

Run:

```bash
cargo test reviewer_prompt_requires_read_only_protocol --manifest-path src-tauri/Cargo.toml
cargo test factual_error_correction_still_respects_routing_policy --manifest-path src-tauri/Cargo.toml
```

Expected: FAIL before the refactor because the current prompt builders do not share those invariants consistently.

- [ ] **Step 2: Implement the shared role-protocol module**

Create `src-tauri/src/daemon/role_config/role_protocol.rs` with shared helpers for:

```rust
pub fn role_desc(role_id: &str) -> &str
pub fn roles_section() -> &'static str
pub fn subject_matter_authority() -> &'static str
pub fn security_research_policy() -> &'static str
pub fn role_specific_rules(role_id: &str) -> &'static str
pub fn correction_routing_rule() -> &'static str
```

Use it from both provider prompt builders instead of duplicating role-policy text.

- [ ] **Step 3: Re-run the focused regression tests**

Run:

```bash
cargo test reviewer_prompt_requires_read_only_protocol --manifest-path src-tauri/Cargo.toml
cargo test factual_error_correction_still_respects_routing_policy --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [ ] **Step 4: Run the broader role/prompt verification**

Run:

```bash
cargo test claude_prompt --manifest-path src-tauri/Cargo.toml
cargo test daemon::role_config::roles::tests:: --manifest-path src-tauri/Cargo.toml
cargo test build_claude_command_sets_sdk_args_and_env --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: PASS with no diff-format issues.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/daemon/role_config/mod.rs src-tauri/src/daemon/role_config/role_protocol.rs src-tauri/src/daemon/role_config/claude_prompt.rs src-tauri/src/daemon/role_config/roles.rs src-tauri/src/daemon/role_config/roles_tests.rs
git commit -m "refactor: share dimweave role prompt protocol"
```

- [ ] **Step 6: Update `## CM Memory`**

Replace Task 2 placeholders with the real commit hash, verification commands, and learned prompt-maintenance rule before closing the work.
