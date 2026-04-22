import { afterEach, beforeEach, describe, expect, mock, test } from "bun:test";
import { forwardRef, useEffect, useImperativeHandle, useRef } from "react";
import { createRoot, type Root } from "react-dom/client";
import { setupDOM, teardownDOM } from "../TaskPanel/dom-test-env";

let bridgeState = {
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
  claudeStreamsByTask: {} as Record<string, Record<string, unknown>>,
  codexStreamsByTask: {} as Record<string, Record<string, unknown>>,
};

const bridgeStore = Object.assign(
  (selector: (state: typeof bridgeState) => unknown) => selector(bridgeState),
  {
    getState: () => bridgeState,
    setState: (
      next:
        | typeof bridgeState
        | ((state: typeof bridgeState) => typeof bridgeState),
    ) => {
      bridgeState =
        typeof next === "function" ? { ...bridgeState, ...next(bridgeState) } : { ...bridgeState, ...next };
    },
    subscribe: () => () => {},
  },
);

let taskState = {
  activeTaskId: null as string | null,
};

const taskStore = Object.assign(
  (selector: (state: typeof taskState) => unknown) => selector(taskState),
  {
    getState: () => taskState,
    setState: (
      next:
        | typeof taskState
        | ((state: typeof taskState) => typeof taskState),
    ) => {
      taskState =
        typeof next === "function" ? { ...taskState, ...next(taskState) } : { ...taskState, ...next };
    },
    subscribe: () => () => {},
  },
);

let scrollToIndexCalls: Array<{ index: string; behavior?: string }> = [];
let scrollerScrollCalls: Array<{ top?: number; behavior?: string }> = [];
let currentScrollHeight = 3200;
let lastScroller: HTMLElement | null = null;
let rafId = 0;
let cancelledRafs = new Set<number>();
let rafQueue: Array<{ id: number; callback: FrameRequestCallback }> = [];

mock.module("@/stores/bridge-store", () => ({ useBridgeStore: bridgeStore }));
mock.module("@/stores/task-store", () => ({ useTaskStore: taskStore }));
mock.module("./MessageBubble", () => ({
  MessageBubble: ({ msg }: { msg: { id: string; message: string } }) => (
    <div data-msg-id={msg.id}>{msg.message}</div>
  ),
  MessageImageLightbox: () => null,
}));
mock.module("./CodexStreamIndicator", () => ({
  CodexStreamIndicator: () => <div>codex-stream</div>,
}));
mock.module("./ClaudeStreamIndicator", () => ({
  ClaudeStreamIndicator: () => <div>claude-stream</div>,
}));
mock.module("react-virtuoso", () => ({
  Virtuoso: forwardRef(function VirtuosoMock(props: any, ref) {
    const scrollerRef = useRef<HTMLElement | null>(null);
    if (!scrollerRef.current) {
      const scroller = document.createElement("div");
      Object.defineProperty(scroller, "scrollHeight", {
        configurable: true,
        get: () => currentScrollHeight,
      });
      (scroller as any).scrollTo = (options: { top?: number; behavior?: string }) => {
        scrollerScrollCalls.push(options);
      };
      scrollerRef.current = scroller;
      lastScroller = scroller;
    }

    useImperativeHandle(ref, () => ({
      scrollToIndex: (options: { index: string; behavior?: string }) => {
        scrollToIndexCalls.push(options);
      },
    }));

    useEffect(() => {
      props.scrollerRef?.(scrollerRef.current);
      return () => props.scrollerRef?.(null);
    }, [props.scrollerRef]);

    return (
      <div data-virtuoso="true">
        {Array.from({ length: props.totalCount }, (_, index) => (
          <div key={index}>{props.itemContent(index)}</div>
        ))}
      </div>
    );
  }),
}));

