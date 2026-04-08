import { describe, expect, test } from "bun:test";
import {
  getSearchQueryForDisclosure,
  getMessageListDisplayState,
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
