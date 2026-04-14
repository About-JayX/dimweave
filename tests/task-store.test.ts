import { describe, expect, test } from "bun:test";
import {
  reduceTaskUpdated,
  reduceActiveTaskChanged,
  reduceSessionTreeChanged,
  reduceArtifactsChanged,
  reduceTaskAgentsChanged,
} from "../src/stores/task-store/events";
import {
  bootstrapTaskStore,
  createStartWorkspaceTaskAction,
  createFetchProviderHistoryAction,
  createLoadWorkspaceTasksAction,
  createConfiguredTaskAction,
  createUpdateTaskConfigAction,
  deriveWorkspaceTaskTitle,
  setReplyTargetPatch,
  snapshotToPatch,
} from "../src/stores/task-store";
import {
  selectActiveReplyTarget,
  selectActiveTaskAgents,
  selectActiveTaskRoleOptions,
  selectDefaultReplyTarget,
} from "../src/stores/task-store/selectors";
import type {
  ProviderHistoryInfo,
  TaskAgentInfo,
  TaskStoreData,
  TaskInfo,
  SessionInfo,
  TaskStoreState,
} from "../src/stores/task-store/types";

function emptyState(): TaskStoreData {
  return {
    activeTaskId: null,
    selectedWorkspace: null,
    tasks: {},
    taskAgents: {},
    replyTargets: {},
    sessions: {},
    artifacts: {},
    providerSummaries: {},
    providerHistory: {},
    providerHistoryLoading: {},
    providerHistoryError: {},
    bootstrapComplete: false,
    bootstrapError: null,
    lastSave: null,
  };
}

function makeAgent(
  agentId: string,
  taskId: string,
  provider: "claude" | "codex",
  role: string,
  order = 0,
): TaskAgentInfo {
  return { agentId, taskId, provider, role, order, createdAt: 100 };
}

function makeTask(id: string, title = "Test"): TaskInfo {
  return {
    taskId: id,
    workspaceRoot: "/ws",
    title,
    status: "draft",
    leadSessionId: null,
    currentCoderSessionId: null,
    leadProvider: "claude",
    coderProvider: "codex",
    createdAt: 100,
    updatedAt: 200,
  };
}

function makeSession(id: string, taskId: string): SessionInfo {
  return {
    sessionId: id,
    taskId,
    parentSessionId: null,
    provider: "claude",
    role: "lead",
    externalSessionId: null,
    status: "active",
    cwd: "/ws",
    title: "Lead",
    createdAt: 100,
    updatedAt: 200,
  };
}

describe("reduceTaskUpdated", () => {
  test("inserts new task into empty store", () => {
    const task = makeTask("t1", "First");
    const patch = reduceTaskUpdated(emptyState(), task);
    expect(patch.tasks?.["t1"]?.title).toBe("First");
  });

  test("overwrites existing task", () => {
    const state = { ...emptyState(), tasks: { t1: makeTask("t1", "Old") } };
    const updated = { ...makeTask("t1", "New"), status: "planning" as const };
    const patch = reduceTaskUpdated(state, updated);
    expect(patch.tasks?.["t1"]?.title).toBe("New");
    expect(patch.tasks?.["t1"]?.status).toBe("planning");
  });

  test("preserves other tasks", () => {
    const state = {
      ...emptyState(),
      tasks: { t1: makeTask("t1"), t2: makeTask("t2") },
    };
    const patch = reduceTaskUpdated(state, {
      ...makeTask("t1", "Updated"),
      status: "done" as const,
    });
    expect(patch.tasks?.["t2"]?.title).toBe("Test");
  });
});

describe("reduceActiveTaskChanged", () => {
  test("sets activeTaskId from null", () => {
    const patch = reduceActiveTaskChanged(emptyState(), { taskId: "t1" });
    expect(patch.activeTaskId).toBe("t1");
  });

  test("clears activeTaskId", () => {
    const state = { ...emptyState(), activeTaskId: "t1" };
    const patch = reduceActiveTaskChanged(state, { taskId: null });
    expect(patch.activeTaskId).toBeNull();
  });
});

