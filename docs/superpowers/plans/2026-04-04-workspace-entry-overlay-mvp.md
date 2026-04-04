# Workspace Entry Overlay MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace per-agent workspace pickers with a shell-owned workspace entry flow: a blocking startup overlay plus a top-right switcher that always creates a fresh task context from exactly one selected workspace.

**Architecture:** Workspace selection becomes shell UI state rather than provider config. A small pure helper module owns exclusive selection and recent-workspace persistence in `localStorage`, while `App.tsx` gates entry until a task is created. After entry, `ShellTopBar` hosts a compact workspace switcher, and Claude/Codex panels consume the active task workspace as read-only input for provider history and connection flows.

**Tech Stack:** React 19, Zustand, Tauri invoke, Bun tests, Tailwind CSS

---

## Execution Rules

- Use `superpowers:test-driven-development` before each task implementation.
- Before every commit, run a deep review loop with `superpowers:requesting-code-review` and address findings until clean.
- Do not start the next task until the current task is verified, reviewed, committed, and the commit record below is updated.
- Keep new files under the repo's soft 200-line limit by splitting helpers/components when needed.

## File Map

### New files

- `src/components/workspace-entry-state.ts`
  - Pure helper functions for:
    - single selected workspace candidate
    - recent workspace normalization
    - recent workspace insert/dedupe/cap
- `src/components/workspace-entry-state.test.ts`
  - Bun tests for helper behavior
- `src/components/WorkspaceEntryOverlay.tsx`
  - Full-screen startup overlay with logo, description, chooser, recent list, and `Continue`
- `src/components/WorkspaceEntryOverlay.test.tsx`
  - Render and selection-state tests for the overlay
- `tests/app-workspace-bootstrap.test.tsx`
  - App-level regression coverage for bootstrap loading and error blocking states
- `tests/agent-workspace-readonly.test.tsx`
  - Regression coverage for read-only workspace rows in Claude/Codex config panels
- `src/components/AppBootstrapGate.tsx`
  - Blocking loading/error shell gate shown before bootstrap completes
- `src/components/WorkspaceSwitcher.tsx`
  - Compact top-right switcher reusing the same selection model
- `src/components/WorkspaceSwitcher.test.tsx`
  - Render tests for empty/current workspace states and popover content

### Modified files

- `src/App.tsx`
  - Own bootstrap state, `workspaceActionError`, recent workspace state, and shell-level workspace entry/switch actions
  - Gate the shell behind `AppBootstrapGate` and `WorkspaceEntryOverlay`
- `tests/task-store.test.ts`
  - Add bootstrap regression coverage for the fresh-session rule
- `src-tauri/src/commands_task.rs`
  - Expose a command to clear active task selection at app bootstrap
- `src-tauri/src/daemon/cmd.rs`
  - Add the daemon command variant for clearing the active task
- `src-tauri/src/daemon/mod.rs`
  - Handle the clear-active-task command during bootstrap
- `src-tauri/src/main.rs`
  - Register the new Tauri command
- `src/components/ShellTopBar.tsx`
  - Replace passive workspace pill with `WorkspaceSwitcher`
- `src/components/ShellTopBar.test.tsx`
  - Update for interactive workspace control
- `src/stores/task-store/types.ts`
  - Add focused actions/state for bootstrap clearing, `bootstrapComplete`, and starting a workspace task if needed
- `src/stores/task-store/index.ts`
  - Add the bootstrap clear ordering and task-start implementation used by overlay and switcher
- `src/components/shell-layout-state.ts`
  - Remove provider-session workspace fallback from shell label resolution
- `tests/shell-layout-state.test.ts`
  - Update shell workspace label tests for active-task-only ownership
- `src/components/ClaudePanel/index.tsx`
  - Remove local `cwd` ownership and folder picker usage
- `src/components/ClaudePanel/ClaudeConfigRows.tsx`
  - Convert project row to read-only display
