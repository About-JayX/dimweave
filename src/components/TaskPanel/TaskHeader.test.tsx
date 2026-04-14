import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";

// Stub Tauri internals — TaskHeader now uses useTaskStore
let callbackId = 0;
Object.assign(globalThis, {
  window: {
    __TAURI_INTERNALS__: {
      transformCallback: () => ++callbackId,
      unregisterCallback: () => {},
      invoke: async (cmd: string) => {
        if (cmd === "plugin:event|listen") return callbackId;
        if (cmd === "daemon_get_status_snapshot") {
          return { agents: [], claudeRole: "lead", codexRole: "coder" };
        }
        if (cmd === "daemon_get_task_snapshot") return null;
        if (cmd === "codex_list_models") return [];
        if (cmd === "codex_get_profile") return null;
        return null;
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: () => {},
    },
    addEventListener: () => {},
    removeEventListener: () => {},
    innerWidth: 800,
  },
  document: {
    addEventListener: () => {},
    removeEventListener: () => {},
  },
  localStorage: {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    length: 0,
  },
});

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
      createElement(TaskHeader, {
        task: baseTask,
        reviewBadge: null,
      }),
    );
    expect(html).toContain("In progress");
    expect(html).not.toContain("Pending Review");
    expect(html).not.toContain("Pending Approval");
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
