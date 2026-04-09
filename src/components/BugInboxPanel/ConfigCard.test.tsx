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
          return null;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => {},
      },
    },
  });
}

describe("ConfigCard", () => {
  test("shows MCP connection status when configured", async () => {
    installTauriStub();
    const { ConfigCard } = await import("./ConfigCard");
    const html = renderToStaticMarkup(
      <ConfigCard
        runtimeState={{
          enabled: true,
          domain: "https://project.feishu.cn",
          workspaceHint: "myspace",
          refreshIntervalMinutes: 10,
          mcpStatus: "connected",
          discoveredToolCount: 8,
          tokenLabel: "tok_a***",
        }}
        loading={false}
        onSave={() => {}}
        onSync={() => {}}
      />,
    );

    expect(html).toContain("Connected");
    expect(html).toContain("8 tools discovered");
    expect(html).toContain("tok_a***");
    expect(html).toContain("Sync now");
    expect(html).toContain("Edit");
  });

  test("shows unauthorized status with error", async () => {
    installTauriStub();
    const { ConfigCard } = await import("./ConfigCard");
    const html = renderToStaticMarkup(
      <ConfigCard
        runtimeState={{
          enabled: true,
          domain: "https://project.feishu.cn",
          refreshIntervalMinutes: 10,
          mcpStatus: "unauthorized",
          discoveredToolCount: 0,
          tokenLabel: "tok_x***",
          lastError: "invalid MCP token",
        }}
        loading={false}
        onSave={() => {}}
        onSync={() => {}}
      />,
    );

    expect(html).toContain("Unauthorized");
    expect(html).toContain("invalid MCP token");
  });

  test("unconfigured state shows Configure button", async () => {
    installTauriStub();
    const { ConfigCard } = await import("./ConfigCard");
    const html = renderToStaticMarkup(
      <ConfigCard
        runtimeState={null}
        loading={false}
        onSave={() => {}}
        onSync={() => {}}
      />,
    );

    expect(html).toContain("Not configured");
    expect(html).toContain("Configure");
  });
});