- `src/components/AgentStatus/CodexPanel.tsx`
  - Remove local `cwd` ownership and folder picker usage
- `src/components/AgentStatus/CodexConfigRows.tsx`
  - Convert project row to read-only display
- `src/components/AgentStatus/provider-session-view-model.ts`
  - Remove provider-session `cwd` fallback from workspace ownership and history scoping
- `tests/provider-session-view-model.test.ts`
  - Regression coverage for active-task-first workspace resolution

## Commit Log

| Task | Commit | Status | Notes |
|------|--------|--------|-------|
| Task 1 | `a415e5f6` | Done | Workspace selection helper + recent history model |
| Task 2 | `44e0cd29` | Done | Blocking entry overlay MVP |
| Task 3 | `6c197d52` | Done | Top-right switcher + task start flow |
| Task 4 | `619162ce` | Done | Agent panel cleanup + regressions |

### Task 1: Workspace Selection State Helpers

**Files:**
- Create: `src/components/workspace-entry-state.ts`
- Test: `src/components/workspace-entry-state.test.ts`

- [ ] **Step 1: Write the failing helper tests**

```ts
describe("workspace entry state", () => {
  test("replaces a previous recent selection when a folder is picked", () => {
    expect(
      selectWorkspaceCandidate(
        { type: "picked", path: "/repo-b" },
        { type: "recent", path: "/repo-a" },
      ),
    ).toEqual({ type: "picked", path: "/repo-b" });
  });

  test("replaces a previous picked folder when a recent workspace is selected", () => {
    expect(
      selectWorkspaceCandidate(
        { type: "recent", path: "/repo-a" },
        { type: "picked", path: "/repo-b" },
      ),
    ).toEqual({ type: "recent", path: "/repo-a" });
  });

  test("deduplicates and caps recent workspaces", () => {
    expect(
      pushRecentWorkspace(
        ["/repo-a", "/repo-b", "/repo-a"],
        "/repo-c",
        3,
      ),
    ).toEqual(["/repo-c", "/repo-a", "/repo-b"]);
  });

  test("normalizes corrupted storage payloads safely", () => {
    expect(loadRecentWorkspaces("not-json")).toEqual([]);
    expect(loadRecentWorkspaces("{\"bad\":true}")).toEqual([]);
  });
});
```

- [ ] **Step 2: Run the helper test to verify it fails**

Run: `bun test src/components/workspace-entry-state.test.ts`
Expected: FAIL because the helper module does not exist yet.

- [ ] **Step 3: Write the minimal helper implementation**

```ts
export type WorkspaceCandidate =
  | { type: "picked"; path: string }
  | { type: "recent"; path: string };

export function selectWorkspaceCandidate(
  next: WorkspaceCandidate,
  _current: WorkspaceCandidate | null,
): WorkspaceCandidate {
  return next;
}

export function loadRecentWorkspaces(raw: string | null): string[] {
  try {
    const parsed = JSON.parse(raw ?? "[]");
    return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === "string" && item.trim().length > 0) : [];
  } catch {
    return [];
  }
}

export function pushRecentWorkspace(
  current: string[],
  nextPath: string,
  limit = 6,
): string[] {
  const trimmed = nextPath.trim();
  if (!trimmed) return current;
  return [trimmed, ...current.filter((item) => item !== trimmed)].slice(0, limit);
}
```

- [ ] **Step 4: Run the helper tests to verify they pass**

Run: `bun test src/components/workspace-entry-state.test.ts`
Expected: PASS

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking issues remain for Task 1.

- [ ] **Step 6: Commit Task 1**

```bash
git add src/components/workspace-entry-state.ts src/components/workspace-entry-state.test.ts docs/superpowers/plans/2026-04-04-workspace-entry-overlay-mvp.md
git commit -m "feat: add workspace entry selection helpers"
```

- [ ] **Step 7: Update the plan commit log**

Update `## Commit Log` with the real commit hash and mark Task 1 as done before starting Task 2.

