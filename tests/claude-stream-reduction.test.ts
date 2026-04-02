import { describe, expect, test } from "bun:test";
import { handleClaudeStreamEvent } from "../src/stores/bridge-store/stream-reducers";

describe("handleClaudeStreamEvent", () => {
  test("accumulates Claude preview payloads into bounded preview text", () => {
    const state = {
      claudeStream: {
        thinking: true,
        previewText: "",
        lastUpdatedAt: 1,
      },
    } as any;

    expect(
      handleClaudeStreamEvent(state, {
        kind: "preview",
        text: "preview",
      }),
    ).toMatchObject({
      claudeStream: {
        thinking: true,
        previewText: "preview",
      },
    });
  });

  test("caps Claude preview text to the latest 5000 chars", () => {
    const state = {
      claudeStream: {
        thinking: true,
        previewText: "a".repeat(4_999),
        lastUpdatedAt: 1,
      },
    } as any;

    const next = handleClaudeStreamEvent(state, {
      kind: "preview",
      text: "bc",
    }) as any;

    expect(next.claudeStream.previewText).toHaveLength(5_000);
    expect(next.claudeStream.previewText.endsWith("bc")).toBe(true);
  });
});
