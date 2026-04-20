import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  getLogsFollowOutputMode,
  shouldAutoScrollLogsOnSurfaceChange,
} from "./view-model";

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
          if (cmd === "daemon_get_task_snapshot") return null;
          return null;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => {},
      },
    },
  });
}

describe("MessagePanel", () => {
  test("auto-scrolls logs only when entering the logs surface with content", () => {
    expect(shouldAutoScrollLogsOnSurfaceChange("chat", "logs", 3)).toBe(true);
    expect(shouldAutoScrollLogsOnSurfaceChange("logs", "logs", 3)).toBe(false);
    expect(shouldAutoScrollLogsOnSurfaceChange("chat", "logs", 0)).toBe(false);
  });

  test("follows log output only when already at the bottom", () => {
    expect(getLogsFollowOutputMode(true)).toBe("smooth");
    expect(getLogsFollowOutputMode(false)).toBe(false);
  });

  test("renders the chat empty state without reviving header controls", async () => {
    installTauriStub();
    const [{ MessagePanel }, { useBridgeStore }] = await Promise.all([
      import("./index"),
      import("@/stores/bridge-store"),
    ]);

    useBridgeStore.setState((state) => ({
      ...state,
      messages: [],
      terminalLines: [],
      permissionPrompts: [],
    }));

    const html = renderToStaticMarkup(
      <MessagePanel
        surfaceMode="chat"
        searchOpen={false}
        onSearchClose={() => {}}
      />,
    );

    expect(html).toContain(
      "No messages yet. Connect Claude and Codex to start bridging.",
    );
    expect(html).not.toContain("Approvals");
    expect(html).not.toContain(">Logs<");
    expect(html).not.toContain("Runtime logs");
  });

  test("default chat surface has no search input visible", async () => {
    installTauriStub();
    const { MessagePanel } = await import("./index");

    // Zustand v5 SSR uses getInitialState() (messages=[]).
    // Either way, search input must not appear by default (searchOpen starts false).
    const html = renderToStaticMarkup(
      <MessagePanel
        surfaceMode="chat"
        searchOpen={false}
        onSearchClose={() => {}}
      />,
    );

    expect(html).not.toContain('type="search"');
  });

  test("SearchRow renders input, close button, and summary only when provided", async () => {
    installTauriStub();
    const { SearchRow } = await import("./index");

    const inputRef = { current: null };
    const withSummary = renderToStaticMarkup(
      createElement(SearchRow, {
        searchQuery: "fix",
        searchSummary: "1 result for fix.",
        inputRef,
        onQueryChange: () => {},
        onClose: () => {},
      }),
    );
    const withoutSummary = renderToStaticMarkup(
      createElement(SearchRow, {
        searchQuery: "",
        searchSummary: null,
        inputRef,
        onQueryChange: () => {},
        onClose: () => {},
      }),
    );

    expect(withSummary).toContain("1 result for fix.");
    expect(withSummary).toContain('aria-label="Close search"');
    expect(withoutSummary).not.toContain("result");
    expect(withoutSummary).toContain('aria-label="Close search"');
  });

  test("search disclosure row appears only when searchOpen is true", async () => {
    installTauriStub();
    const { MessagePanel } = await import("./index");

    // Resting state: MessagePanel does not own the search icon — that lives in ShellTopBar.
    const closedHtml = renderToStaticMarkup(
      <MessagePanel
        surfaceMode="chat"
        searchOpen={false}
        onSearchClose={() => {}}
      />,
    );
    expect(closedHtml).not.toContain('aria-label="Search messages"');
    expect(closedHtml).not.toContain('type="search"');

    // Disclosed state: the search input row appears.
    const openHtml = renderToStaticMarkup(
      <MessagePanel
        surfaceMode="chat"
        searchOpen={true}
        onSearchClose={() => {}}
      />,
    );
    expect(openHtml).toContain('type="search"');
  });

  test("renders the log surface body without duplicating the top bar title", async () => {
    installTauriStub();
    const { MessagePanel } = await import("./index");

    const html = renderToStaticMarkup(
      <MessagePanel
        surfaceMode="logs"
        searchOpen={false}
        onSearchClose={() => {}}
      />,
    );

    // Chat surface stays mounted (hidden via CSS) so scroll position +
    // stream state survive logs-tab trips; verify via class rather than
    // content, since the chat empty-state markup is also present but
    // not visible.
    expect(html).toContain("No logs.");
    expect(html).toContain("hidden");
    expect(html).not.toContain("Runtime logs");
    expect(html).not.toContain(">Clear<");
  });
});
