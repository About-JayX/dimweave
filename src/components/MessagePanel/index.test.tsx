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

    const html = renderToStaticMarkup(<MessagePanel surfaceMode="chat" />);

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
    const html = renderToStaticMarkup(<MessagePanel surfaceMode="chat" />);

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

  test("chat surface always shows header search icon in chat mode", async () => {
    installTauriStub();
    const { MessagePanel } = await import("./index");

    // Zustand v5 SSR uses api.getInitialState() (messages=[]) — not getState().
    // The search chrome must render unconditionally in chat mode so it is
    // verifiable in the SSR snapshot regardless of message count.
    const html = renderToStaticMarkup(<MessagePanel surfaceMode="chat" />);
    expect(html).toContain('aria-label="Search messages"');
  });

  test("renders the log surface body without duplicating the top bar title", async () => {
    installTauriStub();
    const { MessagePanel } = await import("./index");

    const html = renderToStaticMarkup(<MessagePanel surfaceMode="logs" />);

    expect(html).toContain("No logs.");
    expect(html).not.toContain(
      "No messages yet. Connect Claude and Codex to start bridging.",
    );
    expect(html).not.toContain("Runtime logs");
    expect(html).not.toContain(">Clear<");
  });
});
