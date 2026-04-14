import { describe, expect, mock, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import type { TaskAgentInfo, TaskStoreState } from "@/stores/task-store/types";

let mockStoreState: Partial<TaskStoreState> = {};

mock.module("@/stores/task-store", () => ({
  useTaskStore: (selector: (s: any) => any) => selector(mockStoreState),
}));

Object.assign(globalThis, {
  document: { addEventListener: () => {}, removeEventListener: () => {} },
});

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

describe("computeDragReorder", () => {
  test("moves item forward", async () => {
    const { computeDragReorder } = await import("./TaskAgentList");
    expect(computeDragReorder(["a", "b", "c"], 0, 2)).toEqual(["b", "c", "a"]);
  });
  test("moves item backward", async () => {
    const { computeDragReorder } = await import("./TaskAgentList");
    expect(computeDragReorder(["a", "b", "c"], 2, 0)).toEqual(["c", "a", "b"]);
  });
  test("returns null when source equals target", async () => {
    const { computeDragReorder } = await import("./TaskAgentList");
    expect(computeDragReorder(["a", "b"], 1, 1)).toBeNull();
  });
  test("does not mutate original array", async () => {
    const { computeDragReorder } = await import("./TaskAgentList");
    const ids = ["a", "b", "c"];
    computeDragReorder(ids, 0, 2);
    expect(ids).toEqual(["a", "b", "c"]);
  });
  test("adjacent swap works", async () => {
    const { computeDragReorder } = await import("./TaskAgentList");
    expect(computeDragReorder(["a", "b", "c"], 0, 1)).toEqual(["b", "a", "c"]);
    expect(computeDragReorder(["a", "b", "c"], 1, 0)).toEqual(["b", "a", "c"]);
  });
});

describe("TaskAgentList", () => {
  test("renders null when no active task", async () => {
    mockStoreState = { activeTaskId: null, tasks: {}, taskAgents: {} };
    const { TaskAgentList } = await import("./TaskAgentList");
    const html = renderToStaticMarkup(<TaskAgentList />);
    expect(html).toBe("");
  });

  test("shows empty state when task has no agents", async () => {
    mockStoreState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: [] } };
    const { TaskAgentList } = await import("./TaskAgentList");
    const html = renderToStaticMarkup(<TaskAgentList />);
    expect(html).toContain("No agents configured");
    expect(html).toContain("Agents");
  });

  test("renders add-agent button", async () => {
    mockStoreState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: [] } };
    const { TaskAgentList } = await import("./TaskAgentList");
    const html = renderToStaticMarkup(<TaskAgentList />);
    expect(html).toContain("add-agent-btn");
    expect(html).toContain("Add");
  });

  test("renders agent rows with provider badge and role", async () => {
    const agents: TaskAgentInfo[] = [
      makeAgent({ agentId: "a1", provider: "claude", role: "lead", order: 0 }),
      makeAgent({ agentId: "a2", provider: "codex", role: "coder", order: 1 }),
    ];
    mockStoreState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: agents } };
    const { TaskAgentList } = await import("./TaskAgentList");
    const html = renderToStaticMarkup(<TaskAgentList />);
    expect(html).toContain("claude");
    expect(html).toContain("codex");
    expect(html).toContain("lead");
    expect(html).toContain("coder");
    expect((html.match(/agent-row/g) || []).length).toBe(2);
  });

  test("shows displayName when provided", async () => {
    const agents = [makeAgent({ agentId: "a1", role: "lead", displayName: "Claude Lead", order: 0 })];
    mockStoreState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: agents } };
    const { TaskAgentList } = await import("./TaskAgentList");
    const html = renderToStaticMarkup(<TaskAgentList />);
    expect(html).toContain("Claude Lead");
    expect(html).toContain("lead");
  });

  test("repeated roles render correctly", async () => {
    const agents: TaskAgentInfo[] = [
      makeAgent({ agentId: "a1", provider: "claude", role: "coder", order: 0 }),
      makeAgent({ agentId: "a2", provider: "codex", role: "coder", order: 1 }),
    ];
    mockStoreState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: agents } };
    const { TaskAgentList } = await import("./TaskAgentList");
    const html = renderToStaticMarkup(<TaskAgentList />);
    expect((html.match(/agent-row/g) || []).length).toBe(2);
  });

  test("rows have draggable attribute", async () => {
    const agents = [makeAgent({ agentId: "a1", role: "lead", order: 0 })];
    mockStoreState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: agents } };
    const { TaskAgentList } = await import("./TaskAgentList");
    const html = renderToStaticMarkup(<TaskAgentList />);
    expect(html).toContain("draggable");
    expect(html).toContain("cursor-grab");
  });

  test("edit and remove buttons render in each row", async () => {
    const agents = [makeAgent({ agentId: "a1", role: "lead", order: 0 })];
    mockStoreState = { activeTaskId: "t1", tasks: { t1: makeTask() }, taskAgents: { t1: agents } };
    const { TaskAgentList } = await import("./TaskAgentList");
    const html = renderToStaticMarkup(<TaskAgentList />);
    expect(html).toContain("Edit");
    expect(html).toContain("Remove");
  });
});
