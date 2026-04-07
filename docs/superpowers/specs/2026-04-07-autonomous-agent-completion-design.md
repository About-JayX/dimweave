# Autonomous Agent Completion Design

## Summary

Dimweave's current prompt-and-skill stack still mixes two incompatible workflows:

1. a task protocol that expects per-task verification and CM tracking
2. legacy skill steps that still ask the user to review specs, choose execution mode, or implicitly act as the final acceptance gate

This design aligns the system with the intended operating model: agent-to-agent autonomous execution, lead-owned final acceptance, and real git commits for each completed task.

## Product Goal

- Remove unnecessary user-intervention gates from the agent workflow.
- Make lead responsible for final acceptance after deep review and verification.
- Clarify that each task requires a real focused git commit, not just a plan-document note.
- Keep the user informed of verified outcomes without making them a blocking step unless they explicitly request involvement.

## Scope

### In scope

- runtime role protocol text under `src-tauri/src/daemon/role_config/`
- prompt tests that verify the new autonomous-completion rules
- superpowers skills that currently force user review / user execution-choice handoff
- finishing the already-implemented chat/inspector bugfix wave by turning the dirty worktree into real task-scoped commits

### Out of scope

- changing transport/routing mechanics between agents
- redesigning the entire skill system
- altering unrelated product behavior

## Root Causes

### 1. Prompt protocol only records document-level CM entries

`role_protocol.rs` requires a CM entry per task, but it does not explicitly require an actual git commit before proceeding. This lets the workflow satisfy the letter of documentation while leaving the repository dirty.

### 2. Final review exists, final autonomous acceptance does not

The role protocol requires a final deep review before reporting to the user, but it does not clearly state that lead performs final acceptance and should not block on user approval unless the user explicitly requests it.

### 3. Skills still force user gates

`brainstorming` currently requires the user to review the written spec before implementation.

`writing-plans` currently requires the user to choose the execution mode after the plan is written.

Those rules contradict the intended autonomous multi-agent workflow.

## Design Decisions

### A. Strengthen the runtime prompt contract

The lead protocol should explicitly say:

- each task must end in a real focused git commit
- the plan document must record the real CM evidence for that task
- lead performs final acceptance after final deep review and verification
- user notification is the default end state; user approval is only required when explicitly requested by the user

### B. Remove default user-review gates from skills

`brainstorming` should treat user design approval as the approval gate, but once the user says to proceed autonomously, the written spec no longer needs a second mandatory user review checkpoint.

`writing-plans` should default to subagent-driven execution without asking the user to choose between execution modes, unless the user explicitly asks for a different path.

### C. Finish the bugfix work by creating real commits

The chat/inspector bugfix work is already implemented and verified, but it still exists only as dirty working tree changes.

To match the intended protocol:

- split the current tree into task-aligned commits
- keep commit messages aligned with the task-level CM entries already recorded in the plan
- preserve the previously verified file/task boundaries

## Acceptance Criteria

- Lead and coder prompts explicitly describe autonomous final acceptance and real task-scoped commits.
- No default skill step requires user review of the spec or user selection of execution mode when the user has already approved autonomous execution.
- The current chat/inspector bugfix work is no longer left as an uncommitted dirty worktree.
- The plan for the bugfix wave records task-level CM entries that correspond to real git commits.
