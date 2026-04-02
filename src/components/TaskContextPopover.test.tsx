import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskContextPopover } from "./TaskContextPopover";

function installTauriStub() {
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
  });
}

describe("TaskContextPopover", () => {
  test("renders the task-context pane when requested", () => {
    installTauriStub();
    const html = renderToStaticMarkup(
      <TaskContextPopover
        activePane="task"
        onClose={() => {}}
        task={null}
      />,
    );

    expect(html).toContain("data-shell-sidebar-drawer=\"true\"");
    expect(html).toContain("Task workspace");
    expect(html).toContain("No active task");
    expect(html).not.toContain("Runtime control");
    expect(html).not.toContain(
      "The conversation timeline stays live, but task context and review state will appear here once a task is active.",
    );
  });

  test("renders the agents pane when requested", () => {
    installTauriStub();
    const html = renderToStaticMarkup(
      <TaskContextPopover
        activePane="agents"
        onClose={() => {}}
        task={{
          taskId: "task-1",
          title: "Refine shell header",
          workspaceRoot: "/Users/jason/Desktop/figma",
          status: "active",
          reviewStatus: null,
          createdAt: 1,
          updatedAt: 1,
        }}
      />,
    );

    expect(html).toContain("Agents");
    expect(html).toContain("Runtime control");
    expect(html).not.toContain("Task workspace");
  });

  test("renders approvals inside the shared shell drawer", () => {
    installTauriStub();
    const html = renderToStaticMarkup(
      <TaskContextPopover activePane="approvals" onClose={() => {}} task={null} />,
    );

    expect(html).toContain("Approvals");
    expect(html).toContain("Permission queue");
    expect(html).toContain("No pending approvals.");
  });
});
