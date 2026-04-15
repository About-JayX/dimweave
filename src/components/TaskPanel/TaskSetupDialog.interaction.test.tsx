import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { createElement } from "react";

// Mock AgentStatusPanel to avoid deep store dependencies
mock.module("@/components/AgentStatus", () => ({
  AgentStatusPanel: () => createElement("div", { "data-testid": "agent-status-mock" }),
}));

// dnd-kit mocks — capture onDragEnd for programmatic reorder testing
let capturedDragEnd: ((event: any) => void) | null = null;
mock.module("@dnd-kit/core", () => ({
  DndContext: ({ children, onDragEnd }: any) => {
    capturedDragEnd = onDragEnd;
    return createElement("div", { "data-dnd-context": "true" }, children);
  },
  useSensor: () => ({}),
  useSensors: (...args: any[]) => args,
  PointerSensor: class {},
  closestCenter: () => null,
}));
mock.module("@dnd-kit/sortable", () => ({
  SortableContext: ({ children }: any) =>
    createElement("div", { "data-sortable-context": "true" }, children),
  useSortable: () => ({
    attributes: {},
    listeners: {},
    setNodeRef: () => {},
    transform: null,
    transition: null,
    isDragging: false,
  }),
  verticalListSortingStrategy: {},
  arrayMove: (arr: any[], from: number, to: number) => {
    const r = [...arr]; const [item] = r.splice(from, 1); r.splice(to, 0, item); return r;
  },
}));
mock.module("@dnd-kit/utilities", () => ({
  CSS: { Transform: { toString: () => "" } },
}));

import { setupDOM, render, queryAll, click, teardownDOM } from "./dom-test-env";

beforeEach(() => { setupDOM(); capturedDragEnd = null; });
afterEach(() => teardownDOM());

describe("TaskSetupDialog interaction", () => {
  test("empty-task Create submit calls onSubmit with zero agents", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit,
        initialAgents: [],
      }),
    );
    const buttons = queryAll("button");
    const createBtn = buttons.find(
      (b) => b.textContent === "Create" && !(b as HTMLButtonElement).disabled,
    );
    expect(createBtn).toBeTruthy();
    click(createBtn!);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    expect(payload.agents).toEqual([]);
    expect(payload.requestLaunch).toBe(false);
  });

  test("Create & Connect is disabled when no agents", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit,
        initialAgents: [],
      }),
    );
    const connectBtn = queryAll("button").find(
      (b) => b.textContent?.includes("Connect"),
    ) as HTMLButtonElement | undefined;
    expect(connectBtn).toBeTruthy();
    expect(connectBtn!.disabled).toBe(true);
  });

  test("edit-mode Save submit preserves agentId in payload", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit",
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit,
        initialAgents: [
          { provider: "claude", role: "lead", agentId: "a1", displayName: "My Lead" },
          { provider: "codex", role: "coder", agentId: "a2" },
        ],
      }),
    );
    const saveBtn = queryAll("button").find((b) => b.textContent === "Save");
    expect(saveBtn).toBeTruthy();
    click(saveBtn!);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    expect(payload.agents.length).toBe(2);
    expect(payload.agents[0].agentId).toBe("a1");
    expect(payload.agents[0].displayName).toBe("My Lead");
    expect(payload.agents[1].agentId).toBe("a2");
  });

  test("edit-mode rows each have a drag handle affordance", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit",
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit: () => {},
        initialAgents: [
          { provider: "claude", role: "lead", agentId: "a1" },
          { provider: "codex", role: "coder", agentId: "a2" },
        ],
      }),
    );
    const handles = queryAll('[data-drag-handle="true"]');
    expect(handles.length).toBe(2);
  });

  test("edit-mode dnd-kit drag end reorders agents in submit payload", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit",
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit,
        initialAgents: [
          { provider: "claude", role: "lead", agentId: "a1" },
          { provider: "codex", role: "coder", agentId: "a2" },
        ],
      }),
    );

    // Trigger dnd-kit drag: move a1 (index 0) over a2 (index 1)
    expect(capturedDragEnd).toBeTruthy();
    capturedDragEnd!({ active: { id: "a1" }, over: { id: "a2" } });
    await new Promise((r) => setTimeout(r, 50));

    const saveBtn = queryAll("button").find((b) => b.textContent === "Save");
    click(saveBtn!);

    expect(onSubmit).toHaveBeenCalledTimes(1);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    expect(payload.agents[0].agentId).toBe("a2");
    expect(payload.agents[1].agentId).toBe("a1");
  });

  test("Cancel button calls onOpenChange(false)", async () => {
    const onOpenChange = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        workspace: "/repo",
        open: true,
        onOpenChange,
        onSubmit: () => {},
      }),
    );
    const cancelBtn = queryAll("button").find((b) => b.textContent === "Cancel");
    expect(cancelBtn).toBeTruthy();
    click(cancelBtn!);
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
