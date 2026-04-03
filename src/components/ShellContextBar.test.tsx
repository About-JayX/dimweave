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
          return null;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => {},
      },
    },
  });
}

describe("ShellContextBar", () => {
  test("renders a VS Code-style side rail without reviving the top runtime header", async () => {
    installTauriStub();
    const { ShellContextBar } = await import("./ShellContextBar");
    const html = renderToStaticMarkup(
      <ShellContextBar
        activeItem={null}
        messageCount={0}
        themeMode="auto"
        radiusMode="rounded"
        onToggle={() => {}}
        onThemeChange={() => {}}
        onRadiusToggle={() => {}}
      />,
    );

    expect(html).toContain("Task context");
    expect(html).toContain("Agents");
    expect(html).toContain("Approvals");
    expect(html).toContain("Logs");
    expect(html).not.toContain("AgentNexus");
    expect(html).not.toContain("Daemon online");
  });
});
