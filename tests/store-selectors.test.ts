import { describe, expect, test } from "bun:test";
import {
  selectAnyAgentConnected,
  selectPermissionPromptCount,
  selectTerminalErrorCount,
} from "../src/stores/bridge-store/selectors";
import {
  selectActiveTask,
  selectActiveTaskArtifactCount,
  selectActiveTaskSessionCount,
  selectActiveTaskArtifacts,
  selectActiveTaskSessions,
  makeProviderHistoryErrorSelector,
  makeProviderHistoryLoadingSelector,
  makeProviderHistorySelector,
} from "../src/stores/task-store/selectors";

describe("selectAnyAgentConnected", () => {
  test("returns true when either Claude or Codex is connected", () => {
    expect(
      selectAnyAgentConnected({
        agents: {
          claude: { status: "disconnected" },
          codex: { status: "connected" },
        },
      } as any),
    ).toBe(true);
  });

  test("returns false when both primary agents are offline", () => {
    expect(
      selectAnyAgentConnected({
        agents: {
          claude: { status: "disconnected" },
          codex: { status: "disconnected" },
        },
      } as any),
    ).toBe(false);
  });

  test("returns false when the agents map is missing", () => {
    expect(
      selectAnyAgentConnected({
        agents: undefined,
      } as any),
    ).toBe(false);
  });
});

describe("bridge selector counters", () => {
  test("counts pending permission prompts without exposing the prompt list", () => {
    expect(
      selectPermissionPromptCount({
        permissionPrompts: [{ requestId: "a" }, { requestId: "b" }],
      } as any),
    ).toBe(2);
  });

  test("counts only error terminal lines", () => {
    expect(
      selectTerminalErrorCount({
        terminalLines: [
          { kind: "text", line: "ok" },
          { kind: "error", line: "boom" },
          { kind: "error", line: "still bad" },
        ],
      } as any),
    ).toBe(2);
  });
});

describe("selectActiveTask", () => {
  test("returns the active task object without exposing the whole task map", () => {
    expect(
      selectActiveTask({
        activeTaskId: "task-1",
        tasks: {
          "task-1": { taskId: "task-1", title: "Active" },
          "task-2": { taskId: "task-2", title: "Other" },
        },
      } as any),
    ).toEqual({ taskId: "task-1", title: "Active" });
  });

  test("returns null when no active task is selected", () => {
    expect(
      selectActiveTask({
        activeTaskId: null,
        tasks: {},
      } as any),
    ).toBeNull();
  });
});

describe("active task counters", () => {
  test("returns session and artifact counts for the active task only", () => {
    const state = {
      activeTaskId: "task-1",
      sessions: {
        "task-1": [{ sessionId: "a" }, { sessionId: "b" }],
        "task-2": [{ sessionId: "c" }],
      },
      artifacts: {
        "task-1": [{ artifactId: "x" }],
        "task-2": [{ artifactId: "y" }, { artifactId: "z" }],
      },
    };

    expect(selectActiveTaskSessionCount(state as any)).toBe(2);
    expect(selectActiveTaskArtifactCount(state as any)).toBe(1);
  });

  test("returns zero counts when there is no active task", () => {
    const state = {
      activeTaskId: null,
      sessions: {},
      artifacts: {},
    };

    expect(selectActiveTaskSessionCount(state as any)).toBe(0);
    expect(selectActiveTaskArtifactCount(state as any)).toBe(0);
  });
});

describe("active task collection selectors", () => {
  test("reuses a stable empty sessions array when there is no active task", () => {
    const state = {
      activeTaskId: null,
      sessions: {},
    };

    expect(selectActiveTaskSessions(state as any)).toBe(
      selectActiveTaskSessions(state as any),
    );
  });

  test("reuses a stable empty artifacts array when there is no active task", () => {
    const state = {
      activeTaskId: null,
      artifacts: {},
    };

    expect(selectActiveTaskArtifacts(state as any)).toBe(
      selectActiveTaskArtifacts(state as any),
    );
  });

  test("reuses a stable empty sessions array when the active task has no session entry", () => {
    const state = {
      activeTaskId: "task-1",
      sessions: {},
    };

    expect(selectActiveTaskSessions(state as any)).toBe(
      selectActiveTaskSessions(state as any),
    );
  });

  test("reuses a stable empty artifacts array when the active task has no artifact entry", () => {
    const state = {
      activeTaskId: "task-1",
      artifacts: {},
    };

    expect(selectActiveTaskArtifacts(state as any)).toBe(
      selectActiveTaskArtifacts(state as any),
    );
  });
});

describe("provider history selectors", () => {
  test("reuse stable defaults when the requested workspace has no provider history entry", () => {
    const state = {
      providerHistory: {},
      providerHistoryLoading: {},
      providerHistoryError: {},
    };
    const selectHistory = makeProviderHistorySelector("/tmp/project");
    const selectLoading = makeProviderHistoryLoadingSelector("/tmp/project");
    const selectError = makeProviderHistoryErrorSelector("/tmp/project");

    expect(selectHistory(state as any)).toBe(selectHistory(state as any));
    expect(selectLoading(state as any)).toBe(false);
    expect(selectError(state as any)).toBeNull();
  });

  test("read only the requested workspace slice", () => {
    const selectHistory = makeProviderHistorySelector("/tmp/project-a");
    const selectLoading = makeProviderHistoryLoadingSelector("/tmp/project-a");
    const selectError = makeProviderHistoryErrorSelector("/tmp/project-a");
    const entry = { externalId: "claude-1" };
    const state = {
      providerHistory: {
        "/tmp/project-a": [entry],
        "/tmp/project-b": [{ externalId: "codex-2" }],
      },
      providerHistoryLoading: {
        "/tmp/project-a": true,
        "/tmp/project-b": false,
      },
      providerHistoryError: {
        "/tmp/project-a": "failed",
        "/tmp/project-b": null,
      },
    };

    expect(selectHistory(state as any)).toEqual([entry]);
    expect(selectLoading(state as any)).toBe(true);
    expect(selectError(state as any)).toBe("failed");
  });
});