describe("reduceSessionTreeChanged", () => {
  test("sets sessions for a task", () => {
    const sessions = [makeSession("s1", "t1"), makeSession("s2", "t1")];
    const patch = reduceSessionTreeChanged(emptyState(), {
      taskId: "t1",
      sessions,
    });
    expect(patch.sessions?.["t1"]?.length).toBe(2);
  });

  test("replaces existing sessions", () => {
    const state = {
      ...emptyState(),
      sessions: { t1: [makeSession("old", "t1")] },
    };
    const patch = reduceSessionTreeChanged(state, {
      taskId: "t1",
      sessions: [makeSession("new", "t1")],
    });
    expect(patch.sessions?.["t1"]?.length).toBe(1);
    expect(patch.sessions?.["t1"]?.[0]?.sessionId).toBe("new");
  });

  test("preserves other tasks' sessions", () => {
    const state = {
      ...emptyState(),
      sessions: {
        t1: [makeSession("s1", "t1")],
        t2: [makeSession("s2", "t2")],
      },
    };
    const patch = reduceSessionTreeChanged(state, {
      taskId: "t1",
      sessions: [],
    });
    expect(patch.sessions?.["t2"]?.length).toBe(1);
  });
});

describe("reduceArtifactsChanged", () => {
  test("sets artifacts for a task", () => {
    const artifacts = [
      {
        artifactId: "a1",
        taskId: "t1",
        sessionId: "s1",
        kind: "diff" as const,
        title: "patch",
        contentRef: "ref",
        createdAt: 100,
      },
    ];
    const patch = reduceArtifactsChanged(emptyState(), {
      taskId: "t1",
      artifacts,
    });
    expect(patch.artifacts?.["t1"]?.length).toBe(1);
    expect(patch.artifacts?.["t1"]?.[0]?.kind).toBe("diff");
  });

  test("replaces existing artifacts", () => {
    const old = {
      artifactId: "old",
      taskId: "t1",
      sessionId: "s1",
      kind: "plan" as const,
      title: "old",
      contentRef: "ref",
      createdAt: 50,
    };
    const state = { ...emptyState(), artifacts: { t1: [old] } };
    const patch = reduceArtifactsChanged(state, {
      taskId: "t1",
      artifacts: [],
    });
    expect(patch.artifacts?.["t1"]?.length).toBe(0);
  });
});

describe("snapshotToPatch", () => {
  test("hydrates active task, sessions, and artifacts from snapshot", () => {
    const task = makeTask("t1", "Hydrated");
    const session = makeSession("s1", "t1");
    const artifact = {
      artifactId: "a1",
      taskId: "t1",
      sessionId: "s1",
      kind: "plan" as const,
      title: "plan",
      contentRef: "ref",
      createdAt: 300,
    };

    const patch = snapshotToPatch({
      task,
      sessions: [session],
      artifacts: [artifact],
    });

    expect(patch.activeTaskId).toBe("t1");
    expect(patch.tasks?.t1?.title).toBe("Hydrated");
    expect("reviewStatus" in (patch.tasks?.t1 ?? {})).toBe(false);
    expect(patch.sessions?.t1?.[0]?.sessionId).toBe("s1");
    expect(patch.artifacts?.t1?.[0]?.artifactId).toBe("a1");
  });
});

describe("setReplyTargetPatch", () => {
  test("stores a reply target for the active task only", () => {
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      replyTargets: { t2: "lead" as const },
    };

    const patch = setReplyTargetPatch(state, state.activeTaskId, "coder");

    expect(patch.replyTargets).toEqual({
      t2: "lead",
      t1: "coder",
    });
  });

  test("returns an empty patch when there is no active task", () => {
    expect(setReplyTargetPatch(emptyState(), null, "coder")).toEqual({});
  });
});

