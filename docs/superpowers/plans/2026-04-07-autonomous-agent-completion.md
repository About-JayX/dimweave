# Autonomous Agent Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Align Dimweave's prompts and workflow skills with a fully autonomous agent-delivery model, then finish the already-verified chat/inspector bugfix work by turning it into real task-scoped commits.

**Architecture:** First tighten the runtime role protocol so lead-owned final acceptance, real per-task git commits, and non-blocking user notification are explicit. Then update the affected skills so they no longer force user review/choice gates by default. Finally, convert the existing verified UI bugfix worktree into task-aligned commits and record the real CM evidence in the plan.

**Tech Stack:** Rust prompt builders/tests, markdown skill docs, git, Bun, Cargo

---

## File Map

- Modify: `src-tauri/src/daemon/role_config/role_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt.rs`
- Modify: `src-tauri/src/daemon/role_config/roles.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`
- Modify: `/.agents/skills/superpowers/brainstorming/SKILL.md`
- Modify: `/.agents/skills/superpowers/writing-plans/SKILL.md`
- Modify: `docs/superpowers/plans/2026-04-07-chat-timeline-inspector-bugfixes.md`
- Modify: task-scoped tracked files already changed under `src/components/**`

## CM Memory

| Task | Commit | Verification | Notes |
|------|--------|--------------|-------|
| Task 1 | `17a06f55` | `cargo test lead_prompt_enforces_planning_review_reporting_role --manifest-path src-tauri/Cargo.toml`; `cargo test prompt_requires_autonomous_final_acceptance --manifest-path src-tauri/Cargo.toml`; `cargo test role_config::roles::tests --manifest-path src-tauri/Cargo.toml` | Runtime prompt must own autonomous completion rules directly; skill docs alone are insufficient because providers render their prompts from `role_protocol.rs`. |
| Task 2 | `80191d01` | `git diff --check -- .agents/skills/superpowers/brainstorming/SKILL.md .agents/skills/superpowers/writing-plans/SKILL.md` | Skill-level user-review gates must become opt-in rather than default when the user has already approved autonomous execution. |
| Task 3 | `bee9dd20` | `bun test src/components/MessagePanel/MessageList.test.tsx src/components/MessagePanel/index.test.tsx src/components/MessagePanel/CodexStreamIndicator.test.ts src/components/ui/cyber-select.test.tsx src/components/TaskContextPopover.test.tsx src/components/TaskPanel/ArtifactTimeline.test.tsx src/components/TaskPanel/TaskHeader.test.tsx`; `bun run build`; `git status --short` | Previously verified UI bugfix tasks must be converted into real git commits so CM records correspond to actual repository history, not only plan-document checkmarks. |

---

### Task 1: Tighten the runtime prompt contract for autonomous completion

**Acceptance criteria:**
- Lead prompt explicitly requires a real focused git commit per task before advancing.
- Lead prompt explicitly states final acceptance is performed by lead after final deep review and verification.
- Prompt text makes user notification the default end state, not a mandatory approval gate, unless the user explicitly requests involvement.
- Prompt tests cover the new invariants.

**Files:**
- Modify: `src-tauri/src/daemon/role_config/role_protocol.rs`
- Modify: `src-tauri/src/daemon/role_config/claude_prompt.rs`
- Modify: `src-tauri/src/daemon/role_config/roles_tests.rs`

- [x] **Step 1: Add/update prompt tests for autonomous acceptance and real commits**

Add focused assertions that require the lead prompt to contain wording equivalent to:
- each task must end with a real focused git commit
- CM entry must record the real commit evidence
- lead performs final acceptance after final deep review
- user approval is only required when explicitly requested by the user

- [x] **Step 2: Update `role_protocol.rs` lead rules**

Revise the lead section so the Plan Execution Protocol says:
- verify task
- create real git commit for task
- record CM entry with real commit evidence
- do not proceed until both commit and CM entry exist
- after all tasks, run final deep review, final verification, and final acceptance before reporting to user

