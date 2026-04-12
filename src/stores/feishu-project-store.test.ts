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

  test("fetchFilterOptions re-reads runtime state after command", async () => {
    const invoked: string[] = [];
    let callbackId = 0;
    Object.assign(globalThis, {
      window: {
        __TAURI_INTERNALS__: {
          transformCallback: () => ++callbackId,
          unregisterCallback: () => {},
          invoke: async (cmd: string) => {
            invoked.push(cmd);
            if (cmd === "plugin:event|listen") return callbackId;
            if (cmd === "feishu_project_get_state") {
              return {
                enabled: true,
                syncMode: "issues",
                refreshIntervalMinutes: 10,
                mcpStatus: "connected",
                discoveredToolCount: 5,
                tokenLabel: "tok_a***",
                teamMembers: [],
                statusOptions: ["处理中", "已关闭"],
                assigneeOptions: ["Alice"],
              };
            }
            if (cmd === "feishu_project_fetch_filter_options") return null;
            return null;
          },
        },
        __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
      },
    });

    const { useFeishuProjectStore } = await import("./feishu-project-store");
    useFeishuProjectStore.setState({ runtimeState: null });

    await useFeishuProjectStore.getState().fetchFilterOptions();

    const rs = useFeishuProjectStore.getState().runtimeState;
    expect(rs).toBeTruthy();
    expect(rs?.statusOptions).toEqual(["处理中", "已关闭"]);
    expect(rs?.assigneeOptions).toEqual(["Alice"]);

    // Verify command sequence: filter options then get_state
    const filterIdx = invoked.indexOf("feishu_project_fetch_filter_options");
    const stateIdx = invoked.lastIndexOf("feishu_project_get_state");
    expect(filterIdx).toBeGreaterThanOrEqual(0);
    expect(stateIdx).toBeGreaterThan(filterIdx);
  });

  test("hydrateIssuesFirstPage fetches state, options, and items atomically", async () => {
    installTauriStub();
    const { useFeishuProjectStore } = await import("./feishu-project-store");
    useFeishuProjectStore.setState({ issuesHydrated: false, runtimeState: null, items: [] });

    const hydrate = useFeishuProjectStore.getState().hydrateIssuesFirstPage;
    expect(typeof hydrate).toBe("function");

    await hydrate();
    const state = useFeishuProjectStore.getState();
    expect(state.issuesHydrated).toBe(true);
    expect(state.runtimeState).toBeTruthy();
    expect(state.items.length).toBeGreaterThan(0);
  });

  test("saveConfig re-fetches filter options only on workspace change", async () => {
    const invoked: string[] = [];
    let currentWs = "old_ws";
    let callbackId = 0;
    Object.assign(globalThis, {
      window: {
        __TAURI_INTERNALS__: {
          transformCallback: () => ++callbackId,
          unregisterCallback: () => {},
          invoke: async (cmd: string) => {
            invoked.push(cmd);
            if (cmd === "plugin:event|listen") return callbackId;
            if (cmd === "feishu_project_save_config" || cmd === "feishu_project_get_state") {
              return {
                enabled: true, syncMode: "issues", refreshIntervalMinutes: 10,
                mcpStatus: "connected", discoveredToolCount: 5, tokenLabel: "tok***",
                workspaceHint: currentWs, teamMembers: [], statusOptions: [], assigneeOptions: [],
              };
            }
            if (cmd === "feishu_project_fetch_filter_options") return null;
            return null;
          },
        },
        __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
      },
    });
    const { useFeishuProjectStore } = await import("./feishu-project-store");
    const mkRs = (ws: string) => ({
      enabled: true, syncMode: "issues" as const, refreshIntervalMinutes: 10,
      mcpStatus: "connected" as const, discoveredToolCount: 5, tokenLabel: "tok***",
      workspaceHint: ws, teamMembers: [] as string[], statusOptions: [] as string[], assigneeOptions: [] as string[],
    });
    const mkCfg = (ws: string) => ({
      enabled: true, domain: "https://project.feishu.cn", mcp_user_token: "",
      workspace_hint: ws, refresh_interval_minutes: 10, sync_mode: "issues" as const,
    });
    // Workspace change: old_ws → new_ws
    useFeishuProjectStore.setState({ runtimeState: mkRs("old_ws") });
    currentWs = "new_ws";
    invoked.length = 0;
    await useFeishuProjectStore.getState().saveConfig(mkCfg("new_ws"));
    expect(invoked).toContain("feishu_project_fetch_filter_options");
    // Same workspace: new_ws → new_ws
    invoked.length = 0;
    await useFeishuProjectStore.getState().saveConfig(mkCfg("new_ws"));
    expect(invoked).not.toContain("feishu_project_fetch_filter_options");
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
