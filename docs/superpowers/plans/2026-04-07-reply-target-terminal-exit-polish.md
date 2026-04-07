# Reply Target, Terminal Scroll, and Shutdown Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Preserve the selected reply target per task, make the logs surface open at the bottom, and harden app shutdown so all tracked runtimes/connections are torn down cleanly.

**Architecture:** Keep reply-target memory as frontend task-scoped UI state, not daemon persistence. Add bottom-oriented scroll handling to the logs surface with the existing Virtuoso stack. Keep shutdown centralized in the daemon/app exit path and explicitly clear all tracked runtime/connection handles before allowing process exit.

**Tech Stack:** React 19, Zustand, TypeScript, Tauri 2, Rust, tokio, Bun, Cargo

---

## File Map

### Reply target memory

- `src/components/ReplyInput/index.tsx`
- `src/stores/task-store/types.ts`
- `src/stores/task-store/index.ts`
- `src/stores/task-store/selectors.ts`
- `tests/task-store.test.ts`
- `src/components/ReplyInput/index.test.tsx`

### Logs surface scroll

- `src/components/MessagePanel/index.tsx`
- `src/components/MessagePanel/MessageList.tsx`
- `src/components/MessagePanel/index.test.tsx`

### Shutdown cleanup

- `src-tauri/src/main.rs`
- `src-tauri/src/daemon/mod.rs`
- `src-tauri/src/daemon/state.rs`
- `src-tauri/src/daemon/state_runtime.rs`
- `src-tauri/src/daemon/state_tests.rs`

## CM Memory

| Task | Commit | Review | Verification | Memory |
|------|--------|--------|--------------|--------|
| Task 1 | `dda01a6c` | `manual diff review` | `bun test tests/task-store.test.ts src/components/ReplyInput/index.test.tsx`; `git diff --check` | Reply target memory belongs to task-scoped frontend state, not daemon persistence. |
| Task 2 | `500ffcff` | `manual diff review` | `bun test src/components/MessagePanel/index.test.tsx`; `bun run build`; `git diff --check` | Logs should default to bottom on entry, but should not fight the user after manual upward scrolling. |
| Task 3 | `PENDING` | `PENDING` | `PENDING` | App exit should be a single teardown barrier: runtimes stopped, connection handles cleared, then process exit. |

### Task 1: Persist reply target per active task

**Files:**
- Modify: `src/components/ReplyInput/index.tsx`
- Modify: `src/stores/task-store/types.ts`
- Modify: `src/stores/task-store/index.ts`
- Modify: `src/stores/task-store/selectors.ts`
- Modify: `tests/task-store.test.ts`
- Modify: `src/components/ReplyInput/index.test.tsx`

- [x] **Step 1: Write failing tests for task-scoped reply target memory**

Cover:

- per-task target storage in task store
- restoring the previous target for the active task instead of defaulting to `"auto"`
- different tasks keeping independent target values

- [x] **Step 2: Run the targeted tests to verify they fail**

Run:

```bash
bun test tests/task-store.test.ts src/components/ReplyInput/index.test.tsx
```

Expected: FAIL because reply target still lives in local component state.

- [x] **Step 3: Implement task-scoped target memory**

Implementation notes:

- add task-target preference state/actions to the task store
- resolve the active reply target from task store in `ReplyInput`
- update the picker to write through the task-scoped setter
- fall back to `"auto"` when there is no active task

- [x] **Step 4: Re-run verification**

Run:

```bash
bun test tests/task-store.test.ts src/components/ReplyInput/index.test.tsx
git diff --check
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/components/ReplyInput/index.tsx src/stores/task-store/types.ts src/stores/task-store/index.ts src/stores/task-store/selectors.ts tests/task-store.test.ts src/components/ReplyInput/index.test.tsx
git commit -m "fix: remember reply target per task"
```

- [x] **Step 6: Update `## CM Memory`**

### Task 2: Open logs at the bottom and preserve sensible follow behavior

**Files:**
- Modify: `src/components/MessagePanel/index.tsx`
- Modify: `src/components/MessagePanel/MessageList.tsx`
- Modify: `src/components/MessagePanel/index.test.tsx`

- [x] **Step 1: Write failing tests for logs-surface bottom behavior**

Cover:

- switching to logs should scroll to the latest entry
- follow behavior should continue only when already at bottom

- [x] **Step 2: Run the targeted tests to verify they fail**

Run:

```bash
bun test src/components/MessagePanel/index.test.tsx
```

Expected: FAIL because logs surface has no explicit bottom-on-entry logic.

- [x] **Step 3: Implement bottom-oriented logs behavior**

Implementation notes:

- add refs/state for the logs Virtuoso instance
- scroll to the last item when entering `"logs"`
- preserve bottom-follow semantics without overriding manual upward scroll

- [x] **Step 4: Re-run verification**

Run:

```bash
bun test src/components/MessagePanel/index.test.tsx
git diff --check
```

Expected: PASS.

- [x] **Step 5: Commit**

```bash
git add src/components/MessagePanel/index.tsx src/components/MessagePanel/MessageList.tsx src/components/MessagePanel/index.test.tsx
git commit -m "fix: keep logs pinned to the bottom on entry"
```

- [x] **Step 6: Update `## CM Memory`**

### Task 3: Tear down all tracked runtimes/connections on app exit

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/daemon/state.rs`
- Modify: `src-tauri/src/daemon/state_runtime.rs`
- Modify: `src-tauri/src/daemon/state_tests.rs`

- [ ] **Step 1: Write failing tests for explicit shutdown teardown**

Cover:

- shutdown clears tracked runtime/connection handles
- shutdown path does not leave daemon-side agent/runtime state marked live after stop

- [ ] **Step 2: Run the targeted tests to verify they fail**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests::
```

Expected: FAIL because shutdown currently stops runtimes but does not fully assert/clear every tracked connection boundary.

- [ ] **Step 3: Implement explicit shutdown teardown**

Implementation notes:

- centralize a daemon/app shutdown cleanup helper
- stop Codex and Claude runtimes
- clear attached agent/runtime senders and live provider connection state
- keep the app exit barrier waiting on daemon shutdown completion

- [ ] **Step 4: Re-run verification**

Run:

```bash
cargo test --manifest-path src-tauri/Cargo.toml daemon::state::state_tests::
bun run build
git diff --check
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs src-tauri/src/daemon/mod.rs src-tauri/src/daemon/state.rs src-tauri/src/daemon/state_runtime.rs src-tauri/src/daemon/state_tests.rs
git commit -m "fix: fully tear down runtimes on app exit"
```

- [ ] **Step 6: Update `## CM Memory`**
