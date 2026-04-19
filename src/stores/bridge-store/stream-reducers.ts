import type { BridgeState } from "./types";
import type {
  ClaudeStreamPayload,
  CodexStreamPayload,
} from "./listener-payloads";

const MAX_CLAUDE_PREVIEW_CHARS = 5_000;
const MAX_CODEX_PREVIEW_CHARS = 100_000;

export function defaultClaudeStreamState(): BridgeState["claudeStream"] {
  return {
    thinking: false,
    previewText: "",
    thinkingText: "",
    blockType: "idle",
    toolName: "",
    lastUpdatedAt: 0,
  };
}

export function defaultCodexStreamState(): BridgeState["codexStream"] {
  return {
    thinking: false,
    currentDelta: "",
    lastMessage: "",
    turnStatus: "",
    activity: "",
    reasoning: "",
    commandOutput: "",
  };
}

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

/// Pure reducer: given a Claude stream slice and an event, return the next slice.
/// No dependencies on other BridgeState fields — callers decide which bucket to
/// apply it to (per-task bucket plus optional active-task mirror).
export function reduceClaudeStreamSlice(
  prev: BridgeState["claudeStream"],
  payload: ClaudeStreamPayload,
): BridgeState["claudeStream"] {
  const now = Date.now();
  switch (payload.kind) {
    case "thinkingStarted":
      return {
        ...prev,
        thinking: true,
        thinkingText: "",
        blockType: "thinking",
        lastUpdatedAt: now,
      };
    case "thinkingDelta":
      return {
        ...prev,
        thinking: true,
        thinkingText: (prev.thinkingText + (payload.text ?? "")).slice(
          -MAX_CLAUDE_PREVIEW_CHARS,
        ),
        blockType: "thinking",
        lastUpdatedAt: now,
      };
    case "textStarted":
      return {
        ...prev,
        thinking: true,
        previewText: "",
        blockType: "text",
        lastUpdatedAt: now,
      };
    case "textDelta":
      return {
        ...prev,
        thinking: true,
        previewText: (prev.previewText + (payload.text ?? "")).slice(
          -MAX_CLAUDE_PREVIEW_CHARS,
        ),
        blockType: "text",
        lastUpdatedAt: now,
      };
    case "toolStarted":
      return {
        ...prev,
        thinking: true,
        toolName: payload.name ?? "",
        blockType: "tool",
        lastUpdatedAt: now,
      };
    case "preview":
      if (prev.blockType === "text" && prev.previewText.length > 0) {
        return { ...prev, thinking: true, lastUpdatedAt: now };
      }
      return {
        ...prev,
        thinking: true,
        previewText: (prev.previewText + (payload.text ?? "")).slice(
          -MAX_CLAUDE_PREVIEW_CHARS,
        ),
        lastUpdatedAt: now,
      };
    case "done":
    case "reset":
      return {
        ...prev,
        thinking: false,
        previewText: "",
        thinkingText: "",
        blockType: "idle",
        toolName: "",
        lastUpdatedAt: now,
      };
    default:
      return prev;
  }
}

export function handleClaudeStreamEvent(
  state: BridgeState,
  payload: ClaudeStreamPayload,
): Partial<BridgeState> {
  return { claudeStream: reduceClaudeStreamSlice(state.claudeStream, payload) };
}

/// Pure reducer mirroring reduceClaudeStreamSlice — operates on a single
/// codex stream slice without touching other BridgeState fields.
export function reduceCodexStreamSlice(
  prev: BridgeState["codexStream"],
  payload: CodexStreamPayload,
): BridgeState["codexStream"] {
  switch (payload.kind) {
    case "thinking":
      return {
        ...prev,
        thinking: true,
        currentDelta: "",
        turnStatus: "",
        activity: "",
        reasoning: "",
        commandOutput: "",
      };
    case "activity":
      return {
        ...prev,
        activity: payload.label ?? "",
        commandOutput: "",
      };
    case "reasoning":
      return {
        ...prev,
        reasoning: (payload.text ?? "").slice(-MAX_CODEX_PREVIEW_CHARS),
      };
    case "commandOutput":
      return {
        ...prev,
        commandOutput: prev.commandOutput + (payload.text ?? ""),
      };
    case "delta":
      return {
        ...prev,
        currentDelta: (payload.text ?? "").slice(-MAX_CODEX_PREVIEW_CHARS),
      };
    case "message":
      return {
        ...prev,
        lastMessage: payload.text ?? "",
        currentDelta: "",
      };
    case "turnDone":
      return {
        thinking: false,
        currentDelta: "",
        lastMessage: "",
        turnStatus: payload.status ?? "",
        activity: "",
        reasoning: "",
        commandOutput: "",
      };
    default:
      return prev;
  }
}

export function handleCodexStreamEvent(
  state: BridgeState,
  payload: CodexStreamPayload,
): Partial<BridgeState> {
  return { codexStream: reduceCodexStreamSlice(state.codexStream, payload) };
}
