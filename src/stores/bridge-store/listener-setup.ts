import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { useTaskStore } from "@/stores/task-store";
import type { BridgeMessage } from "@/types";
import type { BridgeState } from "./types";
import { GLOBAL_MESSAGE_BUCKET, MAX_MESSAGES_PER_BUCKET } from "./types";
import {
  type AgentMessagePayload,
  type AgentStatusPayload,
  type ClaudeStreamEvent,
  type CodexStreamEvent,
  type PermissionCancelledPayload,
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
  defaultClaudeStreamState,
  defaultCodexStreamState,
  handleClaudeStreamEvent,
  handleCodexStreamEvent,
  reduceClaudeStreamSlice,
  reduceCodexStreamSlice,
  resetClaudeStream,
} from "./stream-reducers";
import type {
  ClaudeStreamPayload,
  CodexStreamPayload,
} from "./listener-payloads";

/// Apply a Claude stream event to the per-task bucket. If the event belongs
/// to the currently active task, also mirror into the singleton so the UI
/// sees the update immediately.
function applyClaudeStreamToBucket(
  state: BridgeState,
  taskId: string | null,
  payload: ClaudeStreamPayload,
): Partial<BridgeState> {
  if (!taskId) return handleClaudeStreamEvent(state, payload);
  const prevBucket =
    state.claudeStreamsByTask[taskId] ?? defaultClaudeStreamState();
  const nextBucket = reduceClaudeStreamSlice(prevBucket, payload);
  const activeId = useTaskStore.getState().activeTaskId;
  return {
    claudeStreamsByTask: {
      ...state.claudeStreamsByTask,
      [taskId]: nextBucket,
    },
    ...(activeId === taskId ? { claudeStream: nextBucket } : {}),
  };
}

function applyCodexStreamToBucket(
  state: BridgeState,
  taskId: string | null,
  payload: CodexStreamPayload,
): Partial<BridgeState> {
  if (!taskId) return handleCodexStreamEvent(state, payload);
  const prevBucket =
    state.codexStreamsByTask[taskId] ?? defaultCodexStreamState();
  const nextBucket = reduceCodexStreamSlice(prevBucket, payload);
  const activeId = useTaskStore.getState().activeTaskId;
  return {
    codexStreamsByTask: {
      ...state.codexStreamsByTask,
      [taskId]: nextBucket,
    },
    ...(activeId === taskId ? { codexStream: nextBucket } : {}),
  };
}

/// Resolve the task id a stream event should be routed into.
///
/// - Explicit taskId from daemon envelope (post Step-2) wins.
/// - For legacy emit sites with no taskId, fall back to the currently
///   active task — the user-visible bucket.
function resolveStreamBucketId(taskId?: string): string | null {
  if (taskId) return taskId;
  return useTaskStore.getState().activeTaskId ?? null;
}

type BridgeSetter = (fn: (state: BridgeState) => Partial<BridgeState>) => void;

