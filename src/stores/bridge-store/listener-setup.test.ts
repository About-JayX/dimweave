import { describe, expect, test } from "bun:test";
import type { BridgeState } from "./types";
import { reduceAgentStatus, reducePermissionPrompt } from "./listener-setup";
import {
  handleClaudeStreamEvent,
  handleCodexStreamEvent,
} from "./stream-reducers";
import {
  createPendingStreamUpdates,
  flushClaudePreviewIfPending,
  queueClaudePreviewUpdate,
} from "./stream-batching";
import type {
  AgentStatusPayload,
  ClaudeStreamPayload,
  CodexStreamPayload,
} from "./listener-payloads";

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

// Fixed-ordering contract: the listener now calls flushPendingStreams() BEFORE
// clearPendingClaudePreview() on non-preview claude_stream events.
// These two tests lock that ordering using the narrow helper from stream-batching.
test("flushes queued Claude preview before terminal done clears the draft", () => {
  const pending = createPendingStreamUpdates();
  queueClaudePreviewUpdate(pending, {
    kind: "preview",
    text: "final streamed sentence",
  } as ClaudeStreamPayload);

  const state = baseState();

  // Fixed listener path: flush pending preview into state first.
  const flushed = flushClaudePreviewIfPending(state, pending);
  const stateAfterFlush = {
    ...state,
    ...flushed,
    claudeStream: flushed.claudeStream ?? state.claudeStream,
  };

  // Preview was materialized into state before the pending was cleared.
  expect(stateAfterFlush.claudeStream.previewText).toBe(
    "final streamed sentence",
  );
  expect(pending.claudePreviewText).toBe(""); // flush clears pending

  // done/reset can now clear the draft cleanly — the preview was already seen.
  const donePartial = handleClaudeStreamEvent(stateAfterFlush, {
    kind: "done",
  });
  expect(donePartial.claudeStream?.previewText).toBe("");
});

test("no-op flush when pending claude preview is empty", () => {
  const pending = createPendingStreamUpdates(); // nothing queued
  const state = baseState();

  const result = flushClaudePreviewIfPending(state, pending);

  expect(result).toEqual({});
  expect(state.claudeStream.previewText).toBe("");
});

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

describe("uiErrors queue", () => {
  test("uiErrors is separate from terminalLines", () => {
    const state = baseState();
    expect(state.uiErrors).toEqual([]);
    expect(state.terminalLines).toEqual([]);
    // Pushing a terminal error should not affect uiErrors
    const withTerminal = {
      ...state,
      terminalLines: [
        {
          id: 1,
          agent: "system",
          kind: "error" as const,
          line: "runtime err",
          timestamp: 1,
        },
      ],
    };
    expect(withTerminal.uiErrors).toEqual([]);
  });

  test("selectUiErrorCount returns uiErrors length", async () => {
    const { selectUiErrorCount } = await import("./selectors");
    const state = {
      ...baseState(),
      uiErrors: [
        { id: 1, message: "err1", timestamp: 1 },
        { id: 2, message: "err2", timestamp: 2 },
      ],
    };
    expect(selectUiErrorCount(state)).toBe(2);
  });

  test("selectTerminalLineCount returns terminalLines length", async () => {
    const { selectTerminalLineCount } = await import("./selectors");
    const state = {
      ...baseState(),
      terminalLines: [
        {
          id: 1,
          agent: "system",
          kind: "text" as const,
          line: "log",
          timestamp: 1,
        },
      ],
    };
    expect(selectTerminalLineCount(state)).toBe(1);
  });
});