describe("bootstrapTaskStore", () => {
  test("hydrates store from daemon snapshot before listening", async () => {
    const task = makeTask("t1", "Hydrated");
    const session = makeSession("s1", "t1");
    const calls: Partial<TaskStoreData>[] = [];
    const commands: string[] = [];

    await bootstrapTaskStore(
      (fn) => {
        calls.push(fn(emptyState()));
      },
      async (cmd) => {
        commands.push(cmd);
        if (cmd === "daemon_clear_active_task") {
          return undefined as never;
        }
        return {
          task,
          sessions: [session],
          artifacts: [],
        };
      },
      async () => [() => {}],
    );

    const hydrationPatch = calls.find((patch) => patch.activeTaskId === "t1");
    expect(commands).toEqual([
      "daemon_clear_active_task",
      "daemon_get_task_snapshot",
    ]);
    expect(hydrationPatch?.tasks?.t1?.title).toBe("Hydrated");
    expect(hydrationPatch?.sessions?.t1?.[0]?.sessionId).toBe("s1");
  });

  test("sets selectedWorkspace from snapshot task workspace", async () => {
    const task = { ...makeTask("t1"), workspaceRoot: "/ws/repo" };
    const calls: Partial<TaskStoreData>[] = [];

    await bootstrapTaskStore(
      (fn) => { calls.push(fn(emptyState())); },
      async (cmd) => {
        if (cmd === "daemon_clear_active_task") return undefined as never;
        return { task, sessions: [], artifacts: [] };
      },
      async () => [() => {}],
    );

    const hydration = calls.find((p) => p.activeTaskId === "t1");
    expect(hydration?.selectedWorkspace).toBe("/ws/repo");
  });

  test("leaves selectedWorkspace null when no snapshot", async () => {
    const calls: Partial<TaskStoreData>[] = [];

    await bootstrapTaskStore(
      (fn) => { calls.push(fn(emptyState())); },
      async (cmd) => {
        if (cmd === "daemon_get_task_snapshot") return null;
        return undefined as never;
      },
      async () => [() => {}],
    );

    const wsPatches = calls.filter((p) => p.selectedWorkspace !== undefined);
    expect(wsPatches.every((p) => p.selectedWorkspace === null)).toBe(true);
  });

  test("clears active task before hydrating snapshot", async () => {
    const commands: string[] = [];

    await bootstrapTaskStore(
      () => {},
      async (cmd) => {
        commands.push(cmd);
        if (cmd === "daemon_get_task_snapshot") {
          return null;
        }
        return undefined as never;
      },
      async () => [() => {}],
    );

    expect(commands[0]).toBe("daemon_clear_active_task");
    expect(commands[1]).toBe("daemon_get_task_snapshot");
  });

  test("marks bootstrap complete only after clear and snapshot finish", async () => {
    const patches: Array<Record<string, unknown>> = [];
    const set = (fn: (state: TaskStoreData) => Record<string, unknown>) => {
      patches.push(fn(emptyState()));
    };

    await bootstrapTaskStore(
      set as any,
      async (cmd) => {
        if (cmd === "daemon_get_task_snapshot") {
          return null;
        }
        return undefined as never;
      },
      async () => [() => {}],
    );

    expect(patches[0]?.bootstrapComplete).toBe(false);
    expect(patches.at(-1)?.bootstrapComplete).toBe(true);
  });

  test("records a blocking bootstrap error when clear fails", async () => {
    const patches: Array<Record<string, unknown>> = [];
    const set = (fn: (state: TaskStoreData) => Record<string, unknown>) => {
      patches.push(fn(emptyState()));
    };

    await expect(
      bootstrapTaskStore(
        set as any,
        async () => {
          throw new Error("clear failed");
        },
        async () => [() => {}],
      ),
    ).rejects.toThrow("clear failed");

    expect(patches.at(-1)?.bootstrapError).toBe("clear failed");
  });
});

