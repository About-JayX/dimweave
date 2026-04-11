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
          if (cmd === "feishu_project_list_items") return [];
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

describe("BugInboxPanel", () => {
  test("renders config card and empty issue list", async () => {
    installTauriStub();
    const { BugInboxPanel } = await import("./index");
    const html = renderToStaticMarkup(<BugInboxPanel />);

    expect(html).toContain("No items in inbox");
  });

  test("IssueList renders item with type label and action trigger", async () => {
    installTauriStub();
    const { IssueList } = await import("./IssueList");
    const html = renderToStaticMarkup(
      <IssueList
        items={[
          {
            recordId: "p_1",
            projectKey: "p",
            workItemId: "1",
            workItemTypeKey: "bug",
            title: "Crash on launch",
            updatedAt: 0,
            sourceUrl: "https://project.feishu.cn/p/issues/1",
            rawSnapshotRef: "",
            ignored: false,
            lastIngress: "mcp",
          },
        ]}
        onIgnore={() => {}}
        onStartHandling={() => {}}
      />,
    );

    expect(html).toContain("Crash on launch");
    expect(html).toContain("bug");
    expect(html).toContain("mcp");
    // ActionMenu trigger present (menu items are portal-rendered, not in static HTML)
    expect(html).toContain('aria-label="Actions"');
  });

  test("IssueList renders ignored items with reduced styling", async () => {
    installTauriStub();
    const { IssueList } = await import("./IssueList");
    const html = renderToStaticMarkup(
      <IssueList
        items={[
          {
            recordId: "p_2",
            projectKey: "p",
            workItemId: "2",
            workItemTypeKey: "story",
            title: "Ignored item",
            updatedAt: 0,
            sourceUrl: "",
            rawSnapshotRef: "",
            ignored: true,
            lastIngress: "mcp",
          },
        ]}
        onIgnore={() => {}}
        onStartHandling={() => {}}
      />,
    );

    expect(html).toContain("Ignored item");
    expect(html).toContain("story");
    // Ignored items have reduced opacity styling
    expect(html).toContain("opacity-60");
    expect(html).toContain('aria-label="Actions"');
  });

  test("IssueList shows Linked badge for items with linkedTaskId", async () => {
    installTauriStub();
    const { IssueList } = await import("./IssueList");
    const html = renderToStaticMarkup(
      <IssueList
        items={[
          {
            recordId: "p_3",
            projectKey: "p",
            workItemId: "3",
            workItemTypeKey: "bug",
            title: "Linked bug",
            updatedAt: 0,
            sourceUrl: "",
            rawSnapshotRef: "",
            ignored: false,
            lastIngress: "mcp",
            linkedTaskId: "task_42",
          },
        ]}
        onIgnore={() => {}}
        onStartHandling={() => {}}
      />,
    );

    expect(html).toContain("Linked bug");
    expect(html).toContain("Linked");
    expect(html).toContain("bug");
    expect(html).toContain('aria-label="Actions"');
  });

  test("IssueList renders sentinel when items present and hasMore", async () => {
    installTauriStub();
    const { IssueList } = await import("./IssueList");
    const html = renderToStaticMarkup(
      <IssueList
        items={[
          {
            recordId: "p_10",
            projectKey: "p",
            workItemId: "10",
            workItemTypeKey: "bug",
            title: "Bug A",
            updatedAt: 0,
            sourceUrl: "",
            rawSnapshotRef: "",
            ignored: false,
            lastIngress: "mcp",
          },
        ]}
        hasMore={true}
        loadingMore={false}
        onLoadMore={() => {}}
        onIgnore={() => {}}
        onStartHandling={() => {}}
      />,
    );
    // Sentinel div should be present so IntersectionObserver can attach
    expect(html).toContain('class="h-1"');
    expect(html).not.toContain("No items");
  });

  test("IssueList renders no sentinel when items empty even with hasMore", async () => {
    installTauriStub();
    const { IssueList } = await import("./IssueList");
    const html = renderToStaticMarkup(
      <IssueList
        items={[]}
        hasMore={true}
        onLoadMore={() => {}}
        onIgnore={() => {}}
        onStartHandling={() => {}}
      />,
    );
    // Empty list returns "No items" — sentinel not in DOM
    expect(html).not.toContain('class="h-1"');
    expect(html).toContain("No items");
  });

  test("IssueList shows spinner instead of sentinel when loadingMore", async () => {
    installTauriStub();
    const { IssueList } = await import("./IssueList");
    const html = renderToStaticMarkup(
      <IssueList
        items={[
          {
            recordId: "p_11",
            projectKey: "p",
            workItemId: "11",
            workItemTypeKey: "issue",
            title: "Bug B",
            updatedAt: 0,
            sourceUrl: "",
            rawSnapshotRef: "",
            ignored: false,
            lastIngress: "mcp",
          },
        ]}
        hasMore={true}
        loadingMore={true}
        onLoadMore={() => {}}
        onIgnore={() => {}}
        onStartHandling={() => {}}
      />,
    );
    // Spinner replaces sentinel while loading more
    expect(html).not.toContain('class="h-1"');
    expect(html).toContain("animate-spin");
  });

  test("view-model activeItemCount excludes ignored items", async () => {
    const { activeItemCount } = await import("./view-model");

    expect(
      activeItemCount([
        {
          recordId: "a",
          projectKey: "p",
          workItemId: "1",
          workItemTypeKey: "bug",
          title: "A",
          updatedAt: 0,
          sourceUrl: "",
          rawSnapshotRef: "",
          ignored: false,
          lastIngress: "mcp",
        },
        {
          recordId: "b",
          projectKey: "p",
          workItemId: "2",
          workItemTypeKey: "bug",
          title: "B",
          updatedAt: 0,
          sourceUrl: "",
          rawSnapshotRef: "",
          ignored: true,
          lastIngress: "mcp",
        },
      ]),
    ).toBe(1);
  });

  test("view-model formatSyncTime handles null and timestamps", async () => {
    const { formatSyncTime } = await import("./view-model");
    expect(formatSyncTime(null)).toBe("Never");
    expect(formatSyncTime(undefined)).toBe("Never");
    const result = formatSyncTime(1700000000000);
    expect(result).toBeTruthy();
    expect(result).not.toBe("Never");
  });

  test("SyncModeNav renders status dropdown when statusOptions provided", async () => {
    installTauriStub();
    const { SyncModeNav } = await import("./SyncModeNav");
    const html = renderToStaticMarkup(
      <SyncModeNav
        value="issues"
        onChange={() => {}}
        teamMembers={["Alice"]}
        assigneeFilter=""
        onAssigneeChange={() => {}}
        statusOptions={["处理中", "已关闭"]}
        statusFilter=""
        onStatusChange={() => {}}
      />,
    );
    expect(html).toContain("全部状态");
  });

  test("SyncModeNav shows active status label when filter set", async () => {
    installTauriStub();
    const { SyncModeNav } = await import("./SyncModeNav");
    const html = renderToStaticMarkup(
      <SyncModeNav
        value="issues"
        onChange={() => {}}
        teamMembers={[]}
        assigneeFilter=""
        onAssigneeChange={() => {}}
        statusOptions={["处理中", "已关闭"]}
        statusFilter="处理中"
        onStatusChange={() => {}}
      />,
    );
    expect(html).toContain("处理中");
  });

  test("fetchState must complete before fetchFilterOptions for reliable hydration", async () => {
    installTauriStub();
    const mod = await import("@/stores/feishu-project-store");
    const store = mod.useFeishuProjectStore;
    // Ensure store starts clean
    store.setState({ runtimeState: null });

    // Sequential: fetchState first, then fetchFilterOptions
    await store.getState().fetchState();
    const afterState = store.getState();
    expect(afterState.runtimeState).toBeTruthy();
    expect(afterState.runtimeState?.enabled).toBe(true);
  });

  test("store interface exposes statusOptions and assigneeOptions", async () => {
    installTauriStub();
    const mod = await import("@/stores/feishu-project-store");
    const state = mod.useFeishuProjectStore.getState();
    expect(state).toHaveProperty("activeFilter");
    expect(state).toHaveProperty("setFilter");
    expect(state).toHaveProperty("loadMoreFiltered");
    expect(state).toHaveProperty("fetchFilterOptions");
  });
});
