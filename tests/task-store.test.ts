import { describe, expect, test } from "bun:test";
import {
  reduceTaskUpdated,
  reduceActiveTaskChanged,
  reduceReviewGateChanged,
  reduceSessionTreeChanged,
  reduceArtifactsChanged,
} from "../src/stores/task-store/events";
import {
  bootstrapTaskStore,
  snapshotToPatch,
} from "../src/stores/task-store";
import type {
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
  };
}

function makeTask(id: string, title = "Test"): TaskInfo {
  return {
    taskId: id,
    workspaceRoot: "/ws",
    title,
    status: "draft",
    reviewStatus: null,
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

describe("reduceReviewGateChanged", () => {
  test("updates review status on existing task", () => {
    const state = { ...emptyState(), tasks: { t1: makeTask("t1") } };
    const patch = reduceReviewGateChanged(state, {
      taskId: "t1",
      reviewStatus: "pending_lead_approval",
    });
    expect(patch.tasks?.["t1"]?.reviewStatus).toBe("pending_lead_approval");
  });

  test("no-ops for unknown task", () => {
    const patch = reduceReviewGateChanged(emptyState(), {
      taskId: "missing",
      reviewStatus: "in_review",
    });
    expect(patch).toEqual({});
  });

  test("clears review status to null", () => {
    const task = { ...makeTask("t1"), reviewStatus: "in_review" as const };
    const state = { ...emptyState(), tasks: { t1: task } };
    const patch = reduceReviewGateChanged(state, {
      taskId: "t1",
      reviewStatus: null,
    });
    expect(patch.tasks?.["t1"]?.reviewStatus).toBeNull();
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
    expect(patch.sessions?.t1?.[0]?.sessionId).toBe("s1");
    expect(patch.artifacts?.t1?.[0]?.artifactId).toBe("a1");
  });
});

describe("bootstrapTaskStore", () => {
  test("hydrates store from daemon snapshot before listening", async () => {
    const task = makeTask("t1", "Hydrated");
    const session = makeSession("s1", "t1");
    const calls: Partial<TaskStoreData>[] = [];

    await bootstrapTaskStore(
      (fn) => {
        calls.push(fn(emptyState()));
      },
      async (cmd) => {
        expect(cmd).toBe("daemon_get_task_snapshot");
        return {
          task,
          sessions: [session],
          artifacts: [],
        };
      },
      async () => [() => {}],
    );

    expect(calls[0]?.activeTaskId).toBe("t1");
    expect(calls[0]?.tasks?.t1?.title).toBe("Hydrated");
    expect(calls[0]?.sessions?.t1?.[0]?.sessionId).toBe("s1");
  });
});
