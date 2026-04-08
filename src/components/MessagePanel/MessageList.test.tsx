import { describe, expect, test, mock } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import {
  filterMessagesByQuery,
  getMessageSearchSummary,
  getMessageListDisplayState,
} from "./view-model";

// Module-level mocks — must precede any dynamic import of components
// that consume these modules so Bun's registry serves the mock version.

// 1. Minimal mutable bridge store (only fields MessageList + indicators read)
let _bs = {
  claudeStream: { thinking: false, previewText: "", thinkingText: "", blockType: "idle" as const, toolName: "", lastUpdatedAt: 0 },
  codexStream: {
    thinking: false, currentDelta: "", lastMessage: "",
    turnStatus: "", activity: "", reasoning: "", commandOutput: "",
  },
};
const _store = Object.assign((sel: (s: typeof _bs) => unknown) => sel(_bs), {
  getState: () => _bs,
  setState: (up: typeof _bs | ((s: typeof _bs) => typeof _bs)) => {
    _bs = typeof up === "function" ? { ..._bs, ...up(_bs) } : { ..._bs, ...up };
  },
  subscribe: () => () => {},
});
mock.module("@/stores/bridge-store", () => ({ useBridgeStore: _store }));

// 2. Fake Virtuoso: renders all items synchronously (bypasses SSR item-skip)
mock.module("react-virtuoso", () => ({
  Virtuoso: ({ totalCount, itemContent, components, context }: {
    totalCount: number;
    itemContent: (i: number) => unknown;
    components?: { Footer?: React.ComponentType<{ context?: unknown }> };
    context?: unknown;
  }) => {
    const Footer = components?.Footer;
    return createElement(
      "div", null,
      ...Array.from({ length: totalCount }, (_, i) =>
        createElement("div", { key: i }, itemContent(i) as React.ReactNode),
      ),
      Footer ? createElement(Footer, { context }) : null,
    );
  },
}));

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
    const state = getMessageListDisplayState({
      messageCount: 3,
      hasClaudeDraft: false,
      streamRailIndicators: ["codex"],
    });
    expect(state.timelineCount).toBe(3);
  });

  test("hasContent is true when only stream indicators are active", () => {
    const state = getMessageListDisplayState({
      messageCount: 0,
      hasClaudeDraft: false,
      streamRailIndicators: ["codex"],
    });
    expect(state.hasContent).toBe(true);
    expect(state.timelineCount).toBe(0);
  });

  test("StreamTailFooter renders container when indicators present and nothing when empty", async () => {
    installTauriStub();
    const { StreamTailFooter } = await import("./MessageList");

    const withIndicator = renderToStaticMarkup(
      createElement(StreamTailFooter, { context: { indicators: ["codex"] } }),
    );
    const withoutIndicator = renderToStaticMarkup(
      createElement(StreamTailFooter, { context: { indicators: [] } }),
    );

    expect(withIndicator).not.toBe(""); // tail container renders for codex
    expect(withoutIndicator).toBe(""); // nothing when no indicators
  });

  test("renders Claude working draft inline when only stream state is active", async () => {
    installTauriStub();
    const [{ MessageList }, { useBridgeStore }] = await Promise.all([
      import("./MessageList"),
      import("@/stores/bridge-store"),
    ]);

    useBridgeStore.setState((state) => ({
      ...state,
      claudeStream: {
        thinking: true,
        previewText: "Reviewing the daemon event path",
        thinkingText: "",
        blockType: "text" as const,
        toolName: "",
        lastUpdatedAt: 1,
      },
    }));

    const html = renderToStaticMarkup(<MessageList messages={[]} />);

    expect(html).toContain("Reviewing the daemon event path");
    expect(html).toContain("writing");
  });

  // GREEN: Regression guard for the draft-to-final handoff (post-fix).
  //
  // Fix (Tasks 2 + 4): route_message() now delivers the final bubble BEFORE
  // ClaudeStreamPayload.Done fires, so when the draft row clears the final
  // message is already present in the messages list.
  //
  // Post-fix state:
  //   - claudeStream is idle (done cleared the draft)
  //   - messages contains the final bubble (route_message ran before Done)
  test("renders the final Claude bubble after the draft row clears", async () => {
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
        thinkingText: "",
        blockType: "idle" as const,
        toolName: "",
        lastUpdatedAt: 2,
      },
    }));

    // Post-fix: route_message() ran before Done, so the final message is here.
    const finalMessage = {
      id: "msg_final",
      from: "claude",
      to: "user",
      content: "Final report delivered to the user.",
      timestamp: 2,
    };
    const html = renderToStaticMarkup(<MessageList messages={[finalMessage]} />);

    expect(html).toContain("Final report delivered to the user.");
    expect(html).not.toContain("writing");
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
        thinkingText: "",
        blockType: "idle" as const,
        toolName: "",
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