/// Replace `messages` with the persisted transcript for `taskId` so chat
/// survives app restarts. Live agent_message events append on top; we
/// dedupe by id in case the stream races with the DB read.
export async function hydrateMessagesForTask(
  taskId: string | null,
  set: BridgeSetter,
): Promise<void> {
  if (!taskId) return;
  try {
    const persisted = await invoke<BridgeMessage[] | null>(
      "daemon_list_task_messages",
      { taskId },
    );
    // Tauri commands typed as Option<Vec<_>> deserialize to null when empty;
    // guard here so the dedupe/merge below doesn't crash the reducer.
    const list = Array.isArray(persisted) ? persisted : [];
    set((s) => {
      const seen = new Set(list.map((m) => m.id));
      const liveBucket = s.messagesByTask[taskId] ?? [];
      const liveForTask = liveBucket.filter((m) => !seen.has(m.id));
      return {
        messagesByTask: {
          ...s.messagesByTask,
          [taskId]: [...list, ...liveForTask],
        },
      };
    });
  } catch (e) {
    // Best-effort — hydration failure keeps the in-memory timeline intact.
    console.error("[bridge-store] hydrate messages failed", e);
  }
}

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

  // On task switch, swap the singleton mirrors (claudeStream/codexStream) to
  // point at the new task's bucket. Keeps in-progress state around — events
  // that arrive while the user is on another task keep updating their task's
  // bucket, and we restore that state when the user returns.
  const unsubscribeTaskSwitch = useTaskStore.subscribe((state, prev) => {
    if (state.activeTaskId !== prev.activeTaskId) {
      clearPendingClaudePreview(pendingStreamUpdates);
      clearPendingCodexStream(pendingStreamUpdates);
      set((s) => {
        const nextId = state.activeTaskId;
        return {
          claudeStream: nextId
            ? (s.claudeStreamsByTask[nextId] ?? defaultClaudeStreamState())
            : defaultClaudeStreamState(),
          codexStream: nextId
            ? (s.codexStreamsByTask[nextId] ?? defaultCodexStreamState())
            : defaultCodexStreamState(),
        };
      });
      void hydrateMessagesForTask(state.activeTaskId, set);
    }
    // Task removal: sweep permission prompts and stream buckets tied to
    // tasks that no longer exist. Without this, deleted tasks leak state.
    const prevIds = Object.keys(prev.tasks);
    const nextIds = new Set(Object.keys(state.tasks));
    const removedIds = prevIds.filter((id) => !nextIds.has(id));
    if (removedIds.length > 0) {
      const removed = new Set(removedIds);
      set((s) => {
        const nextClaudeBuckets = { ...s.claudeStreamsByTask };
        const nextCodexBuckets = { ...s.codexStreamsByTask };
        const nextMessagesByTask = { ...s.messagesByTask };
        for (const id of removed) {
          delete nextClaudeBuckets[id];
          delete nextCodexBuckets[id];
          delete nextMessagesByTask[id];
        }
        return {
          // Drop chat messages tied to deleted tasks so the timeline
          // doesn't keep stale history around for a task that no longer
          // exists. GLOBAL_MESSAGE_BUCKET survives (system diagnostics).
          messagesByTask: nextMessagesByTask,
          permissionPrompts: s.permissionPrompts.filter(
            (p) => !p.taskId || !removed.has(p.taskId),
          ),
          claudeStreamsByTask: nextClaudeBuckets,
          codexStreamsByTask: nextCodexBuckets,
        };
      });
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
      // Route by msg.taskId into per-task bucket. Keeps other tasks'
      // bucket references stable — MessageList in a different task
      // does not re-render when this one appends.
      set((s) => {
        const msg = e.payload.payload;
        const tid = msg.taskId ?? GLOBAL_MESSAGE_BUCKET;
        const existing = s.messagesByTask[tid] ?? [];
        return {
          messagesByTask: {
            ...s.messagesByTask,
            [tid]: [...existing.slice(-(MAX_MESSAGES_PER_BUCKET - 1)), msg],
          },
        };
      });
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
      const taskId = resolveStreamBucketId(e.payload.taskId);
      const payload = e.payload.payload;
      // Batching is only safe for the active task's bucket — the pending
      // preview buffer is singleton. Inactive-task previews bypass the
      // 32ms coalescer so their bucket stays accurate.
      const activeId = useTaskStore.getState().activeTaskId;
      const isActive = taskId !== null && taskId === activeId;
      if (isActive && queueClaudePreviewUpdate(pendingStreamUpdates, payload)) {
        schedulePendingFlush();
        return;
      }
      // Flush any queued preview into state BEFORE clearing pending,
      // so the last preview chunk is visible before done/reset clears the draft.
      if (isActive) {
        flushPendingStreams();
        clearPendingClaudePreview(pendingStreamUpdates);
        if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
          cancelPendingFlush();
        }
      }
      set((s) => applyClaudeStreamToBucket(s, taskId, payload));
    }),
    listen<CodexStreamEvent>("codex_stream", (e) => {
      const taskId = resolveStreamBucketId(e.payload.taskId);
      const payload = e.payload.payload;
      const activeId = useTaskStore.getState().activeTaskId;
      const isActive = taskId !== null && taskId === activeId;
      if (isActive && queueCodexBufferedUpdate(pendingStreamUpdates, payload)) {
        schedulePendingFlush();
        return;
      }
      if (isActive) {
        if (payload.kind === "thinking") {
          clearPendingCodexStream(pendingStreamUpdates);
          if (!hasPendingStreamUpdates(pendingStreamUpdates)) {
            cancelPendingFlush();
          }
        } else {
          flushPendingStreams();
        }
      }
      set((s) => applyCodexStreamToBucket(s, taskId, payload));
    }),
    listen<PermissionPromptPayload>("permission_prompt", (e) => {
      set((s) => reducePermissionPrompt(s, e.payload));
    }),
    listen<PermissionCancelledPayload>("permission_cancelled", (e) => {
      // The originating agent died (subprocess exit / bridge disconnect)
      // before the user could resolve the prompt. Yank it from the queue
      // so the banner disappears; the user would see a stuck "awaiting
      // approval" UI otherwise.
      const { requestId } = e.payload;
      set((s) => ({
        permissionPrompts: s.permissionPrompts.filter(
          (prompt) => prompt.requestId !== requestId,
        ),
        permissionError:
          s.permissionError?.requestId === requestId ? null : s.permissionError,
      }));
    }),
    listen<RuntimeHealthPayload>("runtime_health", (e) => {
      set(() => ({
        runtimeHealth: e.payload.health ?? null,
      }));
    }),
  ]).then((fns) => [...fns, cancelPendingFlush, unsubscribeTaskSwitch]);
}