describe("createFetchProviderHistoryAction", () => {
  test("dedupes concurrent requests for the same workspace", async () => {
    const historyEntry: ProviderHistoryInfo = {
      provider: "claude",
      externalId: "session-1",
      title: "Claude session",
      preview: "preview",
      cwd: "/ws",
      archived: false,
      createdAt: 100,
      updatedAt: 200,
      status: "active",
      normalizedSessionId: null,
      normalizedTaskId: null,
    };
    let state = emptyState();
    let invokeCalls = 0;
    let resolveRequest: ((entries: ProviderHistoryInfo[]) => void) | null = null;
    const invokeImpl = () =>
      new Promise<ProviderHistoryInfo[]>((resolve) => {
        invokeCalls += 1;
        resolveRequest = resolve;
      });
    const set = (
      fn: (current: TaskStoreData) => Partial<TaskStoreData>,
    ) => {
      state = { ...state, ...fn(state) };
    };

    const fetchProviderHistory = createFetchProviderHistoryAction(set, invokeImpl);
    const first = fetchProviderHistory("/ws");
    const second = fetchProviderHistory("/ws");

    expect(invokeCalls).toBe(1);
    expect(state.providerHistoryLoading["/ws"]).toBe(true);

    resolveRequest?.([historyEntry]);
    await Promise.all([first, second]);

    expect(state.providerHistory["/ws"]).toEqual([historyEntry]);
    expect(state.providerHistoryLoading["/ws"]).toBe(false);
    expect(state.providerHistoryError["/ws"]).toBeNull();
  });

  test("does not dedupe requests across different workspaces", async () => {
    let state = emptyState();
    let invokeCalls = 0;
    const invokeImpl = async () => {
      invokeCalls += 1;
      return [];
    };
    const set = (
      fn: (current: TaskStoreData) => Partial<TaskStoreData>,
    ) => {
      state = { ...state, ...fn(state) };
    };

    const fetchProviderHistory = createFetchProviderHistoryAction(set, invokeImpl);
    await Promise.all([
      fetchProviderHistory("/ws-a"),
      fetchProviderHistory("/ws-b"),
    ]);

    expect(invokeCalls).toBe(2);
  });
});

describe("createLoadWorkspaceTasksAction", () => {
  test("merges tasks from daemon_list_tasks into store", async () => {
    const t1 = { ...makeTask("t1", "A"), workspaceRoot: "/ws" };
    const t2 = { ...makeTask("t2", "B"), workspaceRoot: "/ws" };
    let state = { ...emptyState(), tasks: { t0: makeTask("t0", "Existing") } };
    const set = (fn: (s: TaskStoreData) => Partial<TaskStoreData>) => {
      state = { ...state, ...fn(state) };
    };
    const load = createLoadWorkspaceTasksAction(set, async (_cmd, args) => {
      expect(args).toEqual({ workspace: "/ws" });
      return [t1, t2];
    });
    await load("/ws");
    expect(state.tasks.t0?.title).toBe("Existing");
    expect(state.tasks.t1?.title).toBe("A");
    expect(state.tasks.t2?.title).toBe("B");
  });
});

describe("deriveWorkspaceTaskTitle", () => {
  test("uses the last workspace path segment", () => {
    expect(deriveWorkspaceTaskTitle("/Users/jason/projects/dimweave")).toBe(
      "dimweave",
    );
  });
});

