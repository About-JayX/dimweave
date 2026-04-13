import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

// Stub must exist before any component import triggers bridge-store init
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
  localStorage: {
    getItem: () => null,
    setItem: () => {},
    removeItem: () => {},
    clear: () => {},
    key: () => null,
    length: 0,
  },
});

describe("TaskSetupDialog", () => {
  test("renders create-mode dialog with provider selectors", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog
        mode="create"
        workspace="/repo"
        open={true}
        onOpenChange={() => {}}
        onSubmit={() => {}}
      />,
    );
    expect(html).toContain("New Task");
    expect(html).toContain("Lead provider");
    expect(html).toContain("Coder provider");
    expect(html).toContain("Create");
    expect(html).not.toContain("Title");
  });

  test("renders edit-mode dialog with agent configuration panels", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog
        mode="edit"
        workspace="/repo"
        open={true}
        onOpenChange={() => {}}
        onSubmit={() => {}}
        initialLeadProvider="codex"
        initialCoderProvider="claude"
      />,
    );
    expect(html).toContain("Edit Task");
    expect(html).toContain("Save");
    expect(html).toContain("Lead provider");
    expect(html).toContain("Runtime control");
  });

  test("does not render content when closed", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog
        mode="create"
        workspace="/repo"
        open={false}
        onOpenChange={() => {}}
        onSubmit={() => {}}
      />,
    );
    expect(html).not.toContain("New Task");
    expect(html).not.toContain("Lead provider");
  });
});
