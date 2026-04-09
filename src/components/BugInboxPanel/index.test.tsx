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
              projectKey: "myproj",
              tokenLabel: "plugi***",
              pollIntervalMinutes: 10,
              localWebhookPath: "/wh",
              webhookEnabled: false,
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
            lastIngress: "poll",
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
            lastIngress: "webhook",
          },
        ]}
        onIgnore={() => {}}
        onStartHandling={() => {}}
      />,
    );

    expect(html).toContain("Restore");
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
          lastIngress: "poll",
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
          lastIngress: "poll",
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

  test("ConfigCard edit form includes sync parameter fields", async () => {
    installTauriStub();
    const { ConfigCard } = await import("./ConfigCard");
    const html = renderToStaticMarkup(
      <ConfigCard
        runtimeState={{
          enabled: true,
          projectKey: "proj",
          tokenLabel: "plugi***",
          userKey: "u_1",
          pollIntervalMinutes: 15,
          publicWebhookBaseUrl: "https://abc.ngrok.app",
          localWebhookPath: "/wh",
          webhookEnabled: true,
        }}
        loading={false}
        onSave={() => {}}
        onSync={() => {}}
      />,
    );

    // Configured view should show project key
    expect(html).toContain("proj");
    expect(html).toContain("plugi***");
    expect(html).toContain("Edit");
    expect(html).toContain("Sync now");
  });

  test("ConfigCard unconfigured state shows Configure button", async () => {
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
