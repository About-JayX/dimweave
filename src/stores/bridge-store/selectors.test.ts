import { describe, expect, test } from "bun:test";
import {
  makeActiveClaudeStreamSelector,
  makeActiveCodexStreamSelector,
} from "./selectors";
import type { BridgeState } from "./types";

function baseState(): BridgeState {
  return {
    connected: true,
    messagesByTask: {},
    agents: {},
    terminalLines: [],
    uiErrors: [],
    permissionPrompts: [],
    permissionError: null,
    runtimeHealth: null,
    claudeNeedsAttention: false,
    claudeRole: "",
    codexRole: "",
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
    draft: "",
    setDraft: () => {},
    clearClaudeAttention: () => {},
    sendToCodex: () => {},
    clearMessages: () => {},
    stopCodexTui: () => {},
    respondToPermission: async () => {},
    applyConfig: async () => {},
    pushUiError: () => {},
    clearUiErrors: () => {},
    setRole: () => {},
    cleanup: () => {},
  };
}

describe("makeActiveClaudeStreamSelector", () => {
  test("returns the per-task bucket for the given taskId", () => {
    const state = baseState();
    state.claudeStreamsByTask = {
      t1: {
        thinking: true,
        previewText: "hello",
        thinkingText: "",
        blockType: "text",
        toolName: "",
        lastUpdatedAt: 100,
      },
    };
    const sel = makeActiveClaudeStreamSelector("t1");
    expect(sel(state).previewText).toBe("hello");
    expect(sel(state).thinking).toBe(true);
  });

  test("returns a stable default when bucket is missing", () => {
    const state = baseState();
    const sel = makeActiveClaudeStreamSelector("missing");
    const a = sel(state);
    const b = sel(state);
    expect(a.previewText).toBe("");
    expect(a.thinking).toBe(false);
    expect(a).toBe(b);
  });

  test("returns singleton mirror when taskId is null (bootstrap race)", () => {
    const state = baseState();
    state.claudeStream = {
      thinking: true,
      previewText: "from singleton",
      thinkingText: "",
      blockType: "text",
      toolName: "",
      lastUpdatedAt: 1,
    };
    const sel = makeActiveClaudeStreamSelector(null);
    expect(sel(state).previewText).toBe("from singleton");
  });
});

describe("makeActiveCodexStreamSelector", () => {
  test("returns the per-task bucket for the given taskId", () => {
    const state = baseState();
    state.codexStreamsByTask = {
      t1: {
        thinking: true,
        currentDelta: "draft",
        lastMessage: "",
        turnStatus: "",
        activity: "",
        reasoning: "",
        commandOutput: "",
      },
    };
    const sel = makeActiveCodexStreamSelector("t1");
    expect(sel(state).currentDelta).toBe("draft");
  });

  test("returns stable default when bucket is missing", () => {
    const state = baseState();
    const sel = makeActiveCodexStreamSelector("missing");
    const a = sel(state);
    const b = sel(state);
    expect(a.currentDelta).toBe("");
    expect(a).toBe(b);
  });
});
