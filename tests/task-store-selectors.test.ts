import { describe, expect, test } from "bun:test";
import { selectActiveTaskProviderBindings } from "../src/stores/task-store/selectors";
import type { TaskStoreState } from "../src/stores/task-store/types";

function makeState(overrides: Partial<TaskStoreState> = {}): TaskStoreState {
  return {
    activeTaskId: null,
    tasks: {},
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
    createTask: async () => ({}) as any,
    startWorkspaceTask: async () => ({}) as any,
    selectTask: async () => {},
    setReplyTarget: () => {},
    fetchSnapshot: async () => {},
    fetchProviderHistory: async () => {},
    resumeSession: async () => {},
    attachProviderHistory: async () => {},
    cleanup: () => {},
    ...overrides,
  };
}

describe("selectActiveTaskProviderBindings", () => {
  test("returns stable reference for unchanged state", () => {
    const state = makeState({
      activeTaskId: "t1",
      tasks: {
        t1: {
          taskId: "t1",
          workspaceRoot: "/ws",
          title: "Task",
          status: "implementing",
          leadProvider: "claude",
          coderProvider: "codex",
          createdAt: 0,
          updatedAt: 0,
        },
      },
      providerSummaries: {
        t1: {
          taskId: "t1",
          leadProvider: "claude",
          coderProvider: "codex",
          leadOnline: true,
          coderOnline: false,
        },
      },
    });

    const a = selectActiveTaskProviderBindings(state);
    const b = selectActiveTaskProviderBindings(state);
    expect(a).toBe(b); // strict reference equality
  });

  test("returns DEFAULT_BINDINGS for no active task", () => {
    const state = makeState();
    const a = selectActiveTaskProviderBindings(state);
    const b = selectActiveTaskProviderBindings(state);
    expect(a).toBe(b);
    expect(a.leadProvider).toBe("claude");
    expect(a.coderProvider).toBe("codex");
    expect(a.leadOnline).toBe(false);
    expect(a.coderOnline).toBe(false);
  });

  test("updates when provider summary changes", () => {
    const task = {
      taskId: "t1",
      workspaceRoot: "/ws",
      title: "Task",
      status: "implementing" as const,
      leadProvider: "claude" as const,
      coderProvider: "codex" as const,
      createdAt: 0,
      updatedAt: 0,
    };
    const summary1 = {
      taskId: "t1",
      leadProvider: "claude",
      coderProvider: "codex",
      leadOnline: false,
      coderOnline: false,
    };
    const summary2 = {
      ...summary1,
      leadOnline: true,
    };

    const state1 = makeState({
      activeTaskId: "t1",
      tasks: { t1: task },
      providerSummaries: { t1: summary1 },
    });
    const state2 = makeState({
      activeTaskId: "t1",
      tasks: { t1: task },
      providerSummaries: { t1: summary2 },
    });

    const a = selectActiveTaskProviderBindings(state1);
    const b = selectActiveTaskProviderBindings(state2);
    expect(a).not.toBe(b);
    expect(a.leadOnline).toBe(false);
    expect(b.leadOnline).toBe(true);
  });
});
