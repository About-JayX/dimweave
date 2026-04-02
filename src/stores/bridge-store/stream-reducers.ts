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
    lastUpdatedAt: Date.now(),
  };
}

export function handleClaudeStreamEvent(
  state: BridgeState,
  payload: ClaudeStreamPayload,
): Partial<BridgeState> {
  switch (payload.kind) {
    case "thinkingStarted":
      return {
        claudeStream: {
          thinking: true,
          previewText: "",
          lastUpdatedAt: Date.now(),
        },
      };
    case "preview":
      return {
        claudeStream: {
          thinking: true,
          previewText: (
            state.claudeStream.previewText + (payload.text ?? "")
          ).slice(-MAX_CLAUDE_PREVIEW_CHARS),
          lastUpdatedAt: Date.now(),
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
