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
  test("configured state shows workspace name and action trigger", async () => {
    installTauriStub();
    const { ConfigCard } = await import("./ConfigCard");
    const html = renderToStaticMarkup(
      <ConfigCard
        runtimeState={{
          enabled: true,
          domain: "https://project.feishu.cn",
          workspaceHint: "myspace",
          refreshIntervalMinutes: 10,
          syncMode: "todo",
          projectName: null,
          teamMembers: [],
          mcpStatus: "connected",
          discoveredToolCount: 8,
          tokenLabel: "tok_a***",
        }}
        loading={false}
        onSave={() => {}}
        onSync={() => {}}
      />,
    );

    // Shows workspace name as fallback when projectName is null
    expect(html).toContain("myspace");
    // ActionMenu trigger renders (portal menu items not in static HTML)
    expect(html).toContain('aria-label="Actions"');
  });

  test("configured state shows project name when available", async () => {
    installTauriStub();
    const { ConfigCard } = await import("./ConfigCard");
    const html = renderToStaticMarkup(
      <ConfigCard
        runtimeState={{
          enabled: true,
          domain: "https://project.feishu.cn",
          workspaceHint: "myspace",
          refreshIntervalMinutes: 10,
          syncMode: "todo",
          projectName: "极光矩阵--娱乐站",
          teamMembers: [],
          mcpStatus: "connected",
          discoveredToolCount: 8,
          tokenLabel: "tok_a***",
        }}
        loading={false}
        onSave={() => {}}
        onSync={() => {}}
      />,
    );

    expect(html).toContain("极光矩阵--娱乐站");
  });

  test("unauthorized state still renders configured layout", async () => {
    installTauriStub();
    const { ConfigCard } = await import("./ConfigCard");
    const html = renderToStaticMarkup(
      <ConfigCard
        runtimeState={{
          enabled: true,
          domain: "https://project.feishu.cn",
          refreshIntervalMinutes: 10,
          syncMode: "todo",
          projectName: null,
          teamMembers: [],
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

    // Renders configured layout with Feishu Project fallback name
    expect(html).toContain("Feishu Project");
    expect(html).toContain('aria-label="Actions"');
  });

  test("unconfigured state shows default label and action trigger", async () => {
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

    expect(html).toContain("Feishu Project");
    // ActionMenu renders with "Configure" item (portal, but trigger visible)
    expect(html).toContain('aria-label="Actions"');
  });
});
