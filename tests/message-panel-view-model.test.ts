import { describe, expect, test } from "bun:test";
import {
  filterRenderableChatMessages,
  getMessageIdentityPresentation,
  getClaudeTerminalPlaceholder,
  getClaudeAttentionResolution,
  getTransientIndicators,
} from "../src/components/MessagePanel/view-model";

describe("filterRenderableChatMessages", () => {
  test("drops system and whitespace-only messages", () => {
    const messages = [
      {
        id: "1",
        from: "system",
        to: "user",
        content: "system notice",
        timestamp: 1,
      },
      {
        id: "2",
        from: "claude",
        to: "user",
        content: "   \n\t",
        timestamp: 2,
      },
      {
        id: "3",
        from: "codex",
        to: "user",
        content: "visible",
        timestamp: 3,
      },
    ];

    expect(filterRenderableChatMessages(messages as any)).toEqual([
      messages[2],
    ]);
  });
});

describe("getMessageIdentityPresentation", () => {
  test("uses display source for badge color while keeping role visible", () => {
    expect(
      getMessageIdentityPresentation({
        id: "1",
        from: "coder",
        displaySource: "claude",
        to: "user",
        content: "done",
        timestamp: 1,
      } as any),
    ).toEqual({
      badgeSource: "claude",
      roleLabel: "coder",
    });
  });

  test("falls back to from when there is no separate display source", () => {
    expect(
      getMessageIdentityPresentation({
        id: "2",
        from: "user",
        to: "lead",
        content: "hello",
        timestamp: 1,
      } as any),
    ).toEqual({
      badgeSource: "user",
      roleLabel: null,
    });
  });
});

describe("getTransientIndicators", () => {
  test("keeps Claude before Codex when both are active", () => {
    expect(
      getTransientIndicators(
        { thinking: true, previewText: "preview", lastUpdatedAt: 1 },
        {
          thinking: true,
          currentDelta: "delta",
          lastMessage: "",
          turnStatus: "",
        },
      ),
    ).toEqual(["claude", "codex"]);
  });

  test("omits inactive indicators", () => {
    expect(
      getTransientIndicators(
        { thinking: false, previewText: "", lastUpdatedAt: 0 },
        {
          thinking: false,
          currentDelta: "",
          lastMessage: "",
          turnStatus: "",
        },
      ),
    ).toEqual([]);
  });

  test("ignores Claude preview-only state and only shows active thinking", () => {
    expect(
      getTransientIndicators(
        { thinking: false, previewText: "garbled preview", lastUpdatedAt: 1 },
        {
          thinking: false,
          currentDelta: "",
          lastMessage: "",
          turnStatus: "",
        },
      ),
    ).toEqual([]);
  });
});

describe("getClaudeAttentionResolution", () => {
  test("clears store attention while already on claude tab", () => {
    expect(getClaudeAttentionResolution("claude", true)).toEqual({
      nextTab: null,
      clearStoreAttention: true,
    });
  });

  test("switches to claude tab and clears store attention from other tabs", () => {
    expect(getClaudeAttentionResolution("messages", true)).toEqual({
      nextTab: "claude",
      clearStoreAttention: true,
    });
  });

  test("does nothing when there is no pending attention", () => {
    expect(getClaudeAttentionResolution("logs", false)).toEqual({
      nextTab: null,
      clearStoreAttention: false,
    });
  });
});

describe("getClaudeTerminalPlaceholder", () => {
  test("shows a waiting hint when Claude is connected but no terminal output arrived yet", () => {
    expect(getClaudeTerminalPlaceholder(true, false, 0)).toBe(
      "Claude is connected. Waiting for terminal output…",
    );
  });

  test("shows startup hint while terminal is launching with no chunks", () => {
    expect(getClaudeTerminalPlaceholder(false, true, 0)).toBe(
      "Claude terminal is starting. Waiting for output…",
    );
  });

  test("returns idle hint only when Claude is fully inactive", () => {
    expect(getClaudeTerminalPlaceholder(false, false, 0)).toBe(
      "Claude terminal is idle. Connect Claude to start an embedded session.",
    );
  });

  test("returns null once terminal output exists", () => {
    expect(getClaudeTerminalPlaceholder(true, true, 2)).toBeNull();
  });
});
