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

  test("IssueList renders items with ignore button", async () => {
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
    expect(html).toContain("Ignore");
    expect(html).toContain("Handle");
  });

  test("IssueList shows Restore for ignored items", async () => {
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

    expect(html).toContain("Restore");
    expect(html).not.toContain("Handle");
  });

  test("IssueList shows Open task for linked items", async () => {
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

    expect(html).toContain("Open task");
    expect(html).toContain("Linked");
    expect(html).not.toContain("Handle");
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
});
