import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useTaskStore } from "@/stores/task-store";
import type { BridgeState } from "./types";
import {
  type AgentMessagePayload,
  type AgentStatusPayload,
  type ClaudeStreamEvent,
  type CodexStreamEvent,
  type PermissionPromptPayload,
  type RuntimeHealthPayload,
  type SystemLogPayload,
} from "./listener-payloads";

/// Decide whether a stream event should mutate the singleton stream state.
///
/// - If the daemon stamped a taskId and it matches the currently active
///   task, apply the event.
/// - If the daemon did not stamp a taskId (legacy / truly global), apply
///   it as before — no filtering regression on pre-migration emit sites.
/// - If the stamped taskId belongs to a different task, drop the event:
///   the other task's Reasoning indicator must not paint over ours. When
///   the user switches back, the next stream event for that task will
///   re-seed the singleton state.
function shouldApplyStreamEventToActiveTask(taskId?: string): boolean {
  if (!taskId) return true;
  const activeId = useTaskStore.getState().activeTaskId;
  return !activeId || activeId === taskId;
}
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
  resetCodexStream,
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

  // Reset singleton stream state when the user switches tasks. Without this,
  // Task A's Thinking indicator keeps showing on Task B until Task B emits
  // its first stream event — a stale-state glitch. Resubscribe returns an
  // unsubscribe fn that cleans up with the other listeners.
  const unsubscribeTaskSwitch = useTaskStore.subscribe((state, prev) => {
    if (state.activeTaskId !== prev.activeTaskId) {
      clearPendingClaudePreview(pendingStreamUpdates);
      clearPendingCodexStream(pendingStreamUpdates);
      set((s) => ({
        claudeStream: resetClaudeStream(s),
        codexStream: resetCodexStream(s),
      }));
    }
  });
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
    listen<ClaudeStreamEvent>("claude_stream", (e) => {
      if (!shouldApplyStreamEventToActiveTask(e.payload.taskId)) return;
      const payload = e.payload.payload;
      if (queueClaudePreviewUpdate(pendingStreamUpdates, payload)) {
        schedulePendingFlush();
        return;
      }
      // Flush any queued preview into state BEFORE clearing pending,
      // so the last preview chunk is visible before done/reset clears the draft.
      flushPendingStreams();
      clearPendingClaudePreview(pendingStreamUpdates);
      if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
        cancelPendingFlush();
      }
      set((s) => handleClaudeStreamEvent(s, payload));
    }),
    listen<CodexStreamEvent>("codex_stream", (e) => {
      if (!shouldApplyStreamEventToActiveTask(e.payload.taskId)) return;
      const payload = e.payload.payload;
      if (queueCodexBufferedUpdate(pendingStreamUpdates, payload)) {
        schedulePendingFlush();
        return;
      }
      if (payload.kind === "thinking") {
        clearPendingCodexStream(pendingStreamUpdates);
        if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
          cancelPendingFlush();
        }
      } else {
        flushPendingStreams();
      }
      set((s) => handleCodexStreamEvent(s, payload));
    }),
    listen<PermissionPromptPayload>("permission_prompt", (e) => {
      set((s) => reducePermissionPrompt(s, e.payload));
    }),
    listen<RuntimeHealthPayload>("runtime_health", (e) => {
      set(() => ({
        runtimeHealth: e.payload.health ?? null,
      }));
    }),
  ]).then((fns) => [...fns, cancelPendingFlush, unsubscribeTaskSwitch]);
}