describe("createStartWorkspaceTaskAction", () => {
  test("sets the created task active immediately", async () => {
    const task = makeTask("t2", "repo-b");
    let state = emptyState();
    const set = (fn: (current: TaskStoreData) => Partial<TaskStoreData>) => {
      state = { ...state, ...fn(state) };
    };

    const startWorkspaceTask = createStartWorkspaceTaskAction(
      set,
      async (_cmd, args) => {
        expect(args).toEqual({ workspace: "/repo-b", title: "repo-b" });
        return { ...task, workspaceRoot: "/repo-b", title: "repo-b" };
      },
    );

    await startWorkspaceTask("/repo-b");

    expect(state.activeTaskId).toBe("t2");
    expect(state.tasks.t2?.workspaceRoot).toBe("/repo-b");
  });

  test("creates a fresh task context instead of overwriting the current task", async () => {
    const currentTask = makeTask("t1", "repo-a");
    const nextTask = { ...makeTask("t2", "repo-b"), workspaceRoot: "/repo-b" };
    let state: TaskStoreData = {
      ...emptyState(),
      activeTaskId: currentTask.taskId,
      tasks: { [currentTask.taskId]: currentTask },
    };
    const set = (fn: (current: TaskStoreData) => Partial<TaskStoreData>) => {
      state = { ...state, ...fn(state) };
    };

    const startWorkspaceTask = createStartWorkspaceTaskAction(
      set,
      async () => nextTask,
    );

    await startWorkspaceTask("/repo-b");

    expect(state.activeTaskId).toBe("t2");
    expect(state.tasks.t1?.workspaceRoot).toBe("/ws");
    expect(state.tasks.t2?.workspaceRoot).toBe("/repo-b");
  });
});

describe("createConfiguredTaskAction", () => {
  test("passes provider config to daemon and sets task active", async () => {
    const task = makeTask("t1", "Configured");
    let state = emptyState();
    const set = (fn: (s: TaskStoreData) => Partial<TaskStoreData>) => {
      state = { ...state, ...fn(state) };
    };
    const create = createConfiguredTaskAction(set, async (_cmd, args) => {
      expect(args).toEqual({
        workspace: "/ws",
        title: "repo",
        leadProvider: "codex",
        coderProvider: "claude",
      });
      return task;
    });
    await create("/ws", "repo", { leadProvider: "codex", coderProvider: "claude" });
    expect(state.activeTaskId).toBe("t1");
    expect(state.tasks.t1?.title).toBe("Configured");
  });
});

describe("createUpdateTaskConfigAction", () => {
  test("updates provider bindings for a task", async () => {
    const updated = { ...makeTask("t1"), leadProvider: "codex" as const };
    let state = { ...emptyState(), activeTaskId: "t1", tasks: { t1: makeTask("t1") } };
    const set = (fn: (s: TaskStoreData) => Partial<TaskStoreData>) => {
      state = { ...state, ...fn(state) };
    };
    const update = createUpdateTaskConfigAction(set, async (_cmd, args) => {
      expect(args).toEqual({ taskId: "t1", leadProvider: "codex", coderProvider: "claude" });
      return updated;
    });
    await update("t1", { leadProvider: "codex", coderProvider: "claude" });
    expect(state.tasks.t1?.leadProvider).toBe("codex");
  });
});

describe("regression: no-task state stability", () => {
  test("selectedWorkspace set with activeTaskId null is a valid resting state", () => {
    const state: TaskStoreData = {
      ...emptyState(),
      selectedWorkspace: "/ws/project",
      activeTaskId: null,
      bootstrapComplete: true,
    };
    // This combination must be representable: workspace chosen, no task created yet.
    expect(state.selectedWorkspace).toBe("/ws/project");
    expect(state.activeTaskId).toBeNull();
    expect(state.bootstrapComplete).toBe(true);
  });

  test("workspace task list can be populated without forcing an active task", () => {
    const t1 = makeTask("t1");
    const t2 = makeTask("t2");
    const state: TaskStoreData = {
      ...emptyState(),
      selectedWorkspace: "/ws",
      activeTaskId: null,
      tasks: { t1, t2 },
    };
    expect(Object.keys(state.tasks)).toHaveLength(2);
    expect(state.activeTaskId).toBeNull();
  });

  test("no-task state with empty taskAgents is stable", () => {
    const state: TaskStoreData = {
      ...emptyState(),
      selectedWorkspace: "/ws",
      activeTaskId: null,
      taskAgents: {},
    };
    expect(state.taskAgents).toEqual({});
    expect(state.activeTaskId).toBeNull();
  });
});