### Task 2: Blocking Workspace Entry Overlay

**Files:**
- Create: `src/components/AppBootstrapGate.tsx`
- Create: `src/components/WorkspaceEntryOverlay.tsx`
- Test: `src/components/WorkspaceEntryOverlay.test.tsx`
- Test: `tests/app-workspace-bootstrap.test.tsx`
- Modify: `src/App.tsx`
- Modify: `src/stores/task-store/types.ts`
- Modify: `src/stores/task-store/index.ts`
- Modify: `tests/task-store.test.ts`
- Modify: `src-tauri/src/commands_task.rs`
- Modify: `src-tauri/src/daemon/cmd.rs`
- Modify: `src-tauri/src/daemon/mod.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write the failing overlay render tests**

```tsx
test("renders title, chooser, and disabled continue state", () => {
  const html = renderToStaticMarkup(
    <WorkspaceEntryOverlay
      selected={null}
      recentWorkspaces={[]}
      onChooseFolder={() => {}}
      onSelectRecent={() => {}}
      onContinue={() => {}}
    />,
  );

  expect(html).toContain("Choose your workspace");
  expect(html).toContain("Continue");
  expect(html).toContain("disabled");
});

test("renders the selected candidate state", () => {
  const html = renderToStaticMarkup(
    <WorkspaceEntryOverlay
      selected={{ type: "recent", path: "/repo-a" }}
      recentWorkspaces={["/repo-a"]}
      onChooseFolder={() => {}}
      onSelectRecent={() => {}}
      onContinue={() => {}}
    />,
  );

  expect(html).toContain("/repo-a");
});
```

```ts
test("bootstrap clears active task before hydrating snapshot", async () => {
  const calls: string[] = [];
  await bootstrapTaskStore(
    set,
    async (cmd) => {
      calls.push(cmd);
      if (cmd === "daemon_get_task_snapshot") {
        return null;
      }
      return undefined as never;
    },
    async () => [],
  );

  expect(calls[0]).toBe("daemon_clear_active_task");
  expect(calls[1]).toBe("daemon_get_task_snapshot");
});

test("bootstrap marks completion only after clear and snapshot finish", async () => {
  const patches: Array<Record<string, unknown>> = [];
  const set = (updater: any) => {
    const patch = updater({
      activeTaskId: null,
      tasks: {},
      sessions: {},
      artifacts: {},
      providerHistory: {},
      providerHistoryLoading: {},
      providerHistoryError: {},
      bootstrapComplete: true,
      bootstrapError: null,
    });
    patches.push(patch);
  };

  await bootstrapTaskStore(set, async (cmd) => {
    if (cmd === "daemon_get_task_snapshot") {
      return null;
    }
    return undefined as never;
  }, async () => []);

  expect(patches[0]?.bootstrapComplete).toBe(false);
  expect(patches.at(-1)?.bootstrapComplete).toBe(true);
});

test("bootstrap failure blocks the shell and shows an error state", () => {
  const html = renderToStaticMarkup(
    <AppBootstrapGate
      status="error"
      message="Failed to clear active workspace session."
    />,
  );

  expect(html).toContain("Failed to clear active workspace session.");
  expect(html).not.toContain("Choose your workspace");
});
```

- [ ] **Step 2: Run the overlay test to verify it fails**

Run: `bun test src/components/WorkspaceEntryOverlay.test.tsx`
Expected: FAIL because the component does not exist yet.

Run: `bun test tests/task-store.test.ts -t "bootstrap clears active task before hydrating snapshot"`
Expected: FAIL because bootstrap does not clear active task yet.

- [ ] **Step 3: Implement the overlay and gate `App.tsx`**

```tsx
if (!bootstrapComplete) {
  return <AppBootstrapGate status="loading" />;
}

if (bootstrapError) {
  return <AppBootstrapGate status="error" message={bootstrapError} />;
}

