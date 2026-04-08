import { describe, expect, test } from "bun:test";
import type { BridgeState } from "../src/stores/bridge-store/types";
import {
  createPendingStreamUpdates,
  flushPendingStreamUpdates,
  hasPendingStreamUpdates,
  queueClaudePreviewUpdate,
  queueCodexBufferedUpdate,
} from "../src/stores/bridge-store/stream-batching";

function baseState(): BridgeState {
  return {
    connected: true,
    messages: [],
    agents: {},
    terminalLines: [],
    permissionPrompts: [],
    claudeNeedsAttention: false,
    claudeRole: "lead",
    codexRole: "coder",
    claudeStream: {
      thinking: true,
      previewText: "",
      thinkingText: "",
      blockType: "idle",
      toolName: "",
      lastUpdatedAt: 0,
    },
    codexStream: {
      thinking: true,
      currentDelta: "",
      lastMessage: "",
      turnStatus: "",
      activity: "",
      reasoning: "",
      commandOutput: "",
    },
    draft: "",
    setDraft: () => {},
    clearClaudeAttention: () => {},
    sendToCodex: () => {},
    clearMessages: () => {},
    stopCodexTui: () => {},
    respondToPermission: async () => {},
    applyConfig: async () => {},
    setRole: () => {},
    cleanup: () => {},
  };
}

describe("stream batching", () => {
  test("coalesces Claude preview chunks into a single flush", () => {
    const pending = createPendingStreamUpdates();

    expect(
      queueClaudePreviewUpdate(pending, {
        kind: "preview",
        text: "hel",
      }),
    ).toBe(true);
    expect(
      queueClaudePreviewUpdate(pending, {
        kind: "preview",
        text: "lo",
      }),
    ).toBe(true);
    expect(hasPendingStreamUpdates(pending)).toBe(true);

    const partial = flushPendingStreamUpdates(baseState(), pending);

    expect(partial.claudeStream).toMatchObject({
      thinking: true,
      previewText: "hello",
    });
    expect(hasPendingStreamUpdates(pending)).toBe(false);
  });

  test("does not duplicate Claude text already applied from live text deltas", () => {
    const pending = createPendingStreamUpdates();
    const state = baseState();
    state.claudeStream = {
      ...state.claudeStream,
      previewText: "hello",
      blockType: "text",
    };

    expect(
      queueClaudePreviewUpdate(pending, {
        kind: "preview",
        text: "hello",
      }),
    ).toBe(true);

    const partial = flushPendingStreamUpdates(state, pending);

    expect(partial.claudeStream).toMatchObject({
      thinking: true,
      previewText: "hello",
      blockType: "text",
    });
    expect(hasPendingStreamUpdates(pending)).toBe(false);
  });

  test("flushes Codex buffered updates in a single state patch", () => {
    const pending = createPendingStreamUpdates();
    const state = baseState();
    state.codexStream.commandOutput = "stale";

    expect(
      queueCodexBufferedUpdate(pending, {
        kind: "activity",
        label: "Running: bun test",
      }),
    ).toBe(true);
    queueCodexBufferedUpdate(pending, {
      kind: "reasoning",
      text: "first draft",
    });
    queueCodexBufferedUpdate(pending, {
      kind: "reasoning",
      text: "latest reasoning",
    });
    queueCodexBufferedUpdate(pending, {
      kind: "commandOutput",
      text: "line 1\n",
    });
    queueCodexBufferedUpdate(pending, {
      kind: "commandOutput",
      text: "line 2\n",
    });
    queueCodexBufferedUpdate(pending, {
      kind: "delta",
      text: "streaming preview",
    });

    const partial = flushPendingStreamUpdates(state, pending);

    expect(partial.codexStream).toMatchObject({
      activity: "Running: bun test",
      reasoning: "latest reasoning",
      commandOutput: "line 1\nline 2\n",
      currentDelta: "streaming preview",
    });
    expect(hasPendingStreamUpdates(pending)).toBe(false);
  });
});
