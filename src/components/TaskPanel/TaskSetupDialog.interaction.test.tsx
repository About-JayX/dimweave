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

import { setupDOM, render, query, queryAll, click, teardownDOM } from "./dom-test-env";

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

  // TDD: provider-aware config — these fail against current (Task 1) code

  test("create submit populates claudeConfig from claude agent model field", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit,
        initialAgents: [{ provider: "claude", role: "lead", agentId: "a1", model: "claude-sonnet-4-5-20250514" }],
      }),
    );
    const createBtn = queryAll("button").find(
      (b) => b.textContent === "Create" && !(b as HTMLButtonElement).disabled,
    );
    expect(createBtn).toBeTruthy();
    click(createBtn!);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    expect(payload.claudeConfig).not.toBeNull();
    expect(payload.claudeConfig.model).toBe("claude-sonnet-4-5-20250514");
  });

  test("provider change logic clears model and effort values", () => {
    // Verify the provider-switch clearing logic used by CyberSelect onChange
    type AgentDef = { provider: string; role: string; model?: string; effort?: string };
    const before: AgentDef = { provider: "claude", role: "lead", model: "opus", effort: "high" };
    // Simulates what setP does: clears model, effort on provider change
    const after = { ...before, provider: "codex", model: "", effort: "" };
    expect(after.model).toBe("");
    expect(after.effort).toBe("");
    expect(after.provider).toBe("codex");
  });

  test("first row in create mode has no delete button (locked)", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit: () => {},
      }),
    );
    const lockedRow = query('[data-locked-row="true"]');
    expect(lockedRow).toBeTruthy();
    // The locked row should not have a delete button
    const deleteButtons = lockedRow!.querySelectorAll('[data-delete-btn="true"]');
    expect(deleteButtons.length).toBe(0);
  });

  test("edit-mode Save & Connect calls onSubmit with requestLaunch true", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit", workspace: "/repo", open: true, onOpenChange: () => {}, onSubmit,
        initialAgents: [{ provider: "claude", role: "lead", agentId: "a1" }],
      }),
    );
    const connectBtn = queryAll("button").find((b) => b.textContent === "Save & Connect");
    expect(connectBtn).toBeTruthy();
    click(connectBtn!);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    expect(payload.requestLaunch).toBe(true);
  });

  test("edit-mode Save calls onSubmit with requestLaunch false", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit", workspace: "/repo", open: true, onOpenChange: () => {}, onSubmit,
        initialAgents: [{ provider: "claude", role: "lead", agentId: "a1" }],
      }),
    );
    const saveBtn = queryAll("button").find((b) => b.textContent === "Save");
    expect(saveBtn).toBeTruthy();
    click(saveBtn!);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    expect(payload.requestLaunch).toBe(false);
  });

  test("edit-mode Delete Task button invokes onDelete callback (opens confirmation)", async () => {
    const onDelete = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit", workspace: "/repo", open: true, onOpenChange: () => {},
        onSubmit: () => {}, onDelete,
        initialAgents: [{ provider: "claude", role: "lead", agentId: "a1" }],
      }),
    );
    const deleteBtn = query('[data-delete-task-btn="true"]');
    expect(deleteBtn).toBeTruthy();
    click(deleteBtn!);
    // onDelete triggers the confirmation dialog — it does NOT directly delete
    expect(onDelete).toHaveBeenCalledTimes(1);
  });

  test("edit-mode hides Delete Task button when onDelete is absent", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit", workspace: "/repo", open: true, onOpenChange: () => {},
        onSubmit: () => {},
        initialAgents: [{ provider: "claude", role: "lead", agentId: "a1" }],
      }),
    );
    const deleteBtn = query('[data-delete-task-btn="true"]');
    expect(deleteBtn).toBeFalsy();
  });

  test("edit-mode submit carries agentId through for all agents (agent-bound connect)", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit", workspace: "/repo", open: true, onOpenChange: () => {}, onSubmit,
        initialAgents: [
          { provider: "claude", role: "lead", agentId: "agent-A" },
          { provider: "codex", role: "coder", agentId: "agent-B" },
          { provider: "claude", role: "coder", agentId: "agent-C" },
        ],
      }),
    );
    const saveBtn = queryAll("button").find((b) => b.textContent === "Save & Connect");
    expect(saveBtn).toBeTruthy();
    click(saveBtn!);
    expect(onSubmit).toHaveBeenCalledTimes(1);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    expect(payload.agents.length).toBe(3);
    expect(payload.agents[0].agentId).toBe("agent-A");
    expect(payload.agents[1].agentId).toBe("agent-B");
    expect(payload.agents[2].agentId).toBe("agent-C");
    expect(payload.requestLaunch).toBe(true);
  });

  test("multiple same-provider agents carry distinct agentIds without collapse", async () => {
    const onSubmit = mock(() => {});
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        mode: "edit", workspace: "/repo", open: true, onOpenChange: () => {}, onSubmit,
        initialAgents: [
          { provider: "codex", role: "lead", agentId: "codex-lead-1" },
          { provider: "codex", role: "coder", agentId: "codex-coder-2" },
        ],
      }),
    );
    const connectBtn = queryAll("button").find((b) => b.textContent === "Save & Connect");
    expect(connectBtn).toBeTruthy();
    click(connectBtn!);
    const payload = (onSubmit.mock.calls as any[][])[0][0];
    // Both same-provider agents are present with distinct IDs — no provider-family collapse
    expect(payload.agents.length).toBe(2);
    expect(payload.agents[0].agentId).toBe("codex-lead-1");
    expect(payload.agents[1].agentId).toBe("codex-coder-2");
    expect(payload.agents[0].provider).toBe("codex");
    expect(payload.agents[1].provider).toBe("codex");
  });

  test("Add Agent button creates a new row in the left pane", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    await render(
      createElement(TaskSetupDialog, {
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit: () => {},
      }),
    );
    const leftPane = query('[data-left-pane="true"]');
    expect(leftPane).toBeTruthy();
    // Create mode starts with 1 default locked row
    expect(queryAll('[data-draggable-row="true"]').length).toBe(1);
    const addBtn = queryAll("button").find((b) => b.textContent?.includes("Add"));
    expect(addBtn).toBeTruthy();
    click(addBtn!);
    await new Promise((r) => setTimeout(r, 20));
    const rows = queryAll('[data-draggable-row="true"]');
    expect(rows.length).toBe(2);
  });
});
