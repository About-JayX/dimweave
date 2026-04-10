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
          if (cmd === "feishu_project_get_state") {
            return {
              enabled: false,
              pollIntervalMinutes: 10,
              localWebhookPath: "/wh",
              webhookEnabled: false,
            };
          }
          if (cmd === "feishu_project_list_items") return [];
          if (cmd === "telegram_get_state") return null;
          return null;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => {},
      },
    },
  });
}

describe("ToolsPanel", () => {
  test("renders both Telegram and Feishu Project disclosure sections", async () => {
    installTauriStub();
    const { ToolsPanel } = await import("./index");
    const html = renderToStaticMarkup(<ToolsPanel />);

    expect(html).toContain("Telegram");
    expect(html).toContain("Feishu Project");
  });

  test("Feishu Project section is expanded by default", async () => {
    installTauriStub();
    const { ToolsPanel } = await import("./index");
    const html = renderToStaticMarkup(<ToolsPanel />);

    // Feishu expanded: its child content (BugInboxPanel) is rendered
    expect(html).toContain("No items in inbox");
  });

  test("Telegram section is collapsed by default", async () => {
    installTauriStub();
    const { ToolsPanel } = await import("./index");
    const html = renderToStaticMarkup(<ToolsPanel />);

    // Telegram collapsed: its header shows but inner TelegramPanel content is not rendered
    expect(html).toContain("Telegram");
    // When collapsed, TelegramPanel's loading skeleton or status dot should not appear
    expect(html).not.toContain("123456:ABC-DEF");
  });
});
