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
  test("renders the chat empty state without reviving header controls", async () => {
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

    expect(html).toContain("No messages yet. Connect Claude and Codex to start bridging.");
    expect(html).not.toContain("Approvals");
    expect(html).not.toContain(">Logs<");
    expect(html).not.toContain("Runtime logs");
  });

  test("renders the log surface body without duplicating the top bar title", async () => {
    installTauriStub();
    const { MessagePanel } = await import("./index");

    const html = renderToStaticMarkup(<MessagePanel surfaceMode="logs" />);

    expect(html).toContain("No logs.");
    expect(html).not.toContain("No messages yet. Connect Claude and Codex to start bridging.");
    expect(html).not.toContain("Runtime logs");
    expect(html).not.toContain(">Clear<");
  });
});