return (
  <>
    {!activeTask && (
      <WorkspaceEntryOverlay
        selected={selectedWorkspace}
        recentWorkspaces={recentWorkspaces}
        actionError={workspaceActionError}
        onChooseFolder={handleChooseWorkspace}
        onSelectRecent={setSelectedWorkspace}
        onContinue={handleContinueIntoWorkspace}
      />
    )}
  </>
);
```

```ts
export async function bootstrapTaskStore(...) {
  set(() => ({ bootstrapComplete: false, bootstrapError: null }));
  await invokeImpl("daemon_clear_active_task");
  const snap = await invokeImpl<TaskSnapshot | null>("daemon_get_task_snapshot");
  if (snap) {
    set(() => snapshotToPatch(snap));
  }
  set(() => ({ bootstrapComplete: true }));
}
```

Implementation notes:
- Make `App.tsx` the single owner of `recentWorkspaces` state.
- Add `workspaceActionError: string | null` to `App.tsx` as the shared failure state for entry and switching actions.
- Reuse the existing `useCodexAccountStore().pickDirectory` capability from `App.tsx`, then pass the chosen path down through shell-level callbacks instead of letting panels call it directly.
- Hydrate that state once from `localStorage` after bootstrap succeeds.
- Pass the same `recentWorkspaces` array and selection callbacks to both `WorkspaceEntryOverlay` and `WorkspaceSwitcher`.
- Keep the overlay blocking and centered.
- Reuse the helper module for selection state and recent history.
- Do not auto-enter after folder pick; require an explicit `Continue`.
- Add a short code comment in `App.tsx` explaining that boot starts without an active task because the daemon does not persist active task selection across launches.
- Hydrate recent workspaces from `localStorage.getItem("dimweave:recent-workspaces")`.
- Treat invalid or corrupted stored data as `[]`.
- Persist recent workspaces as `JSON.stringify(string[])`.
- Write back to `dimweave:recent-workspaces` only after `Continue` succeeds.
- Add a minimal daemon command that sets `active_task_id` to `None` without deleting task history.
- On task creation failure, keep the overlay open, preserve the selected candidate, and render an inline error.
- Thread `workspaceActionError` into `WorkspaceEntryOverlay` so entry failure rendering is explicit.
- Keep the app in a non-interactive bootstrap phase until `daemon_clear_active_task` and `daemon_get_task_snapshot` have both completed.
- If either `daemon_clear_active_task` or `daemon_get_task_snapshot` fails, set `bootstrapError`, keep `bootstrapComplete` false, and render a blocking bootstrap error state instead of the workspace overlay.
- Ensure no workspace-dependent shell controls are interactive until bootstrap completes.

- [ ] **Step 4: Run targeted tests and smoke verification**

Run: `bun test src/components/WorkspaceEntryOverlay.test.tsx src/components/workspace-entry-state.test.ts tests/task-store.test.ts tests/app-workspace-bootstrap.test.tsx`
Expected: PASS

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking issues remain for Task 2.

- [ ] **Step 6: Commit Task 2**

```bash
git add src/App.tsx src/stores/task-store/types.ts src/stores/task-store/index.ts tests/task-store.test.ts tests/app-workspace-bootstrap.test.tsx src/components/AppBootstrapGate.tsx src/components/WorkspaceEntryOverlay.tsx src/components/WorkspaceEntryOverlay.test.tsx src-tauri/src/commands_task.rs src-tauri/src/daemon/cmd.rs src-tauri/src/daemon/mod.rs src-tauri/src/main.rs docs/superpowers/plans/2026-04-04-workspace-entry-overlay-mvp.md
git commit -m "feat: add blocking workspace entry overlay"
```

- [ ] **Step 7: Update the plan commit log**

Update `## Commit Log` with the real commit hash and mark Task 2 as done before starting Task 3.

### Task 3: Top-Right Workspace Switcher and Task Start Flow

