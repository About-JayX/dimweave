import { describe, expect, mock, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

// Mock the store to avoid mock.module bleed-over from other test files
mock.module("@/stores/task-store", () => ({
  useTaskStore: (sel: (s: any) => any) => sel({
    activeTaskId: "task-001",
    lastSave: null,
    providerSummaries: {
      "task-001": { taskId: "task-001", leadOnline: false, coderOnline: false },
    },
    taskAgents: {},
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

  test("shows fallback provider badges when no task agents configured", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask }),
    );
    expect(html).toContain("lead:");
    expect(html).toContain("coder:");
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
