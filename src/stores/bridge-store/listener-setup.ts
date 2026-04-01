import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BridgeState } from "./types";
import {
  type AgentMessagePayload,
  type AgentStatusPayload,
  type ClaudeStreamPayload,
  type ClaudeTerminalDataPayload,
  type ClaudeTerminalStatusPayload,
  type CodexStreamPayload,
  type PermissionPromptPayload,
  type SystemLogPayload,
} from "./listener-payloads";

const MAX_CODEX_PREVIEW_CHARS = 100_000;

type BridgeSetter = (fn: (state: BridgeState) => Partial<BridgeState>) => void;

type NextLogId = () => number;

export function createBridgeListeners(
  set: BridgeSetter,
  nextLogId: NextLogId,
): Promise<UnlistenFn[]> {
  return Promise.all([
    listen<AgentMessagePayload>("agent_message", (e) => {
      set((s) => ({
        messages: [...s.messages.slice(-999), e.payload.payload],
      }));
    }),
    listen<SystemLogPayload>("system_log", (e) => {
      const { level, message } = e.payload;
      set((s) => ({
        terminalLines: [
          ...s.terminalLines.slice(-200),
          {
            id: nextLogId(),
            agent: "system",
            kind: level === "error" ? ("error" as const) : ("text" as const),
            line: message,
            timestamp: Date.now(),
          },
        ],
      }));
    }),
    listen<ClaudeTerminalDataPayload>("claude_terminal_data", (e) => {
      set((s) => ({
        claudeTerminalChunks: [
          ...s.claudeTerminalChunks.slice(-999),
          { id: nextLogId(), data: e.payload.data, timestamp: Date.now() },
        ],
      }));
    }),
    listen("claude_terminal_reset", () => {
      set((s) => ({
        claudeTerminalChunks: [],
        claudeTerminalExitCode: undefined,
        claudeTerminalDetail: undefined,
        claudeStream: resetClaudeStream(s),
      }));
    }),
    listen<ClaudeTerminalStatusPayload>("claude_terminal_status", (e) => {
      set(() => ({
        claudeTerminalRunning: e.payload.running,
        claudeTerminalExitCode: e.payload.exitCode,
        claudeTerminalDetail: e.payload.detail,
      }));
    }),
    listen<AgentStatusPayload>("agent_status", (e) => {
      const { agent, online, providerSession } = e.payload;
      set((s) => ({
        agents: {
          ...s.agents,
          [agent]: {
            ...s.agents[agent],
            name: agent,
            displayName: s.agents[agent]?.displayName ?? agent,
            status: online ? ("connected" as const) : ("disconnected" as const),
            providerSession: online ? providerSession : undefined,
          },
        },
        ...(agent === "claude" && !online
          ? { claudeStream: resetClaudeStream(s) }
          : {}),
      }));
    }),
    listen("claude_terminal_attention", () => {
      set((s) => ({
        claudeNeedsAttention: true,
        claudeFocusNonce: s.claudeFocusNonce + 1,
      }));
    }),
    listen<ClaudeStreamPayload>("claude_stream", (e) => {
      set((s) => handleClaudeStreamEvent(s, e.payload));
    }),
    listen<CodexStreamPayload>("codex_stream", (e) => {
      set((s) => handleCodexStreamEvent(s, e.payload));
    }),
    listen<PermissionPromptPayload>("permission_prompt", (e) => {
      set((s) => ({
        permissionPrompts: [
          ...s.permissionPrompts.filter(
            (prompt) => prompt.requestId !== e.payload.requestId,
          ),
          e.payload,
        ],
      }));
    }),
  ]);
}

function resetClaudeStream(state: BridgeState): BridgeState["claudeStream"] {
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
      return {};
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
          // daemon sends the full normalized preview on each delta update
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