describe("task-agent snapshot hydration", () => {
  test("snapshotToPatch hydrates taskAgents from snapshot", () => {
    const task = makeTask("t1");
    const agents = [
      makeAgent("a1", "t1", "claude", "lead", 0),
      makeAgent("a2", "t1", "codex", "coder", 1),
    ];
    const patch = snapshotToPatch({
      task,
      sessions: [],
      artifacts: [],
      taskAgents: agents,
    });
    expect(patch.taskAgents?.t1).toHaveLength(2);
    expect(patch.taskAgents?.t1?.[0]?.agentId).toBe("a1");
    expect(patch.taskAgents?.t1?.[1]?.role).toBe("coder");
  });

  test("snapshotToPatch with empty taskAgents produces empty array", () => {
    const task = makeTask("t1");
    const patch = snapshotToPatch({
      task,
      sessions: [],
      artifacts: [],
      taskAgents: [],
    });
    expect(patch.taskAgents?.t1).toEqual([]);
  });

  test("snapshotToPatch without taskAgents defaults to empty", () => {
    const task = makeTask("t1");
    const patch = snapshotToPatch({ task, sessions: [], artifacts: [] });
    expect(patch.taskAgents?.t1).toEqual([]);
  });
});

describe("selectActiveTaskAgents", () => {
  test("returns agents for the active task", () => {
    const agents = [makeAgent("a1", "t1", "claude", "lead")];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
    } as unknown as TaskStoreState;
    expect(selectActiveTaskAgents(state)).toEqual(agents);
  });

  test("returns empty array when no active task", () => {
    const state = { ...emptyState() } as unknown as TaskStoreState;
    expect(selectActiveTaskAgents(state)).toEqual([]);
  });
});

describe("selectActiveTaskRoleOptions", () => {
  test("returns unique roles sorted by order", () => {
    const agents = [
      makeAgent("a1", "t1", "claude", "lead", 0),
      makeAgent("a2", "t1", "codex", "coder", 1),
    ];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
    } as unknown as TaskStoreState;
    expect(selectActiveTaskRoleOptions(state)).toEqual(["auto", "lead", "coder"]);
  });

  test("deduplicates same-role agents", () => {
    const agents = [
      makeAgent("a1", "t1", "claude", "coder", 0),
      makeAgent("a2", "t1", "codex", "coder", 1),
    ];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
    } as unknown as TaskStoreState;
    expect(selectActiveTaskRoleOptions(state)).toEqual(["auto", "coder"]);
  });

  test("returns only auto when no agents", () => {
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: [] },
    } as unknown as TaskStoreState;
    expect(selectActiveTaskRoleOptions(state)).toEqual(["auto"]);
  });

  test("returns only auto when no active task", () => {
    const state = { ...emptyState() } as unknown as TaskStoreState;
    expect(selectActiveTaskRoleOptions(state)).toEqual(["auto"]);
  });
});

describe("selectDefaultReplyTarget", () => {
  test("defaults to lead when lead role is present", () => {
    const agents = [
      makeAgent("a1", "t1", "claude", "lead", 0),
      makeAgent("a2", "t1", "codex", "coder", 1),
    ];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
    } as unknown as TaskStoreState;
    expect(selectDefaultReplyTarget(state)).toBe("lead");
  });

  test("defaults to first ordered role when no lead", () => {
    const agents = [
      makeAgent("a1", "t1", "codex", "coder", 0),
      makeAgent("a2", "t1", "claude", "reviewer", 1),
    ];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
    } as unknown as TaskStoreState;
    expect(selectDefaultReplyTarget(state)).toBe("coder");
  });

  test("defaults to auto when no agents", () => {
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: [] },
    } as unknown as TaskStoreState;
    expect(selectDefaultReplyTarget(state)).toBe("auto");
  });

  test("defaults to auto when no active task", () => {
    const state = { ...emptyState() } as unknown as TaskStoreState;
    expect(selectDefaultReplyTarget(state)).toBe("auto");
  });
});