function resetStores() {
  bridgeState = {
    claudeStream: {
      thinking: false,
      previewText: "",
      thinkingText: "",
      blockType: "idle",
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
    claudeStreamsByTask: {},
    codexStreamsByTask: {},
  };
  taskState = { activeTaskId: null };
  scrollToIndexCalls = [];
  scrollerScrollCalls = [];
  currentScrollHeight = 3200;
  lastScroller = null;
  rafId = 0;
  cancelledRafs = new Set();
  rafQueue = [];
}

function makeMessage(id: string, message: string, timestamp: number) {
  return {
    id,
    source: {
      kind: "agent" as const,
      agentId: "claude",
      role: "lead",
      provider: "claude" as const,
    },
    target: { kind: "user" as const },
    message,
    timestamp,
  };
}

function flush() {
  return new Promise((resolve) => setTimeout(resolve, 20));
}

async function flushRafFrame() {
  const frame = rafQueue;
  rafQueue = [];
  for (const entry of frame) {
    if (cancelledRafs.has(entry.id)) continue;
    entry.callback(0);
  }
  await flush();
}

async function flushRafFrames(count: number) {
  for (let index = 0; index < count; index += 1) {
    if (rafQueue.length === 0) break;
    await flushRafFrame();
  }
}

function dispatchWheelUp(scroller: HTMLElement) {
  const event = new window.Event("wheel");
  Object.defineProperty(event, "deltaY", {
    configurable: true,
    value: -120,
  });
  scroller.dispatchEvent(event);
}

function dispatchScrollbarDragScroll(scroller: HTMLElement) {
  scroller.dispatchEvent(new window.Event("mousedown", { bubbles: true }));
  scroller.dispatchEvent(new window.Event("scroll", { bubbles: true }));
}

let root: Root | null = null;
let container: HTMLDivElement | null = null;

beforeEach(() => {
  setupDOM();
  resetStores();
  Object.assign(window, {
    requestAnimationFrame: (callback: FrameRequestCallback) => {
      const id = ++rafId;
      rafQueue.push({ id, callback });
      return id;
    },
    cancelAnimationFrame: (id: number) => {
      cancelledRafs.add(id);
    },
  });
  container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
});

afterEach(() => {
  root?.unmount();
  root = null;
  container?.remove();
  container = null;
  teardownDOM();
  resetStores();
});

describe("MessageList interaction", () => {
  test("switching to another non-empty task re-aligns the viewport to the absolute bottom", async () => {
    const { MessageList } = await import("./MessageList");

    taskStore.setState({ activeTaskId: "task_a" });
    root!.render(
      <MessageList
        messages={[makeMessage("a1", "task A message", 1)]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(2);

    scrollToIndexCalls = [];
    scrollerScrollCalls = [];
    currentScrollHeight = 6400;

    taskStore.setState({ activeTaskId: "task_b" });
    root!.render(
      <MessageList
        messages={[
          makeMessage("b1", "task B first", 1),
          makeMessage("b2", "task B second", 2),
          makeMessage("b3", "task B third", 3),
        ]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(2);

    expect(scrollToIndexCalls).toContainEqual({
      index: "LAST",
      behavior: "auto",
    });
    expect(scrollerScrollCalls).toContainEqual({ top: 6400 });
  });

  test("task switch clears a carried-over back-to-bottom latch", async () => {
    const { MessageList } = await import("./MessageList");

    taskStore.setState({ activeTaskId: "task_a" });
    root!.render(
      <MessageList
        messages={[
          makeMessage("a1", "task A first", 1),
          makeMessage("a2", "task A second", 2),
        ]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(2);

    expect(lastScroller).toBeTruthy();
    dispatchWheelUp(lastScroller!);
    await flush();
    expect(container?.textContent).toContain("Back to bottom");

    taskStore.setState({ activeTaskId: "task_b" });
    root!.render(
      <MessageList
        messages={[makeMessage("b1", "task B only", 3)]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(2);

    expect(container?.textContent).not.toContain("Back to bottom");
  });

  test("task switch keeps pinning while the new transcript height is still settling", async () => {
    const { MessageList } = await import("./MessageList");

    taskStore.setState({ activeTaskId: "task_a" });
    root!.render(
      <MessageList
        messages={[makeMessage("a1", "task A message", 1)]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(2);

    scrollToIndexCalls = [];
    scrollerScrollCalls = [];
    currentScrollHeight = 6400;

    taskStore.setState({ activeTaskId: "task_b" });
    root!.render(
      <MessageList
        messages={[
          makeMessage("b1", "task B first", 1),
          makeMessage("b2", "task B second", 2),
          makeMessage("b3", "task B third", 3),
        ]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(2);
    expect(scrollerScrollCalls.at(-1)).toEqual({ top: 6400 });

    currentScrollHeight = 6800;
    await flushRafFrame();

    expect(scrollerScrollCalls.at(-1)).toEqual({ top: 6800 });
  });

  test("task switch does not repeatedly scroll to the same stable bottom target", async () => {
    const { MessageList } = await import("./MessageList");

    taskStore.setState({ activeTaskId: "task_a" });
    root!.render(
      <MessageList
        messages={[makeMessage("a1", "task A message", 1)]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(2);

    scrollerScrollCalls = [];
    currentScrollHeight = 6400;

    taskStore.setState({ activeTaskId: "task_b" });
    root!.render(
      <MessageList
        messages={[
          makeMessage("b1", "task B first", 1),
          makeMessage("b2", "task B second", 2),
          makeMessage("b3", "task B third", 3),
        ]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(4);

    const identicalBottomCalls = scrollerScrollCalls.filter(
      (call) => call.top === 6400,
    );
    expect(identicalBottomCalls).toHaveLength(1);
  });

  test("scrollbar drag during task-switch settling disables further auto-bottom nudges", async () => {
    const { MessageList } = await import("./MessageList");

    taskStore.setState({ activeTaskId: "task_a" });
    root!.render(
      <MessageList
        messages={[makeMessage("a1", "task A message", 1)]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrames(2);

    scrollerScrollCalls = [];
    currentScrollHeight = 6400;

    taskStore.setState({ activeTaskId: "task_b" });
    root!.render(
      <MessageList
        messages={[
          makeMessage("b1", "task B first", 1),
          makeMessage("b2", "task B second", 2),
          makeMessage("b3", "task B third", 3),
        ]}
        searchActive={false}
      />,
    );
    await flush();
    await flushRafFrame();
    expect(scrollerScrollCalls.at(-1)).toEqual({ top: 6400 });

    expect(lastScroller).toBeTruthy();
    dispatchScrollbarDragScroll(lastScroller!);
    await flush();
    expect(container?.textContent).toContain("Back to bottom");

    currentScrollHeight = 6800;
    await flushRafFrames(3);

    expect(scrollerScrollCalls.some((call) => call.top === 6800)).toBe(false);
  });
});