**Files:**
- Create: `src/components/WorkspaceSwitcher.tsx`
- Test: `src/components/WorkspaceSwitcher.test.tsx`
- Modify: `src/components/ShellTopBar.tsx`
- Modify: `src/components/ShellTopBar.test.tsx`
- Modify: `src/components/shell-layout-state.ts`
- Modify: `tests/shell-layout-state.test.ts`
- Modify: `src/stores/task-store/types.ts`
- Modify: `src/stores/task-store/index.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: Write the failing switcher and task-start tests**

```tsx
test("shows choose workspace label when no active task exists", () => {
  const html = renderToStaticMarkup(
    <WorkspaceSwitcher
      workspaceLabel="No workspace selected"
      selected={null}
      recentWorkspaces={["/repo-a"]}
      onChooseFolder={() => {}}
      onSelectRecent={() => {}}
      onContinue={() => {}}
    />,
  );

  expect(html).toContain("Choose workspace");
  expect(html).toContain("Recent workspaces");
});
```

- [ ] **Step 2: Run the switcher tests to verify they fail**

Run: `bun test src/components/WorkspaceSwitcher.test.tsx src/components/ShellTopBar.test.tsx`
Expected: FAIL because the switcher does not exist yet.

- [ ] **Step 3: Implement the compact switcher and shared task-start action**

```ts
startWorkspaceTask: async (workspace: string) => {
  const title = deriveWorkspaceTaskTitle(workspace);
  const task = await invoke<TaskInfo>("daemon_create_task", { workspace, title });
  set((s) => ({ tasks: { ...s.tasks, [task.taskId]: task }, activeTaskId: task.taskId }));
  return task;
}
```

Implementation notes:
- Reuse a dedicated `startWorkspaceTask` action instead of calling `createTask` ad hoc from multiple components.
- Reuse a small helper such as `deriveWorkspaceTaskTitle(workspace)` instead of inline path splitting so title derivation stays consistent.
- `daemon_create_task` already creates and selects the task in the daemon.
- Mirror that timing in Zustand by setting `activeTaskId` as soon as the command resolves.
- Keep the top-right control compact; do not reopen the full-screen overlay after entry.
- Reuse the same single-selection and recent-history helpers as the entry overlay.
- Mirror the overlay callback surface so folder pick and recent selection go through the same candidate state transitions.
- Pass `workspaceActionError` into `WorkspaceSwitcher` so switch failures render inline without inventing a separate state path.
- On successful switching, append the chosen workspace to recent history.
- Treat selecting the currently active workspace from the switcher as a no-op.
- Update `resolveShellWorkspaceLabel` so it only reflects `activeTask.workspaceRoot`; if there is no active task, it must return `No workspace selected` even when provider sessions are connected.

- [ ] **Step 4: Run targeted tests and build verification**

Run: `bun test src/components/WorkspaceSwitcher.test.tsx src/components/ShellTopBar.test.tsx src/components/WorkspaceEntryOverlay.test.tsx`
Expected: PASS

Run: `npm run build`
Expected: success

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking issues remain for Task 3.

- [ ] **Step 6: Commit Task 3**

```bash
git add src/App.tsx src/components/ShellTopBar.tsx src/components/ShellTopBar.test.tsx src/components/WorkspaceSwitcher.tsx src/components/WorkspaceSwitcher.test.tsx src/components/shell-layout-state.ts tests/shell-layout-state.test.ts src/stores/task-store/types.ts src/stores/task-store/index.ts docs/superpowers/plans/2026-04-04-workspace-entry-overlay-mvp.md
git commit -m "feat: route workspace changes through shell top bar"
```

- [ ] **Step 7: Update the plan commit log**

Update `## Commit Log` with the real commit hash and mark Task 3 as done before starting Task 4.

### Task 4: Remove Per-Agent Workspace Pickers