- [x] **Step 3: Re-run focused prompt tests**

Run:

```bash
cargo test lead_prompt_enforces_planning_review_reporting_role --manifest-path src-tauri/Cargo.toml
cargo test prompt_requires_autonomous_final_acceptance --manifest-path src-tauri/Cargo.toml
cargo test role_config::roles::tests --manifest-path src-tauri/Cargo.toml
```

Expected: PASS.

- [x] **CM:** `refactor: require autonomous final acceptance in role prompts` — commit `17a06f55`

---

### Task 2: Remove default user-gated review/choice steps from the active skills

**Acceptance criteria:**
- `brainstorming` no longer forces a mandatory user review of the written spec when autonomous execution has already been approved.
- `writing-plans` no longer forces a user choice between execution modes by default.
- Both skills preserve explicit user override behavior when the user asks to review or choose.

**Files:**
- Modify: `/.agents/skills/superpowers/brainstorming/SKILL.md`
- Modify: `/.agents/skills/superpowers/writing-plans/SKILL.md`

- [x] **Step 1: Update `brainstorming` to remove the default written-spec user gate**

Change the checklist/process wording so:
- user design approval still happens before implementation
- after the spec is written and self-reviewed, lead may proceed directly to writing-plans unless the user explicitly asked to review the spec file

- [x] **Step 2: Update `writing-plans` to default to autonomous execution**

Replace the current “Which approach?” handoff with default guidance:
- use subagent-driven execution by default
- only ask the user if they explicitly requested a different execution mode

- [x] **Step 3: Verify skill diffs are clean**

Run:

```bash
git diff --check -- .agents/skills/superpowers/brainstorming/SKILL.md .agents/skills/superpowers/writing-plans/SKILL.md
```

Expected: no formatting issues.

- [x] **CM:** `docs: remove default user gates from autonomous workflows` — commit `80191d01`

---

### Task 3: Finish the pending chat/inspector bugfix work with real task commits

**Acceptance criteria:**
- The dirty worktree for the chat/inspector bugfix wave is converted into real task-aligned commits.
- Each commit corresponds to one previously verified task boundary.
- The plan document for that wave records real commit evidence instead of placeholder CM-only text.
- Repository is clean after verification.

**Files:**
- Modify: `docs/superpowers/plans/2026-04-07-chat-timeline-inspector-bugfixes.md`
- Modify/commit existing verified bugfix files already changed in `src/components/**`

- [x] **Step 1: Inspect the current dirty tree and map files back to Tasks 1-4**

Confirm the current modified files still partition cleanly into:
- Task 1 stream tail + bottom anchoring
- Task 2 search row
- Task 3 provider history select
- Task 4 task inspector layout

- [x] **Step 2: Create real task-scoped commits in order**

Commit each task separately with focused staging. Use commit messages aligned to the existing task summaries:

```bash
fix: inline transient stream tail into message timeline
fix: move message search into dedicated panel row
fix: show provider history with title and task metadata
refactor: simplify task inspector structure
```

- [x] **Step 3: Update the bugfix plan with real CM evidence**

Replace the current CM-only checkboxes in `docs/superpowers/plans/2026-04-07-chat-timeline-inspector-bugfixes.md` with real commit evidence for each task.

- [x] **Step 4: Run final verification on the completed bugfix branch**

Run:

```bash
bun test \
  src/components/MessagePanel/MessageList.test.tsx \
  src/components/MessagePanel/index.test.tsx \
  src/components/MessagePanel/CodexStreamIndicator.test.ts \
  src/components/ui/cyber-select.test.tsx \
  src/components/TaskContextPopover.test.tsx \
  src/components/TaskPanel/ArtifactTimeline.test.tsx \
  src/components/TaskPanel/TaskHeader.test.tsx
bun run build
git status --short
```

Expected:
- tests pass
- build succeeds
- working tree clean except for this plan/spec task if not yet committed

- [x] **CM:** `docs: backfill real commit evidence into chat/inspector bugfix plan` — commit `bee9dd20`
