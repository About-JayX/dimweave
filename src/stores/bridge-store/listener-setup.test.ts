import { describe, expect, test } from "bun:test";
import type { BridgeState } from "./types";
import { handleCodexStreamEvent } from "./stream-reducers";
import type { CodexStreamPayload } from "./listener-payloads";

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
      thinking: false,
      previewText: "",
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

function applyCodexEvent(
  state: BridgeState,
  payload: CodexStreamPayload,
): BridgeState {
  const partial = handleCodexStreamEvent(state, payload);
  return {
    ...state,
    ...partial,
    codexStream: partial.codexStream ?? state.codexStream,
  };
}

describe("handleCodexStreamEvent", () => {
  test("stores activity labels and clears stale command output", () => {
    const state = baseState();
    state.codexStream.commandOutput = "old output";

    const next = applyCodexEvent(state, {
      kind: "activity",
      label: "Running: ls -la",
    });

    expect(next.codexStream.activity).toBe("Running: ls -la");
    expect(next.codexStream.commandOutput).toBe("");
  });

  test("accumulates command output and clears transient content on turn completion", () => {
    let state = baseState();
    state = applyCodexEvent(state, {
      kind: "commandOutput",
      text: "line 1\n",
    });
    state = applyCodexEvent(state, {
      kind: "commandOutput",
      text: "line 2\n",
    });

    expect(state.codexStream.commandOutput).toBe("line 1\nline 2\n");

    state = applyCodexEvent(state, {
      kind: "turnDone",
      status: "completed",
    });

    expect(state.codexStream.thinking).toBe(false);
    expect(state.codexStream.activity).toBe("");
    expect(state.codexStream.reasoning).toBe("");
    expect(state.codexStream.commandOutput).toBe("");
  });
});
