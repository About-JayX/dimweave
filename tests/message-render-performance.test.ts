import { describe, expect, test } from "bun:test";
import { areMessageBubblePropsEqual } from "../src/components/MessagePanel/MessageBubble";
import { prepareMessageContent } from "../src/components/MessageMarkdown";

describe("areMessageBubblePropsEqual", () => {
  test("treats stable message data as memo-safe even across wrapper rerenders", () => {
    const prev = {
      msg: {
        id: "1",
        source: { kind: "agent" as const, agentId: "claude", role: "lead", provider: "claude" as const, displaySource: "claude" },
        target: { kind: "user" as const },
        content: "same content",
        timestamp: 123,
      },
    };
    const next = {
      msg: {
        id: "1",
        source: { kind: "agent" as const, agentId: "claude", role: "lead", provider: "claude" as const, displaySource: "claude" },
        target: { kind: "user" as const },
        content: "same content",
        timestamp: 123,
      },
    };

    expect(areMessageBubblePropsEqual(prev as any, next as any)).toBe(true);
  });

  test("forces rerender when the visible message payload changes", () => {
    const prev = {
      msg: {
        id: "1",
        source: { kind: "agent" as const, agentId: "claude", role: "lead", provider: "claude" as const },
        target: { kind: "user" as const },
        content: "before",
        timestamp: 123,
      },
    };
    const next = {
      msg: {
        id: "1",
        source: { kind: "agent" as const, agentId: "claude", role: "lead", provider: "claude" as const },
        target: { kind: "user" as const },
        content: "after",
        timestamp: 123,
      },
    };

    expect(areMessageBubblePropsEqual(prev as any, next as any)).toBe(false);
  });
});

describe("prepareMessageContent", () => {
  test("routes plain text through the lightweight render path", () => {
    expect(prepareMessageContent("Plain text only\nsecond line")).toEqual({
      cleaned: "Plain text only\nsecond line",
      renderMode: "plain",
    });
  });

  test("keeps markdown content on the markdown path", () => {
    expect(prepareMessageContent("## Heading\n- item")).toEqual({
      cleaned: "## Heading\n- item",
      renderMode: "markdown",
    });
  });

  test("keeps inline strong markdown on the markdown path for Claude role intros", () => {
    expect(prepareMessageContent("你好！我是 **lead**（协调者），目前在线。")).toEqual({
      cleaned: "你好！我是 **lead**（协调者），目前在线。",
      renderMode: "markdown",
    });
  });

  test("strips escapes before deciding the render path", () => {
    expect(prepareMessageContent("\u001b[31mHello\u001b[0m")).toEqual({
      cleaned: "Hello",
      renderMode: "plain",
    });
  });
});
