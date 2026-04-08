import { describe, expect, test } from "bun:test";
import {
  filterRenderableChatMessages,
  getMessageIdentityPresentation,
  getClaudeAttentionResolution,
  getMessageListDisplayState,
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

describe("getMessageListDisplayState", () => {
  test("keeps the virtualized timeline count tied to persisted messages only", () => {
    const state = getMessageListDisplayState({
      messageCount: 2,
      hasClaudeDraft: false,
      streamRailIndicators: ["claude", "codex"],
    });

    expect(state.timelineCount).toBe(2);
    expect(state.streamRailIndicators).toEqual(["claude", "codex"]);
    expect(state.hasContent).toBe(true);
  });

  test("treats active stream indicators as content even before the first persisted message", () => {
    const state = getMessageListDisplayState({
      messageCount: 0,
      hasClaudeDraft: false,
      streamRailIndicators: ["claude"],
    });

    expect(state.timelineCount).toBe(0);
    expect(state.streamRailIndicators).toEqual(["claude"]);
    expect(state.hasContent).toBe(true);
  });
});

describe("getClaudeAttentionResolution", () => {
  test("clears store attention while already on the messages tab", () => {
    expect(getClaudeAttentionResolution("messages", true)).toEqual({
      nextTab: null,
      clearStoreAttention: true,
    });
  });

  test("switches back to messages and clears store attention from other tabs", () => {
    expect(getClaudeAttentionResolution("logs", true)).toEqual({
      nextTab: "messages",
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
