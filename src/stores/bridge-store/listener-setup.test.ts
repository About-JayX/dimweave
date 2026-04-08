import { describe, expect, test } from "bun:test";
import type { BridgeState } from "./types";
import { reduceAgentStatus, reducePermissionPrompt } from "./listener-setup";
import { handleCodexStreamEvent } from "./stream-reducers";
import type {
  AgentStatusPayload,
  CodexStreamPayload,
} from "./listener-payloads";

function baseState(): BridgeState {
  return {
    connected: true,
    messages: [],
    agents: {},
    terminalLines: [],
    permissionPrompts: [],
    permissionError: null,
    runtimeHealth: null,
    claudeNeedsAttention: false,
    claudeRole: "lead",
    codexRole: "coder",
    claudeStream: {
      thinking: false,
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
  test("clears stale permission errors when a new prompt arrives", () => {
    const state = baseState();
    state.permissionError = {
      requestId: "req_old",
      message: "Previous approval failed",
    };

    const next = reducePermissionPrompt(state, {
      requestId: "req_new",
      toolName: "shell",
      description: "Allow command",
      inputPreview: "ls -la",
      agent: "claude",
      createdAt: 123,
    });

    expect(next.permissionError).toBeNull();
    expect(next.permissionPrompts).toEqual([
      {
        requestId: "req_new",
        toolName: "shell",
        description: "Allow command",
        inputPreview: "ls -la",
        agent: "claude",
        createdAt: 123,
      },
    ]);
  });

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

describe("reduceAgentStatus role sync", () => {
  function applyAgentStatus(
    state: BridgeState,
    payload: AgentStatusPayload,
  ): BridgeState {
    const partial = reduceAgentStatus(state, payload);
    return { ...state, ...partial };
  }

  test("updates codexRole when codex agent-status arrives online with role", () => {
    const state = baseState();
    const next = applyAgentStatus(state, {
      agent: "codex",
      online: true,
      role: "lead",
    });
    expect(next.codexRole).toBe("lead");
  });

  test("updates claudeRole when claude agent-status arrives online with role", () => {
    const state = baseState();
    const next = applyAgentStatus(state, {
      agent: "claude",
      online: true,
      role: "coder",
    });
    expect(next.claudeRole).toBe("coder");
  });

  test("does not change role when agent-status has no role field", () => {
    const state = { ...baseState(), codexRole: "coder" };
    const next = applyAgentStatus(state, {
      agent: "codex",
      online: true,
    });
    expect(next.codexRole).toBe("coder");
  });

  test("does not update role on offline status", () => {
    const state = { ...baseState(), claudeRole: "lead" };
    const next = applyAgentStatus(state, {
      agent: "claude",
      online: false,
      role: "coder",
    });
    expect(next.claudeRole).toBe("lead");
  });
});
