import { describe, expect, test } from "bun:test";
import {
  getSearchQueryForDisclosure,
  getMessageListDisplayState,
  isMessageSearchActive,
  getMessageListFollowOutputMode,
  shouldResetMessageListInitialScroll,
} from "./view-model";

describe("getSearchQueryForDisclosure", () => {
  test("closing the disclosure clears any hidden search query", () => {
    expect(getSearchQueryForDisclosure(false, "error")).toBe("");
  });

  test("open disclosure preserves the active query", () => {
    expect(getSearchQueryForDisclosure(true, "error")).toBe("error");
  });
});

describe("getMessageListDisplayState", () => {
  test("adds one inline Claude draft item when Claude is thinking", () => {
    const state = getMessageListDisplayState({
      messageCount: 2,
      hasClaudeDraft: true,
      streamRailIndicators: ["codex"],
    });

    expect(state.timelineCount).toBe(3);
    expect(state.streamRailIndicators).toEqual(["codex"]);
    expect(state.hasContent).toBe(true);
  });

  test("does not inflate timeline count for footer-only codex indicators", () => {
    const state = getMessageListDisplayState({
      messageCount: 2,
      hasClaudeDraft: false,
      streamRailIndicators: ["codex"],
    });

    expect(state.timelineCount).toBe(2);
  });
});

describe("isMessageSearchActive", () => {
  test("non-empty trimmed query is active", () => {
    expect(isMessageSearchActive("error")).toBe(true);
  });

  test("whitespace-only query is not active", () => {
    expect(isMessageSearchActive("   ")).toBe(false);
  });

  test("empty string is not active", () => {
    expect(isMessageSearchActive("")).toBe(false);
  });
});

describe("getMessageListFollowOutputMode", () => {
  test("active search disables message-list follow output", () => {
    expect(getMessageListFollowOutputMode(true, true)).toBe(false);
  });

  test("active search disables follow even when not at bottom", () => {
    expect(getMessageListFollowOutputMode(true, false)).toBe(false);
  });

  test("inactive search at bottom enables smooth follow", () => {
    expect(getMessageListFollowOutputMode(false, true)).toBe("smooth");
  });

  test("inactive search not at bottom disables follow", () => {
    expect(getMessageListFollowOutputMode(false, false)).toBe(false);
  });
});

describe("shouldResetMessageListInitialScroll", () => {
  test("zero-result search does not reset initial scroll state", () => {
    expect(shouldResetMessageListInitialScroll(true, 0)).toBe(false);
  });

  test("inactive search with zero results resets initial scroll", () => {
    expect(shouldResetMessageListInitialScroll(false, 0)).toBe(true);
  });

  test("inactive search with messages does not reset", () => {
    expect(shouldResetMessageListInitialScroll(false, 5)).toBe(false);
  });

  test("active search with messages does not reset", () => {
    expect(shouldResetMessageListInitialScroll(true, 5)).toBe(false);
  });
});
