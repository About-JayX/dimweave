import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  filterMessagesByQuery,
  getMessageSearchSummary,
  getMessageListDisplayState,
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
          return null;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => {},
      },
      requestAnimationFrame: (callback: FrameRequestCallback) => {
        callback(0);
        return 1;
      },
      cancelAnimationFrame: () => {},
    },
  });
}

describe("MessageList", () => {
  test("filters long sessions by message content and attachment names", () => {
    const filtered = filterMessagesByQuery(
      [
        {
          id: "msg_1",
          from: "claude",
          to: "user",
          content: "Created the rollout plan",
          timestamp: 1,
        },
        {
          id: "msg_2",
          from: "codex",
          to: "user",
          content: "Attached the latest screenshot",
          timestamp: 2,
          attachments: [
            {
              filePath: "/tmp/review.png",
              fileName: "review.png",
              isImage: true,
            },
          ],
        },
      ],
      "review.png",
    );

    expect(filtered.map((message) => message.id)).toEqual(["msg_2"]);
    expect(getMessageSearchSummary("review.png", filtered.length)).toBe(
      "1 result for review.png.",
    );
  });

  test("stream indicators do not inflate timelineCount", () => {
    const state = getMessageListDisplayState(3, ["claude", "codex"]);
    expect(state.timelineCount).toBe(3);
  });

  test("hasContent is true when only stream indicators are active", () => {
    const state = getMessageListDisplayState(0, ["claude"]);
    expect(state.hasContent).toBe(true);
    expect(state.timelineCount).toBe(0);
  });

  test("StreamTailFooter renders container when indicators present and nothing when empty", async () => {
    installTauriStub();
    const { StreamTailFooter } = await import("./MessageList");

    const withIndicator = renderToStaticMarkup(
      createElement(StreamTailFooter, { context: { indicators: ["claude"] } }),
    );
    const withoutIndicator = renderToStaticMarkup(
      createElement(StreamTailFooter, { context: { indicators: [] } }),
    );

    expect(withIndicator).not.toBe(""); // tail container renders when active
    expect(withoutIndicator).toBe(""); // nothing when no indicators
  });

  test("renders a search-specific empty state when no filtered messages remain", async () => {
    installTauriStub();
    const [{ MessageList }, { useBridgeStore }] = await Promise.all([
      import("./MessageList"),
      import("@/stores/bridge-store"),
    ]);
    useBridgeStore.setState((state) => ({
      ...state,
      claudeStream: {
        thinking: false,
        previewText: "",
        lastUpdatedAt: 0,
      },
      codexStream: {
        thinking: false,
        currentDelta: "",
        lastMessage: "",
        turnStatus: "",
        activity: "",
        reasoning: "",
        commandOutput: "",
      },
    }));

    const html = renderToStaticMarkup(
      <MessageList
        messages={[]}
        emptyStateMessage="No messages match rollout."
      />,
    );

    expect(html).toContain("No messages match rollout.");
  });
});
