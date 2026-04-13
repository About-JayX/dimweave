import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskHeader } from "./TaskHeader";

const baseTask = {
  taskId: "task-001",
  title: "Fix routing bug",
  workspaceRoot: "/repo",
  status: "implementing" as const,
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
    // Both status badge and review badge should appear
    expect(html).toContain("Review");
    // amber styles indicate review badge tone
    expect(html).toContain("amber");
  });

  test("renders an edit-task button", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, { task: baseTask, onEditTask: () => {} }),
    );
    expect(html).toContain("Edit task");
  });

  test("does not render review badge when absent", () => {
    const html = renderToStaticMarkup(
      createElement(TaskHeader, {
        task: baseTask,
        reviewBadge: null,
      }),
    );
    // No explicit review badge — only status label present
    expect(html).toContain("In progress");
    // review badge label not present
    expect(html).not.toContain("Pending Review");
    expect(html).not.toContain("Pending Approval");
  });
});
