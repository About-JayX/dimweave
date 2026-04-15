import { describe, expect, mock, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

// Mock the store — includes agents for task-001 to verify task-scoped reads
mock.module("@/stores/task-store", () => ({
  useTaskStore: (sel: (s: any) => any) => sel({
    activeTaskId: "task-001",
    lastSave: null,
    taskAgents: {
      "task-001": [
        { agentId: "a1", taskId: "task-001", provider: "claude", role: "lead", order: 0, createdAt: 1 },
        { agentId: "a2", taskId: "task-001", provider: "codex", role: "coder", order: 1, createdAt: 2 },
      ],
    },
  }),
}));

import { TaskHeader } from "./TaskHeader";

const baseTask = {
  taskId: "task-001",
  title: "Fix routing bug",
  projectRoot: "/repo",
  taskWorktreeRoot: "/repo",
  workspaceRoot: "/repo",
  status: "implementing" as const,
  leadProvider: "claude" as const,
  coderProvider: "codex" as const,
  createdAt: 1,
  updatedAt: 1,
};

describe("TaskHeader", () => {
  test("shows task title, taskId, workspace, and status", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask }),
    );
    expect(html).toContain("Fix routing bug");
    expect(html).toContain("task-001");
    expect(html).toContain("/repo");
    expect(html).toContain("In progress");
  });

  test("renders review badge when provided", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, {
        task: { ...baseTask, status: "reviewing" },
        reviewBadge: { label: "Review", tone: "warning" },
      }),
    );
    expect(html).toContain("Review");
    expect(html).toContain("amber");
  });

  test("does not render review badge when absent", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, reviewBadge: null }),
    );
    expect(html).toContain("In progress");
    expect(html).not.toContain("Pending Review");
  });

  test("renders agent chip inline on expanded card (card-only surface contract)", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask }),
    );
    expect(html).toContain("lead:");
    expect(html).toContain("claude");
  });

  test("shows no agent badges when task has zero agents", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: { ...baseTask, taskId: "task-no-agents" } }),
    );
    expect(html).not.toContain("lead:");
    expect(html).not.toContain("coder:");
  });

  test("renders icon-only edit affordance when onEditTask is provided", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, onEditTask: () => {} }),
    );
    // icon-only: button carries a data marker and tooltip, no visible text label
    expect(html).toContain('data-edit-icon="true"');
    expect(html).toContain('title="Edit task"');
    expect(html).toContain('aria-label="Edit task"');
  });

  test("does not render edit affordance when onEditTask is absent", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask }),
    );
    expect(html).not.toContain('data-edit-icon="true"');
  });

  test("status chip has compact lower-right placement marker", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask }),
    );
    expect(html).toContain('data-task-status="true"');
  });

  test("agent pills render in store order (order-persistence regression)", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask }),
    );
    const leadIdx = html.indexOf("lead:");
    const coderIdx = html.indexOf("coder:");
    expect(leadIdx).toBeGreaterThan(-1);
    expect(coderIdx).toBeGreaterThan(-1);
    expect(leadIdx).toBeLessThan(coderIdx);
  });
});

describe("card-only selection contract", () => {
  test("inactive collapsed card exposes role=button for keyboard selection", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, collapsed: true, onClick: () => {} } as any),
    );
    expect(html).toContain('role="button"');
    expect(html).toContain("tabindex");
  });

  test("active card without onClick has no button role (non-navigable root)", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, onEditTask: () => {} }),
    );
    expect(html).not.toContain('role="button"');
  });

  test("Edit icon is reachable on the active card via data marker", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, onEditTask: () => {} }),
    );
    expect(html).toContain('data-edit-icon="true"');
    expect(html).toContain("<button");
  });
});

describe("collapsed accordion header", () => {
  test("marks container with data-collapsed when collapsed", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, collapsed: true, onClick: () => {} } as any),
    );
    expect(html).toContain('data-collapsed="true"');
  });

  test("applies cursor-pointer when onClick is provided", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, onClick: () => {} } as any),
    );
    expect(html).toContain("cursor-pointer");
  });

  test("hides edit button in collapsed mode even when handler exists", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, {
        task: baseTask,
        collapsed: true,
        onClick: () => {},
        onEditTask: () => {},
      } as any),
    );
    expect(html).not.toContain("Edit task");
  });

  test("dialog-redesign regression: card pills show both providers in store order", () => {
    // After the unified two-pane dialog redesign (75d1fce6, 81fc11ac, 48759edd),
    // card pills must still render from the persisted agent store order.
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask }),
    );
    expect(html).toContain("claude");
    expect(html).toContain("codex");
    const claudeIdx = html.indexOf("claude");
    const codexIdx = html.indexOf("codex");
    expect(claudeIdx).toBeLessThan(codexIdx);
  });

  test("does not leak active task agents into a different task header", () => {
    // Mock has agents for task-001 (active). Render header for task-other.
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: { ...baseTask, taskId: "task-other" } }),
    );
    expect(html).not.toContain("lead:");
    expect(html).not.toContain("claude");
  });
});
