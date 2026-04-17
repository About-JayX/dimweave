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
//    Also captures the latest props so tests can assert on followOutput, etc.
let lastVirtuosoProps: Record<string, unknown> | null = null;
mock.module("react-virtuoso", () => ({
  Virtuoso: (props: {
    totalCount: number;
    itemContent: (i: number) => unknown;
    components?: { Footer?: React.ComponentType<{ context?: unknown }> };
    context?: unknown;
    followOutput?: unknown;
    [key: string]: unknown;
  }) => {
    lastVirtuosoProps = props;
    const { totalCount, itemContent, components, context } = props;
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

describe("getDraftScrollStrategy", () => {
  // Distinguishes the two scroll semantics used by the draft anchor effect:
  //  "scroller-bottom" — scrollTo(scrollHeight) pins to absolute content bottom
  //  "last-index"      — scrollToIndex("LAST") only ensures item is in viewport
  test("when scroller element exists, strategy is scroller-bottom", async () => {
    installTauriStub();
    const { getDraftScrollStrategy } = await import("./MessageList");
    expect(getDraftScrollStrategy(true)).toBe("scroller-bottom");
  });

  test("when scroller element is absent (SSR / unmounted), strategy is last-index", async () => {
    const { getDraftScrollStrategy } = await import("./MessageList");
    expect(getDraftScrollStrategy(false)).toBe("last-index");
  });
});

describe("MessageList", () => {
  test("filters long sessions by message content and attachment names", () => {
    const filtered = filterMessagesByQuery(
      [
        {
          id: "msg_1",
          source: { kind: "agent", agentId: "claude", role: "lead", provider: "claude" },
          target: { kind: "user" },
          message: "Created the rollout plan",
          timestamp: 1,
        },
        {
          id: "msg_2",
          source: { kind: "agent", agentId: "codex", role: "coder", provider: "codex" },
          target: { kind: "user" },
          message: "Attached the latest screenshot",
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
      source: { kind: "agent" as const, agentId: "claude", role: "lead", provider: "claude" as const },
      target: { kind: "user" as const },
      message: "Final report delivered to the user.",
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

  test("followOutput is a function that returns false during search", async () => {
    installTauriStub();
    const [{ MessageList }, { useBridgeStore }] = await Promise.all([
      import("./MessageList"),
      import("@/stores/bridge-store"),
    ]);
    useBridgeStore.setState((state) => ({
      ...state,
      claudeStream: {
        thinking: false, previewText: "", thinkingText: "",
        blockType: "idle" as const, toolName: "", lastUpdatedAt: 0,
      },
      codexStream: {
        thinking: false, currentDelta: "", lastMessage: "",
        turnStatus: "", activity: "", reasoning: "", commandOutput: "",
      },
    }));

    lastVirtuosoProps = null;
    renderToStaticMarkup(
      <MessageList
        messages={[{
          id: "msg_1", source: { kind: "agent" as const, agentId: "claude", role: "lead", provider: "claude" as const }, target: { kind: "user" as const },
          message: "Found the root cause", timestamp: 1,
        }]}
        searchActive={true}
      />,
    );

    expect(lastVirtuosoProps).not.toBeNull();
    const followOutput = lastVirtuosoProps!.followOutput as () => false | "smooth";
    expect(typeof followOutput).toBe("function");
    expect(followOutput()).toBe(false);
  });

  test("followOutput function returns smooth when sticky (initial state)", async () => {
    installTauriStub();
    const [{ MessageList }, { useBridgeStore }] = await Promise.all([
      import("./MessageList"),
      import("@/stores/bridge-store"),
    ]);
    useBridgeStore.setState((state) => ({
      ...state,
      claudeStream: {
        thinking: false, previewText: "", thinkingText: "",
        blockType: "idle" as const, toolName: "", lastUpdatedAt: 0,
      },
      codexStream: {
        thinking: false, currentDelta: "", lastMessage: "",
        turnStatus: "", activity: "", reasoning: "", commandOutput: "",
      },
    }));

    lastVirtuosoProps = null;
    renderToStaticMarkup(
      <MessageList
        messages={[{
          id: "msg_1", source: { kind: "agent" as const, agentId: "claude", role: "lead", provider: "claude" as const }, target: { kind: "user" as const },
          message: "Found the root cause", timestamp: 1,
        }]}
        searchActive={false}
      />,
    );

    expect(lastVirtuosoProps).not.toBeNull();
    const followOutput = lastVirtuosoProps!.followOutput as () => false | "smooth";
    expect(typeof followOutput).toBe("function");
    expect(followOutput()).toBe("smooth");
  });

  test("draft row renders at totalCount = messages.length + 1 and followOutput stays smooth", async () => {
    installTauriStub();
    const [{ MessageList }, { useBridgeStore }] = await Promise.all([
      import("./MessageList"),
      import("@/stores/bridge-store"),
    ]);
    useBridgeStore.setState((state) => ({
      ...state,
      claudeStream: {
        thinking: true,
        previewText: "Drafting reply…",
        thinkingText: "",
        blockType: "text" as const,
        toolName: "",
        lastUpdatedAt: 1,
      },
      codexStream: {
        thinking: false, currentDelta: "", lastMessage: "",
        turnStatus: "", activity: "", reasoning: "", commandOutput: "",
      },
    }));

    lastVirtuosoProps = null;
    renderToStaticMarkup(
      <MessageList
        messages={[{
          id: "msg_1", source: { kind: "user" as const }, target: { kind: "agent" as const, agentId: "claude" },
          message: "Start streaming", timestamp: 1,
        }]}
        searchActive={false}
      />,
    );

    // totalCount should be messages.length + 1 (the inline draft row).
    expect(lastVirtuosoProps).not.toBeNull();
    expect(lastVirtuosoProps!.totalCount).toBe(2);
    // followOutput must still return "smooth" — draft start must not lose sticky.
    const followOutput = lastVirtuosoProps!.followOutput as () => false | "smooth";
    expect(followOutput()).toBe("smooth");
    // search frozen: draft active + search → no follow
    const { MessageList: ML2 } = await import("./MessageList");
    lastVirtuosoProps = null;
    renderToStaticMarkup(
      <ML2
        messages={[{
          id: "msg_1", source: { kind: "user" as const }, target: { kind: "agent" as const, agentId: "claude" },
          message: "Start streaming", timestamp: 1,
        }]}
        searchActive={true}
      />,
    );
    expect(lastVirtuosoProps).not.toBeNull();
    const followOutputSearch = lastVirtuosoProps!.followOutput as () => false | "smooth";
    expect(followOutputSearch()).toBe(false);
  });

  test("content growth (atBottomStateChange false) does not disable auto-follow", async () => {
    installTauriStub();
    const [{ MessageList }, { useBridgeStore }] = await Promise.all([
      import("./MessageList"),
      import("@/stores/bridge-store"),
    ]);
    useBridgeStore.setState((state) => ({
      ...state,
      claudeStream: {
        thinking: false, previewText: "", thinkingText: "",
        blockType: "idle" as const, toolName: "", lastUpdatedAt: 0,
      },
      codexStream: {
        thinking: false, currentDelta: "", lastMessage: "",
        turnStatus: "", activity: "", reasoning: "", commandOutput: "",
      },
    }));

    lastVirtuosoProps = null;
    renderToStaticMarkup(
      <MessageList
        messages={[{
          id: "msg_1", source: { kind: "agent" as const, agentId: "claude", role: "lead", provider: "claude" as const }, target: { kind: "user" as const },
          message: "Streaming content", timestamp: 1,
        }]}
        searchActive={false}
      />,
    );

    // Simulate Virtuoso reporting content-growth scroll-away (NOT user-initiated).
    // This must NOT clear sticky mode — only user interaction (wheel/pointer) should.
    const atBottomStateChange = lastVirtuosoProps!.atBottomStateChange as (b: boolean) => void;
    atBottomStateChange(false);

    const followOutput = lastVirtuosoProps!.followOutput as () => false | "smooth";
    expect(followOutput()).toBe("smooth");
  });
});
