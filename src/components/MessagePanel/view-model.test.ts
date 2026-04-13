import { describe, expect, test } from "bun:test";
import {
  getSearchQueryForDisclosure,
  getMessageListDisplayState,
  isMessageSearchActive,
  getMessageListFollowOutputMode,
  shouldClearStickyOnScroll,
  shouldResetMessageListInitialScroll,
  shouldScrollOnDraftStart,
  STICKY_BOTTOM_THRESHOLD,
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
  test("search active disables follow regardless of sticky state", () => {
    expect(getMessageListFollowOutputMode(true, true)).toBe(false);
    expect(getMessageListFollowOutputMode(true, false)).toBe(false);
  });

  test("sticky mode enables smooth follow", () => {
    expect(getMessageListFollowOutputMode(false, true)).toBe("smooth");
  });

  test("user-scrolled-away (not sticky) disables follow", () => {
    // With the Claude-style refactor, the second parameter is a ref-backed
    // sticky flag set only by wheel events, not by Virtuoso atBottomStateChange.
    // When the user explicitly scrolls away, follow must stop.
    expect(getMessageListFollowOutputMode(false, false)).toBe(false);
  });
});

describe("shouldClearStickyOnScroll", () => {
  test("upward scroll beyond threshold with no immunity clears sticky", () => {
    expect(
      shouldClearStickyOnScroll(true, STICKY_BOTTOM_THRESHOLD + 1, false),
    ).toBe(true);
  });

  test("upward scroll beyond threshold during immunity window keeps sticky", () => {
    expect(
      shouldClearStickyOnScroll(true, STICKY_BOTTOM_THRESHOLD + 1, true),
    ).toBe(false);
  });

  test("downward or flat scroll never clears sticky regardless of immunity", () => {
    expect(
      shouldClearStickyOnScroll(false, STICKY_BOTTOM_THRESHOLD + 1, false),
    ).toBe(false);
    expect(
      shouldClearStickyOnScroll(false, STICKY_BOTTOM_THRESHOLD + 1, true),
    ).toBe(false);
  });

  test("upward scroll within threshold keeps sticky even without immunity", () => {
    expect(
      shouldClearStickyOnScroll(true, STICKY_BOTTOM_THRESHOLD - 1, false),
    ).toBe(false);
  });
});

describe("shouldScrollOnDraftStart", () => {
  test("draft active + sticky + no search → should scroll to bottom", () => {
    expect(shouldScrollOnDraftStart(true, false, true)).toBe(true);
  });

  test("draft not started → should not scroll", () => {
    expect(shouldScrollOnDraftStart(false, false, true)).toBe(false);
  });

  test("search active during draft → should not scroll", () => {
    expect(shouldScrollOnDraftStart(true, true, true)).toBe(false);
  });

  test("user scrolled away (not sticky) during draft → should not scroll", () => {
    expect(shouldScrollOnDraftStart(true, false, false)).toBe(false);
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
