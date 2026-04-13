import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

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
          if (cmd === "feishu_project_get_state") {
            return {
              enabled: false,
              pollIntervalMinutes: 10,
              localWebhookPath: "/wh",
              webhookEnabled: false,
            };
          }
          if (cmd === "feishu_project_list_items") return [];
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
  test("normalizes invalid persisted sidebar widths", async () => {
    installTauriStub();
    const { normalizeSidebarWidth } = await import("./TaskContextPopover");

    expect(normalizeSidebarWidth("not-a-number")).toBe(280);
    expect(normalizeSidebarWidth("120")).toBe(280);
    expect(normalizeSidebarWidth("999")).toBe(640);
    expect(normalizeSidebarWidth("320")).toBe(320);
  });

  test("renders an embedded shell panel even while collapsed", async () => {
    installTauriStub();
    const { TaskContextPopover } = await import("./TaskContextPopover");
    const html = renderToStaticMarkup(
      <TaskContextPopover activePane={null} onClose={() => {}} task={null} />,
    );

    expect(html).toContain('data-shell-sidebar-panel="true"');
    expect(html).toContain('aria-hidden="true"');
    expect(html).not.toContain("fixed left-20 top-4");
  });

  test("renders the task-context pane when requested", async () => {
    installTauriStub();
    const { TaskContextPopover } = await import("./TaskContextPopover");
    const html = renderToStaticMarkup(
      <TaskContextPopover activePane="task" onClose={() => {}} task={null} />,
    );

    expect(html).toContain('data-shell-sidebar-panel="true"');
    expect(html).toContain("Task workspace");
    expect(html).toContain("No active task");
    expect(html).not.toContain("Runtime control");
    expect(html).not.toContain("Active sessions"); // dashboard metric grid removed
    expect(html).not.toContain(
      "The conversation timeline stays live, but task context and review state will appear here once a task is active.",
    );
  });

  test("renders approvals inside the shared shell drawer", async () => {
    installTauriStub();
    const { TaskContextPopover } = await import("./TaskContextPopover");
    const html = renderToStaticMarkup(
      <TaskContextPopover
        activePane="approvals"
        onClose={() => {}}
        task={null}
      />,
    );

    expect(html).toContain("Approvals");
    expect(html).toContain("Permission queue");
    expect(html).toContain("No pending approvals.");
  });

  test("renders tools pane inside the shared shell drawer", async () => {
    installTauriStub();
    const { TaskContextPopover } = await import("./TaskContextPopover");
    const html = renderToStaticMarkup(
      <TaskContextPopover activePane="bugs" onClose={() => {}} task={null} />,
    );

    expect(html).toContain("Tools");
    expect(html).toContain("Integrations");
    expect(html).not.toContain("Bug Inbox");
    expect(html).toContain("Feishu Project");
    expect(html).toContain("Telegram");
  });
});