describe("selectActiveReplyTarget (live default-target rule)", () => {
  test("uses lead as default when no stored target and lead agent exists", () => {
    const agents = [
      makeAgent("a1", "t1", "claude", "lead", 0),
      makeAgent("a2", "t1", "codex", "coder", 1),
    ];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
    } as unknown as TaskStoreState;
    expect(selectActiveReplyTarget(state)).toBe("lead");
  });

  test("uses first ordered role as default when no lead and no stored target", () => {
    const agents = [
      makeAgent("a1", "t1", "codex", "reviewer", 0),
      makeAgent("a2", "t1", "claude", "coder", 1),
    ];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
    } as unknown as TaskStoreState;
    expect(selectActiveReplyTarget(state)).toBe("reviewer");
  });

  test("returns auto when no agents and no stored target", () => {
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: [] },
    } as unknown as TaskStoreState;
    expect(selectActiveReplyTarget(state)).toBe("auto");
  });

  test("returns stored target when explicitly set", () => {
    const agents = [
      makeAgent("a1", "t1", "claude", "lead", 0),
      makeAgent("a2", "t1", "codex", "coder", 1),
    ];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
      replyTargets: { t1: "coder" },
    } as unknown as TaskStoreState;
    expect(selectActiveReplyTarget(state)).toBe("coder");
  });

  test("returns auto when no active task", () => {
    const state = { ...emptyState() } as unknown as TaskStoreState;
    expect(selectActiveReplyTarget(state)).toBe("auto");
  });

  test("supports extensible role names", () => {
    const agents = [
      makeAgent("a1", "t1", "claude", "architect", 0),
    ];
    const state = {
      ...emptyState(),
      activeTaskId: "t1",
      tasks: { t1: makeTask("t1") },
      taskAgents: { t1: agents },
    } as unknown as TaskStoreState;
    expect(selectActiveReplyTarget(state)).toBe("architect");
  });
});

// ── reduceTaskAgentsChanged ────────────────────────────────

describe("reduceTaskAgentsChanged", () => {
  test("inserts agents for a task", () => {
    const agents = [
      makeAgent("a1", "t1", "claude", "lead", 0),
      makeAgent("a2", "t1", "codex", "coder", 1),
    ];
    const patch = reduceTaskAgentsChanged(emptyState(), {
      taskId: "t1",
      agents,
    });
    expect(patch.taskAgents?.["t1"]).toHaveLength(2);
    expect(patch.taskAgents?.["t1"]?.[0].role).toBe("lead");
  });

  test("replaces existing agents for same task", () => {
    const state = {
      ...emptyState(),
      taskAgents: {
        t1: [makeAgent("old", "t1", "claude", "lead", 0)],
      },
    };
    const newAgents = [makeAgent("new", "t1", "codex", "architect", 0)];
    const patch = reduceTaskAgentsChanged(state, {
      taskId: "t1",
      agents: newAgents,
    });
    expect(patch.taskAgents?.["t1"]).toHaveLength(1);
    expect(patch.taskAgents?.["t1"]?.[0].agentId).toBe("new");
  });

  test("preserves agents for other tasks", () => {
    const state = {
      ...emptyState(),
      taskAgents: {
        t1: [makeAgent("a1", "t1", "claude", "lead", 0)],
        t2: [makeAgent("a2", "t2", "codex", "coder", 0)],
      },
    };
    const patch = reduceTaskAgentsChanged(state, {
      taskId: "t1",
      agents: [],
    });
    expect(patch.taskAgents?.["t1"]).toHaveLength(0);
    expect(patch.taskAgents?.["t2"]).toHaveLength(1);
  });
});
