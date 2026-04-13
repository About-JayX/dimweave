import { describe, expect, test } from "bun:test";
import { selectActiveTaskProviderBindings } from "../src/stores/task-store/selectors";
import type { TaskStoreState } from "../src/stores/task-store/types";

const NOOP = () => {};
const NOOP_ASYNC = async () => ({}) as any;
const STUB_ACTIONS = {
  createTask: NOOP_ASYNC, startWorkspaceTask: NOOP_ASYNC,
  selectTask: NOOP_ASYNC, setReplyTarget: NOOP, fetchSnapshot: NOOP_ASYNC,
  fetchProviderHistory: NOOP_ASYNC, resumeSession: NOOP_ASYNC,
  attachProviderHistory: NOOP_ASYNC, cleanup: NOOP,
} as unknown as Pick<TaskStoreState, "createTask" | "startWorkspaceTask" | "selectTask" | "setReplyTarget" | "fetchSnapshot" | "fetchProviderHistory" | "resumeSession" | "attachProviderHistory" | "cleanup">;

const EMPTY_DATA = {
  activeTaskId: null, tasks: {}, replyTargets: {}, sessions: {},
  artifacts: {}, providerSummaries: {}, providerHistory: {},
  providerHistoryLoading: {}, providerHistoryError: {},
  bootstrapComplete: false, bootstrapError: null, lastSave: null,
};

function makeState(o: Partial<TaskStoreState> = {}): TaskStoreState {
  return { ...EMPTY_DATA, ...STUB_ACTIONS, ...o } as TaskStoreState;
}

const TASK_T1 = {
  taskId: "t1", workspaceRoot: "/ws", title: "Task",
  status: "implementing" as const, leadProvider: "claude" as const,
  coderProvider: "codex" as const, createdAt: 0, updatedAt: 0,
};
const SUMMARY_T1 = {
  taskId: "t1", leadProvider: "claude", coderProvider: "codex",
  leadOnline: false, coderOnline: false,
};

describe("selectActiveTaskProviderBindings", () => {
  test("returns stable reference for unchanged state", () => {
    const s = makeState({
      activeTaskId: "t1", tasks: { t1: TASK_T1 },
      providerSummaries: { t1: { ...SUMMARY_T1, leadOnline: true } },
    });
    expect(selectActiveTaskProviderBindings(s)).toBe(selectActiveTaskProviderBindings(s));
  });

  test("returns DEFAULT_BINDINGS for no active task", () => {
    const s = makeState();
    const a = selectActiveTaskProviderBindings(s);
    expect(selectActiveTaskProviderBindings(s)).toBe(a);
    expect(a.leadOnline).toBe(false);
    expect(a.coderOnline).toBe(false);
  });

  test("updates when provider summary changes", () => {
    const s1 = makeState({
      activeTaskId: "t1", tasks: { t1: TASK_T1 },
      providerSummaries: { t1: SUMMARY_T1 },
    });
    const s2 = makeState({
      activeTaskId: "t1", tasks: { t1: TASK_T1 },
      providerSummaries: { t1: { ...SUMMARY_T1, leadOnline: true } },
    });
    const a = selectActiveTaskProviderBindings(s1);
    const b = selectActiveTaskProviderBindings(s2);
    expect(a).not.toBe(b);
    expect(a.leadOnline).toBe(false);
    expect(b.leadOnline).toBe(true);
  });
});
