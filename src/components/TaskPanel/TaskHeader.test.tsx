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
      ],
    },
  }),
}));

import { TaskHeader } from "./TaskHeader";

const baseTask = {
  taskId: "task-001",
  title: "Fix routing bug",
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

  test("shows no agent badges when task has zero agents", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: { ...baseTask, taskId: "task-no-agents" } }),
    );
    expect(html).not.toContain("lead:");
    expect(html).not.toContain("coder:");
  });

  test("renders edit-task button when onEditTask is provided", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, onEditTask: () => {} }),
    );
    expect(html).toContain("Edit task");
  });

  test("does not render edit-task button when onEditTask is absent", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask }),
    );
    expect(html).not.toContain("Edit task");
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

  test("does not leak active task agents into a different task header", () => {
    // Mock has agents for task-001 (active). Render header for task-other.
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: { ...baseTask, taskId: "task-other" } }),
    );
    expect(html).not.toContain("lead:");
    expect(html).not.toContain("claude");
  });
});