**Files:**
- Modify: `src/components/ClaudePanel/index.tsx`
- Modify: `src/components/ClaudePanel/ClaudeConfigRows.tsx`
- Modify: `src/components/AgentStatus/CodexPanel.tsx`
- Modify: `src/components/AgentStatus/CodexConfigRows.tsx`
- Modify: `src/components/AgentStatus/provider-session-view-model.ts`
- Test: `tests/provider-session-view-model.test.ts`
- Test: `tests/agent-workspace-readonly.test.tsx`

- [ ] **Step 1: Write the failing regression tests**

```tsx
test("agent configuration panels no longer render Select project", () => {
  const html = renderToStaticMarkup(<ClaudeConfigRows cwd="/repo" disabled />);
  expect(html).not.toContain("Select project...");
  expect(html).toContain("/repo");
});

test("codex configuration rows show a read-only workspace label", () => {
  const html = renderToStaticMarkup(
    <CodexConfigRows
      locked
      profile={null}
      models={[]}
      selectedModel=""
      modelSelectOptions={[]}
      handleModelChange={() => {}}
      reasoningOptions={[]}
      selectedReasoning=""
      setSelectedReasoning={() => {}}
      reasoningSelectOptions={[]}
      cwd="/repo"
      handlePickDir={() => {}}
    />,
  );

  expect(html).not.toContain("Select project...");
  expect(html).toContain("/repo");
});

test("active task workspace wins over provider session cwd", () => {
  expect(
    resolveProviderHistoryWorkspace("/task-workspace", {
      provider: "codex",
      externalSessionId: "thread_123",
      cwd: "/provider-session-workspace",
      connectionMode: "new",
    }),
  ).toBe("/task-workspace");
});
```

- [ ] **Step 2: Run the regression tests to verify they fail**

Run: `bun test tests/agent-workspace-readonly.test.tsx tests/provider-session-view-model.test.ts`
Expected: FAIL until the old per-agent project picker UI is removed.

- [ ] **Step 3: Implement the panel cleanup**

```tsx
<div className="flex items-center justify-between">
  <span className="text-[10px] text-muted-foreground">Project</span>
  <span className="max-w-44 truncate font-mono text-[11px] text-secondary-foreground">
    {cwd ? shortenPath(cwd) : "Workspace required"}
  </span>
</div>
```

Implementation notes:
- Delete local `cwd` state and `pickDirectory` usage from both provider panels.
- Read `activeTask.workspaceRoot` in both provider panels and make it the only source for `effectiveCwd`.
- Update `resolveProviderHistoryWorkspace` to accept active-task workspace input and stop consulting `providerSession.cwd`.
- Keep provider history loading keyed off the active task workspace only.
- Do not fall back to `providerSession.cwd` once the shell has entered a task.
- Keep connect buttons disabled when no active workspace exists.

- [ ] **Step 4: Run full frontend verification**

Run: `bun test`
Expected: PASS

Run: `npm run build`
Expected: success

- [ ] **Step 5: Run deep review and fix findings**

Run the task through `superpowers:requesting-code-review`.
Expected: no blocking issues remain for Task 4.

- [ ] **Step 6: Commit Task 4**

```bash
git add src/components/ClaudePanel/index.tsx src/components/ClaudePanel/ClaudeConfigRows.tsx src/components/AgentStatus/CodexPanel.tsx src/components/AgentStatus/CodexConfigRows.tsx src/components/AgentStatus/provider-session-view-model.ts tests/provider-session-view-model.test.ts tests/agent-workspace-readonly.test.tsx docs/superpowers/plans/2026-04-04-workspace-entry-overlay-mvp.md
git commit -m "refactor: make agent workspace read-only"
```

- [ ] **Step 7: Update the plan commit log**

Update `## Commit Log` with the real commit hash and mark Task 4 as done.

## Final Verification

- [x] Run: `bun test`
- [x] Run: `npm run build`
- [ ] Confirm the entry overlay appears when there is no active task.
- [x] Confirm selecting either a picked folder or a recent workspace enables `Continue`.
- [x] Confirm top-right workspace switching creates a fresh task context.
- [x] Confirm Claude/Codex advanced panels no longer expose independent project pickers.
