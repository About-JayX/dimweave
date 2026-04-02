import { describe, expect, test } from "bun:test";
import { selectAnyAgentConnected } from "../src/stores/bridge-store/selectors";
import { selectActiveTask } from "../src/stores/task-store/selectors";

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
