import { describe, expect, test } from "bun:test";
import {
  getSearchQueryForDisclosure,
  getMessageListDisplayState,
  isMessageSearchActive,
  shouldScrollOnStreamTail,
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

describe("shouldScrollOnStreamTail", () => {
  test("claude draft + at-bottom + no search → nudge", () => {
    expect(shouldScrollOnStreamTail(true, false, false, true)).toBe(true);
  });

  test("codex stream visible + at-bottom + no search → nudge", () => {
    expect(shouldScrollOnStreamTail(false, true, false, true)).toBe(true);
  });

  test("both claude and codex active → nudge", () => {
    expect(shouldScrollOnStreamTail(true, true, false, true)).toBe(true);
  });

  test("neither active → no nudge", () => {
    expect(shouldScrollOnStreamTail(false, false, false, true)).toBe(false);
  });

  test("search active suppresses nudge regardless of stream state", () => {
    expect(shouldScrollOnStreamTail(true, true, true, true)).toBe(false);
  });

  test("user scrolled away suppresses nudge", () => {
    expect(shouldScrollOnStreamTail(true, true, false, false)).toBe(false);
  });
});
