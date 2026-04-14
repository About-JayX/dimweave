import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { createElement } from "react";
import type { TaskAgentInfo, TaskStoreState } from "@/stores/task-store/types";

// ── Mock store with trackable actions ──────────────────────────
const addTaskAgent = mock(async () => ({
  agentId: "new", taskId: "t1", provider: "claude" as const,
  role: "", order: 0, createdAt: 1,
}));
const removeTaskAgent = mock(async () => {});
const updateTaskAgent = mock(async () => {});
const reorderTaskAgents = mock(async () => {});

function makeTask(id = "t1") {
  return {
    taskId: id, projectRoot: "/repo", taskWorktreeRoot: "/repo",
    workspaceRoot: "/repo", title: "Test",
    status: "draft" as const, leadProvider: "claude" as const,
    coderProvider: "codex" as const, createdAt: 1, updatedAt: 1,
  };
}
function makeAgent(o: Partial<TaskAgentInfo> & { agentId: string; role: string }): TaskAgentInfo {
  return { taskId: "t1", provider: "claude", displayName: null, order: 0, createdAt: 1, ...o };
}

let mockState: Partial<TaskStoreState> = {};

mock.module("@/stores/task-store", () => ({
  useTaskStore: (sel: (s: any) => any) => sel({
    ...mockState,
    addTaskAgent, removeTaskAgent, updateTaskAgent, reorderTaskAgents,
  }),
}));

import { setupDOM, render, queryAll, click, teardownDOM } from "./dom-test-env";

beforeEach(() => {
  setupDOM();
  addTaskAgent.mockClear();
  removeTaskAgent.mockClear();
  updateTaskAgent.mockClear();
  reorderTaskAgents.mockClear();
});
afterEach(() => teardownDOM());

describe("TaskAgentList interaction", () => {
  test("Remove button calls removeTaskAgent with correct agentId", async () => {
    const agents: TaskAgentInfo[] = [
      makeAgent({ agentId: "a1", role: "lead", order: 0 }),
      makeAgent({ agentId: "a2", provider: "codex", role: "coder", order: 1 }),
    ];
    mockState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: agents } };
    const { TaskAgentList } = await import("./TaskAgentList");
    await render(createElement(TaskAgentList));

    const removeBtns = queryAll('button[title="Remove"]');
    expect(removeBtns.length).toBe(2);
    click(removeBtns[0]);
    expect(removeTaskAgent).toHaveBeenCalledTimes(1);
    expect((removeTaskAgent.mock.calls as any[][])[0][0]).toBe("a1");
  });

  test("drag-drop reorder calls reorderTaskAgents", async () => {
    const agents: TaskAgentInfo[] = [
      makeAgent({ agentId: "a1", role: "lead", order: 0 }),
      makeAgent({ agentId: "a2", provider: "codex", role: "coder", order: 1 }),
      makeAgent({ agentId: "a3", provider: "codex", role: "reviewer", order: 2 }),
    ];
    mockState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: agents } };
    const { TaskAgentList } = await import("./TaskAgentList");
    await render(createElement(TaskAgentList));

    const rows = queryAll('[data-testid="agent-row"]');
    expect(rows.length).toBe(3);

    // Simulate drag: row[0] to row[2]
    // happy-dom DragEvent doesn't propagate dataTransfer via constructor,
    // so we set it manually on each event instance.
    const Win = globalThis.window as any;
    const dt = { effectAllowed: "", dropEffect: "" };
    const mkDrag = (type: string) => {
      const ev = new Win.Event(type, { bubbles: true });
      ev.dataTransfer = dt;
      ev.preventDefault = () => {};
      return ev;
    };
    rows[0].dispatchEvent(mkDrag("dragstart"));
    rows[2].dispatchEvent(mkDrag("dragover"));
    rows[2].dispatchEvent(mkDrag("drop"));

    expect(reorderTaskAgents).toHaveBeenCalledTimes(1);
    const calls = reorderTaskAgents.mock.calls as any[][];
    expect(calls[0][0]).toBe("t1");
    expect(calls[0][1]).toEqual(["a2", "a3", "a1"]);
  });

  test("Add button opens editor dialog", async () => {
    mockState = {
      activeTaskId: "t1",
      tasks: { t1: makeTask() },
      taskAgents: { t1: [] },
    };
    const { TaskAgentList } = await import("./TaskAgentList");
    await render(createElement(TaskAgentList));

    const addBtn = queryAll('[data-testid="add-agent-btn"]');
    expect(addBtn.length).toBe(1);
    click(addBtn[0]);
    // Re-render to pick up state change
    await new Promise((r) => setTimeout(r, 50));
    const dialogs = queryAll('[role="dialog"]');
    expect(dialogs.length).toBe(1);
  });
});
