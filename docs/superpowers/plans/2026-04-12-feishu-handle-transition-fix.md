# Feishu Handle-to-Implementing Transition Fix Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When the user clicks Handle, the lead still receives bug context and plans the fix as today; then on the first lead→coder implementation handoff, the task enters `Implementing` and the linked Feishu bug is transitioned to `处理中`.

**Architecture:** Keep the existing Handle-bug task creation and snapshot/handoff flow. Add one task-flow transition for `lead -> coder`, then attach a Feishu side effect after routing succeeds: look up the linked bug by `linked_task_id`, query transitable states, find `处理中`, and call `transition_state` with the returned `transition_id`. If Feishu rejects the transition (for example `No Permission`), do not block routing; log and surface the failure while leaving the task chain intact.

**Tech Stack:** Rust, Tauri daemon, Feishu MCP HTTP client, Cargo test.

---

## Baseline Evidence

- Isolated worktree: `.worktrees/feishu-handle-transition-fix` on branch `fix/feishu-handle-transition-fix`
- Baseline verification before implementation:
  - `cargo build --manifest-path bridge/Cargo.toml`
  - `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`
- Baseline result: pass

## Project Memory

### Recent related commits

- `9d7af78a` — initial Handle-bug task launch flow
- `7bd19593` — description extraction for Feishu handoff
- `a122e6fc` — split visible filtered view from raw sync cache
- `7cb061bd` / `b9eba99e` — current-owner filter semantics

### Verified runtime evidence

- Current Handle flow:
  - click `Handle` → `feishu_project_start_handling`
  - daemon fetches context via `get_workitem_brief` + `list_workitem_comments`
  - creates/selects task
  - writes snapshot JSON
  - routes handoff message to `lead`
- Current task-flow transitions:
  - `coder -> lead (done)` → `Reviewing`
  - `lead -> user (done)` → `Done`
  - **No** `lead -> coder` → `Implementing`
- Feishu transition capability is available:
  - `get_transitable_states(project_key, work_item_id, work_item_type, user_key)` returns a transition with:
    - `state_name = 处理中`
    - `state_key = IN PROGRESS`
    - `id = 24640619` (sample issue `6948545648`)
  - `transition_state(project_key, work_item_id, transition_id)` is the correct tool shape per live `tools/list` schema
- Real environment caveat:
  - sample transition call returns `No Permission`
  - so status-transition execution must be best-effort and non-blocking

### Lessons that constrain this plan

- Do not alter the existing Handle-bug context/snapshot flow.
- Do not gate coder dispatch on Feishu status-update success.
- Use the linked-bug relation already stored on issue items; do not add a second linkage model.
- Only the first real lead→coder handoff should flip task status to `Implementing`.

## File Map

- Modify: `src-tauri/src/daemon/orchestrator/task_flow.rs`
- Modify: `src-tauri/src/daemon/orchestrator/tests.rs`
- Modify: `src-tauri/src/daemon/routing_dispatch.rs`
- Modify: `src-tauri/src/daemon/feishu_project_task_link.rs`
- Modify: `src-tauri/src/daemon/feishu_project_task_link_tests.rs`

## CM Memory

| Task | Planned commit message | Verification | Memory |
|------|------------------------|--------------|--------|
| Task 1 | `fix: transition feishu bug when implementation starts` | `cargo test --manifest-path src-tauri/Cargo.toml daemon::orchestrator::tests:: -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml daemon::feishu_project_task_link_tests:: -- --nocapture`; `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture` | Add a `lead -> coder` Implementing transition in the task graph and trigger a best-effort Feishu `transition_state` side effect after successful routing. Feishu transition failures must not block task routing. |

---

### Task 1: Start implementation state on first lead→coder handoff

**task_id:** `feishu-handle-transition-code`

**Acceptance criteria:**

- First successful `lead -> coder` routed message for a Feishu-linked task updates task status to `Implementing`.
- The daemon attempts to transition the linked Feishu bug to `处理中` using `get_transitable_states` + `transition_state`.
- If Feishu returns `No Permission` or other transition failure, the lead→coder message still delivers and task status still changes to `Implementing`.
- Existing Handle-bug snapshot/handoff flow remains unchanged.

**allowed_files:**

- `src-tauri/src/daemon/orchestrator/task_flow.rs`
- `src-tauri/src/daemon/orchestrator/tests.rs`
- `src-tauri/src/daemon/routing_dispatch.rs`
- `src-tauri/src/daemon/feishu_project_task_link.rs`
- `src-tauri/src/daemon/feishu_project_task_link_tests.rs`

**max_files_changed:** `5`

**max_added_loc:** `180`

**max_deleted_loc:** `70`

**verification_commands:**

- `cargo test --manifest-path src-tauri/Cargo.toml daemon::orchestrator::tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml daemon::feishu_project_task_link_tests:: -- --nocapture`
- `cargo test --manifest-path src-tauri/Cargo.toml feishu_project -- --nocapture`

- [ ] **Step 1: Add failing tests first**

Add tests proving:

- `lead -> coder` routed messages move a task from `Draft/Planning` into `Implementing`
- the Feishu helper can locate the linked bug by `task_id`
- transition target selection finds `处理中` from `get_transitable_states`
- Feishu transition failure is non-blocking

- [ ] **Step 2: Run verification and confirm failure before implementation**

- [ ] **Step 3: Implement the minimal task-flow + Feishu side effect**

Make only these changes:

- update task-flow rules so `lead -> coder` starts implementation
- add a helper in `feishu_project_task_link.rs` to:
  - find the bug item by `linked_task_id`
  - resolve a user key from detail role members
  - call `get_transitable_states`
  - pick the `处理中` transition
  - call `transition_state(project_key, work_item_id, transition_id)`
- invoke that helper from `routing_dispatch.rs` after successful message delivery

Do not:

- change frontend code
- change the Handle snapshot payload shape
- block coder routing on Feishu API failure

- [ ] **Step 4: Re-run verification**

- [ ] **Step 5: Commit**

```bash
git add \
  src-tauri/src/daemon/orchestrator/task_flow.rs \
  src-tauri/src/daemon/orchestrator/tests.rs \
  src-tauri/src/daemon/routing_dispatch.rs \
  src-tauri/src/daemon/feishu_project_task_link.rs \
  src-tauri/src/daemon/feishu_project_task_link_tests.rs
git commit -m "fix: transition feishu bug when implementation starts"
```

- [ ] **Step 6: Update `## CM Memory` with the real commit SHA after lead review**
