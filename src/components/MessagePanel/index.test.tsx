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
          return null;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => {},
      },
    },
  });
}

describe("MessagePanel", () => {
  test("keeps the chat header free of logs and approvals buttons", async () => {
    installTauriStub();
    const [{ MessagePanel }, { useBridgeStore }] = await Promise.all([
      import("./index"),
      import("@/stores/bridge-store"),
    ]);

    useBridgeStore.setState((state) => ({
      ...state,
      messages: [],
      terminalLines: [],
      permissionPrompts: [],
    }));

    const html = renderToStaticMarkup(<MessagePanel surfaceMode="chat" />);

    expect(html).toContain("Primary timeline");
    expect(html).not.toContain("Approvals");
    expect(html).not.toContain(">Logs<");
  });

  test("renders runtime logs as the active main surface when requested", async () => {
    installTauriStub();
    const { MessagePanel } = await import("./index");

    const html = renderToStaticMarkup(<MessagePanel surfaceMode="logs" />);

    expect(html).toContain("Runtime logs");
    expect(html).toContain("No logs.");
    expect(html).not.toContain("Primary timeline");
  });
});
