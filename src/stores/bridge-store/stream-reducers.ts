import type { BridgeState } from "./types";
import type {
  ClaudeStreamPayload,
  CodexStreamPayload,
} from "./listener-payloads";

const MAX_CLAUDE_PREVIEW_CHARS = 5_000;
const MAX_CODEX_PREVIEW_CHARS = 100_000;

export function resetClaudeStream(
  state: BridgeState,
): BridgeState["claudeStream"] {
  return {
    ...state.claudeStream,
    thinking: false,
    previewText: "",
    thinkingText: "",
    blockType: "idle",
    toolName: "",
    lastUpdatedAt: Date.now(),
  };
}

export function resetCodexStream(
  state: BridgeState,
): BridgeState["codexStream"] {
  return {
    ...state.codexStream,
    thinking: false,
    currentDelta: "",
    lastMessage: "",
    turnStatus: "",
    activity: "",
    reasoning: "",
    commandOutput: "",
  };
}

export function handleClaudeStreamEvent(
  state: BridgeState,
  payload: ClaudeStreamPayload,
): Partial<BridgeState> {
  const now = Date.now();
  switch (payload.kind) {
    case "thinkingStarted":
      return {
        claudeStream: {
          ...state.claudeStream,
          thinking: true,
          thinkingText: "",
          blockType: "thinking",
          lastUpdatedAt: now,
        },
      };
    case "thinkingDelta":
      return {
        claudeStream: {
          ...state.claudeStream,
          thinking: true,
          thinkingText: (
            state.claudeStream.thinkingText + (payload.text ?? "")
          ).slice(-MAX_CLAUDE_PREVIEW_CHARS),
          blockType: "thinking",
          lastUpdatedAt: now,
        },
      };
    case "textStarted":
      return {
        claudeStream: {
          ...state.claudeStream,
          thinking: true,
          previewText: "",
          blockType: "text",
          lastUpdatedAt: now,
        },
      };
    case "textDelta":
      return {
        claudeStream: {
          ...state.claudeStream,
          thinking: true,
          previewText: (
            state.claudeStream.previewText + (payload.text ?? "")
          ).slice(-MAX_CLAUDE_PREVIEW_CHARS),
          blockType: "text",
          lastUpdatedAt: now,
        },
      };
    case "toolStarted":
      return {
        claudeStream: {
          ...state.claudeStream,
          thinking: true,
          toolName: payload.name ?? "",
          blockType: "tool",
          lastUpdatedAt: now,
        },
      };
    case "preview":
      // Legacy batched preview — still used by batching layer
      if (
        state.claudeStream.blockType === "text" &&
        state.claudeStream.previewText.length > 0
      ) {
        return {
          claudeStream: {
            ...state.claudeStream,
            thinking: true,
            lastUpdatedAt: now,
          },
        };
      }
      return {
        claudeStream: {
          ...state.claudeStream,
          thinking: true,
          previewText: (
            state.claudeStream.previewText + (payload.text ?? "")
          ).slice(-MAX_CLAUDE_PREVIEW_CHARS),
          lastUpdatedAt: now,
        },
      };
    case "done":
    case "reset":
      return { claudeStream: resetClaudeStream(state) };
    default:
      return {};
  }
}

export function handleCodexStreamEvent(
  state: BridgeState,
  payload: CodexStreamPayload,
): Partial<BridgeState> {
  switch (payload.kind) {
    case "thinking":
      return {
        codexStream: {
          ...state.codexStream,
          thinking: true,
          currentDelta: "",
          turnStatus: "",
          activity: "",
          reasoning: "",
          commandOutput: "",
        },
      };
    case "activity":
      return {
        codexStream: {
          ...state.codexStream,
          activity: payload.label ?? "",
          commandOutput: "",
        },
      };
    case "reasoning":
      return {
        codexStream: {
          ...state.codexStream,
          reasoning: (payload.text ?? "").slice(-MAX_CODEX_PREVIEW_CHARS),
        },
      };
    case "commandOutput":
      return {
        codexStream: {
          ...state.codexStream,
          commandOutput: state.codexStream.commandOutput + (payload.text ?? ""),
        },
      };
    case "delta":
      return {
        codexStream: {
          ...state.codexStream,
          currentDelta: (payload.text ?? "").slice(-MAX_CODEX_PREVIEW_CHARS),
        },
      };
    case "message":
      return {
        codexStream: {
          ...state.codexStream,
          lastMessage: payload.text ?? "",
          currentDelta: "",
        },
      };
    case "turnDone":
      return {
        codexStream: {
          thinking: false,
          currentDelta: "",
          lastMessage: "",
          turnStatus: payload.status ?? "",
          activity: "",
          reasoning: "",
          commandOutput: "",
        },
      };
    default:
      return {};
  }
}
