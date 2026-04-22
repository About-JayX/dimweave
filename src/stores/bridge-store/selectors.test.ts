import { describe, expect, test } from "bun:test";
import {
  makeActiveTaskMessagesSelector,
  makeActiveClaudeStreamSelector,
  makeActiveCodexStreamSelector,
  selectTotalMessageCount,
} from "./selectors";
import type { BridgeState } from "./types";
import { GLOBAL_MESSAGE_BUCKET } from "./types";

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

function makeAgentMessage(
  id: string,
  timestamp: number,
  message: string,
  attachments: BridgeState["messagesByTask"][string][number]["attachments"] = [],
) {
  return {
    id,
    source: {
      kind: "agent" as const,
      agentId: "claude",
      role: "lead",
      provider: "claude" as const,
    },
    target: { kind: "user" as const },
    message,
    attachments,
    timestamp,
  };
}

function makeSystemMessage(id: string, timestamp: number, message: string) {
  return {
    id,
    source: { kind: "system" as const },
    target: { kind: "user" as const },
    message,
    timestamp,
  };
}

describe("makeActiveTaskMessagesSelector", () => {
  test("returns a stable merged reference when task and global buckets are unchanged", () => {
    const state = baseState();
    state.messagesByTask = {
      [GLOBAL_MESSAGE_BUCKET]: [makeSystemMessage("global", 1, "system note")],
      task_a: [makeAgentMessage("agent", 2, "hello")],
    };
    const sel = makeActiveTaskMessagesSelector("task_a");

    const first = sel(state);
    const second = sel(state);

    expect(first.map((msg) => msg.id)).toEqual(["global", "agent"]);
    expect(first).toBe(second);
  });
});

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

describe("selectTotalMessageCount", () => {
  test("counts only renderable chat messages across all buckets", () => {
    const state = baseState();
    state.messagesByTask = {
      [GLOBAL_MESSAGE_BUCKET]: [makeSystemMessage("sys", 1, "daemon started")],
      task_a: [makeAgentMessage("visible", 2, "hello world")],
      task_b: [
        makeAgentMessage("attachment-only", 3, "   ", [
          {
            filePath: "/tmp/report.png",
            fileName: "report.png",
            isImage: true,
          },
        ]),
      ],
      task_c: [makeAgentMessage("empty", 4, "   ")],
    };

    expect(selectTotalMessageCount(state)).toBe(2);
  });
});
