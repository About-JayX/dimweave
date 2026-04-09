import { describe, expect, test } from "bun:test";

function installTauriStub() {
  let callbackId = 0;
  Object.assign(globalThis, {
    window: {
      __TAURI_INTERNALS__: {
        transformCallback: () => ++callbackId,
        unregisterCallback: () => {},
        invoke: async (cmd: string) => {
          if (cmd === "plugin:event|listen") return callbackId;
          if (cmd === "feishu_project_get_state") {
            return {
              enabled: true,
              domain: "https://project.feishu.cn",
              workspaceHint: "myspace",
              refreshIntervalMinutes: 10,
              mcpStatus: "connected",
              discoveredToolCount: 5,
              tokenLabel: "tok_a***",
            };
          }
          if (cmd === "feishu_project_list_items") {
            return [
              {
                recordId: "proj_1",
                projectKey: "proj",
                workItemId: "1",
                workItemTypeKey: "bug",
                title: "Crash",
                updatedAt: 0,
                sourceUrl: "https://example.com",
                rawSnapshotRef: "",
                ignored: false,
                lastIngress: "mcp",
              },
            ];
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

describe("feishu-project-store", () => {
  test("exports store with expected actions", async () => {
    installTauriStub();
    const { useFeishuProjectStore } = await import("./feishu-project-store");
    const state = useFeishuProjectStore.getState();

    expect(state.runtimeState).toBeNull();
    expect(state.items).toEqual([]);
    expect(typeof state.fetchState).toBe("function");
    expect(typeof state.fetchItems).toBe("function");
    expect(typeof state.saveConfig).toBe("function");
    expect(typeof state.syncNow).toBe("function");
    expect(typeof state.setIgnored).toBe("function");
    expect(typeof state.startHandling).toBe("function");
    expect(typeof state.cleanup).toBe("function");
  });

  test("types match Rust camelCase serde output", () => {
    const item = {
      recordId: "proj_1",
      projectKey: "proj",
      workItemId: "1",
      workItemTypeKey: "bug",
      title: "Test",
      updatedAt: 0,
      sourceUrl: "",
      rawSnapshotRef: "",
      ignored: false,
      lastIngress: "mcp" as const,
    };
    expect(item.workItemId).toBe("1");
    expect(item.lastIngress).toBe("mcp");
  });
});
