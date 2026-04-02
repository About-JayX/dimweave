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
  test("renders task context as a side trigger instead of a top runtime header", async () => {
    installTauriStub();
    const { ShellContextBar } = await import("./ShellContextBar");
    const html = renderToStaticMarkup(
      <ShellContextBar
        mobileInspectorOpen={false}
        onToggleMobileInspector={() => {}}
      />,
    );

    expect(html).toContain("Task context");
    expect(html).not.toContain("AGENTNEXUS");
    expect(html).not.toContain("Runtime");
    expect(html).not.toContain("Claude");
    expect(html).not.toContain("Codex");
  });
});
