import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { BridgeState } from "./types";
import {
  type AgentMessagePayload,
  type AgentStatusPayload,
  type ClaudeStreamPayload,
  type CodexStreamPayload,
  type PermissionPromptPayload,
  type RuntimeHealthPayload,
  type SystemLogPayload,
} from "./listener-payloads";
import {
  clearPendingClaudePreview,
  clearPendingCodexStream,
  createPendingStreamUpdates,
  flushPendingStreamUpdates,
  hasPendingStreamUpdates,
  queueClaudePreviewUpdate,
  queueCodexBufferedUpdate,
} from "./stream-batching";
import {
  handleClaudeStreamEvent,
  handleCodexStreamEvent,
  resetClaudeStream,
} from "./stream-reducers";

type BridgeSetter = (fn: (state: BridgeState) => Partial<BridgeState>) => void;

type NextLogId = () => number;

export function reduceAgentStatus(
  state: BridgeState,
  payload: AgentStatusPayload,
): Partial<BridgeState> {
  const { agent, online, providerSession, role } = payload;
  const roleUpdate: Partial<BridgeState> =
    online && role
      ? agent === "claude"
        ? { claudeRole: role }
        : agent === "codex"
          ? { codexRole: role }
          : {}
      : {};
  return {
    agents: {
      ...state.agents,
      [agent]: {
        ...state.agents[agent],
        name: agent,
        displayName: state.agents[agent]?.displayName ?? agent,
        status: online ? ("connected" as const) : ("disconnected" as const),
        providerSession: online ? providerSession : undefined,
      },
    },
    ...(agent === "claude" && !online
      ? { claudeStream: resetClaudeStream(state) }
      : {}),
    ...roleUpdate,
  };
}

export function reducePermissionPrompt(
  state: BridgeState,
  payload: PermissionPromptPayload,
): Partial<BridgeState> {
  return {
    permissionPrompts: [
      ...state.permissionPrompts.filter(
        (prompt) => prompt.requestId !== payload.requestId,
      ),
      payload,
    ],
    permissionError: null,
  };
}

export function createBridgeListeners(
  set: BridgeSetter,
  nextLogId: NextLogId,
): Promise<UnlistenFn[]> {
  const pendingStreamUpdates = createPendingStreamUpdates();
  let flushTimer: ReturnType<typeof setTimeout> | null = null;
  const cancelPendingFlush = () => {
    if (flushTimer === null) return;
    clearTimeout(flushTimer);
    flushTimer = null;
  };

  const flushPendingStreams = () => {
    flushTimer = null;
    if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
      return;
    }
    set((s) => flushPendingStreamUpdates(s, pendingStreamUpdates));
  };

  const schedulePendingFlush = () => {
    if (flushTimer !== null) return;
    flushTimer = setTimeout(flushPendingStreams, 32);
  };

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
    listen<AgentStatusPayload>("agent_status", (e) => {
      const { agent, online } = e.payload;
      if (agent === "claude" && !online) {
        clearPendingClaudePreview(pendingStreamUpdates);
        if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
          cancelPendingFlush();
        }
      }
      if (agent === "codex" && !online) {
        clearPendingCodexStream(pendingStreamUpdates);
        if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
          cancelPendingFlush();
        }
      }
      set((s) => reduceAgentStatus(s, e.payload));
    }),
    listen<ClaudeStreamPayload>("claude_stream", (e) => {
      if (queueClaudePreviewUpdate(pendingStreamUpdates, e.payload)) {
        schedulePendingFlush();
        return;
      }
      clearPendingClaudePreview(pendingStreamUpdates);
      if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
        cancelPendingFlush();
      }
      set((s) => handleClaudeStreamEvent(s, e.payload));
    }),
    listen<CodexStreamPayload>("codex_stream", (e) => {
      if (queueCodexBufferedUpdate(pendingStreamUpdates, e.payload)) {
        schedulePendingFlush();
        return;
      }
      if (e.payload.kind === "thinking") {
        clearPendingCodexStream(pendingStreamUpdates);
        if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
          cancelPendingFlush();
        }
      } else {
        flushPendingStreams();
      }
      set((s) => handleCodexStreamEvent(s, e.payload));
    }),
    listen<PermissionPromptPayload>("permission_prompt", (e) => {
      set((s) => reducePermissionPrompt(s, e.payload));
    }),
    listen<RuntimeHealthPayload>("runtime_health", (e) => {
      set(() => ({
        runtimeHealth: e.payload.health ?? null,
      }));
    }),
  ]).then((fns) => [...fns, cancelPendingFlush]);
}
