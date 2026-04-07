import { describe, expect, test } from "bun:test";
import {
  reduceTaskUpdated,
  reduceActiveTaskChanged,
  reduceSessionTreeChanged,
  reduceArtifactsChanged,
} from "../src/stores/task-store/events";
import {
  bootstrapTaskStore,
  createStartWorkspaceTaskAction,
  createFetchProviderHistoryAction,
  deriveWorkspaceTaskTitle,
  snapshotToPatch,
} from "../src/stores/task-store";
import type {
  ProviderHistoryInfo,
  TaskStoreData,
  TaskInfo,
  SessionInfo,
} from "../src/stores/task-store/types";

function emptyState(): TaskStoreData {
  return {
    activeTaskId: null,
    tasks: {},
    sessions: {},
    artifacts: {},
    providerHistory: {},
    providerHistoryLoading: {},
    providerHistoryError: {},
    bootstrapComplete: false,
    bootstrapError: null,
  };
}

function makeTask(id: string, title = "Test"): TaskInfo {
  return {
    taskId: id,
    workspaceRoot: "/ws",
    title,
    status: "draft",
    leadSessionId: null,
    currentCoderSessionId: null,
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
