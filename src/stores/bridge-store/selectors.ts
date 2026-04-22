import type { BridgeMessage } from "@/types";
import { hasMessagePayload } from "@/lib/message-payload";
import type { BridgeState } from "./types";
import { GLOBAL_MESSAGE_BUCKET } from "./types";
import {
  defaultClaudeStreamState,
  defaultCodexStreamState,
} from "./stream-reducers";

export function selectConnected(state: BridgeState) {
  return state.connected;
}

export function selectAgents(state: BridgeState) {
  return state.agents;
}

/// Stable empty array so selectors for tasks with no messages return
/// referentially stable data — consumers' `useMemo` chains don't
/// invalidate when an unrelated task receives a new message.
const EMPTY_MESSAGES: BridgeMessage[] = [];

/// Build a selector that reads the messages bucket for `taskId` plus the
/// global (system-diagnostic) bucket merged in chronological order. When
/// `taskId` is null, return global-only. The factory returns a stable
/// function per taskId so React can rely on reference equality.
///
/// Merging is O(n_task + n_global); for typical sessions the global
/// bucket stays short (diagnostics), so total cost remains well below
/// the previous `filterMessagesByTaskId` over the full flat array.
export function makeActiveTaskMessagesSelector(
  taskId: string | null,
): (state: BridgeState) => BridgeMessage[] {
  let lastTaskBucket: BridgeMessage[] | null = null;
  let lastGlobalBucket: BridgeMessage[] | null = null;
  let lastMerged: BridgeMessage[] | null = null;

  return (state: BridgeState) => {
    const global =
      state.messagesByTask[GLOBAL_MESSAGE_BUCKET] ?? EMPTY_MESSAGES;
    if (!taskId) return global;
    const task = state.messagesByTask[taskId] ?? EMPTY_MESSAGES;
    if (global.length === 0) return task;
    if (task.length === 0) return global;
    if (task === lastTaskBucket && global === lastGlobalBucket && lastMerged) {
      return lastMerged;
    }
    // Merge two already-chronologically-sorted arrays by timestamp.
    const merged: BridgeMessage[] = [];
    let i = 0;
    let j = 0;
    while (i < task.length && j < global.length) {
      if (task[i].timestamp <= global[j].timestamp) merged.push(task[i++]);
      else merged.push(global[j++]);
    }
    while (i < task.length) merged.push(task[i++]);
    while (j < global.length) merged.push(global[j++]);
    lastTaskBucket = task;
    lastGlobalBucket = global;
    lastMerged = merged;
    return merged;
  };
}

/// Total count of renderable chat messages across all task buckets. Excludes
/// system diagnostics and empty-payload rows so the top-bar badge matches what
/// the chat surface can actually render.
export function selectTotalMessageCount(state: BridgeState): number {
  let n = 0;
  for (const bucket of Object.values(state.messagesByTask)) {
    for (const message of bucket) {
      if (
        message.source.kind !== "system" &&
        hasMessagePayload(message.message, message.attachments)
      ) {
        n += 1;
      }
    }
  }
  return n;
}

export function selectAnyAgentConnected(state: BridgeState) {
  const agents = state.agents ?? {};
  return (
    agents.codex?.status === "connected" ||
    agents.claude?.status === "connected"
  );
}

export function selectPermissionPromptCount(state: BridgeState) {
  return state.permissionPrompts.length;
}

/// Active-task-scoped permission prompt count. Legacy prompts (no taskId)
/// are always counted so operators don't miss a stuck daemon request.
export function makeActiveTaskPermissionPromptCountSelector(
  taskId: string | null,
) {
  return (state: BridgeState) => {
    if (!taskId) return state.permissionPrompts.length;
    let count = 0;
    for (const p of state.permissionPrompts) {
      if (!p.taskId || p.taskId === taskId) count += 1;
    }
    return count;
  };
}

/// Count of prompts for OTHER tasks (not the active one). Used by
/// TaskHeader to show a pending-elsewhere badge so users aren't
/// unaware of a stuck approval when they're viewing a different task.
export function selectOtherTaskPermissionPromptCounts(
  state: BridgeState,
): Record<string, number> {
  const counts: Record<string, number> = {};
  for (const p of state.permissionPrompts) {
    if (!p.taskId) continue;
    counts[p.taskId] = (counts[p.taskId] ?? 0) + 1;
  }
  return counts;
}

export function selectTerminalErrorCount(state: BridgeState) {
  let count = 0;
  for (const line of state.terminalLines) {
    if (line.kind === "error") count += 1;
  }
  return count;
}

export function selectTerminalLineCount(state: BridgeState) {
  return state.terminalLines.length;
}

export function selectUiErrors(state: BridgeState) {
  return state.uiErrors;
}

export function selectUiErrorCount(state: BridgeState) {
  return state.uiErrors.length;
}

const DEFAULT_CLAUDE_STREAM = defaultClaudeStreamState();
const DEFAULT_CODEX_STREAM = defaultCodexStreamState();

export function makeActiveClaudeStreamSelector(
  taskId: string | null,
): (state: BridgeState) => BridgeState["claudeStream"] {
  if (!taskId) return (state) => state.claudeStream;
  return (state) => state.claudeStreamsByTask[taskId] ?? DEFAULT_CLAUDE_STREAM;
}

export function makeActiveCodexStreamSelector(
  taskId: string | null,
): (state: BridgeState) => BridgeState["codexStream"] {
  if (!taskId) return (state) => state.codexStream;
  return (state) => state.codexStreamsByTask[taskId] ?? DEFAULT_CODEX_STREAM;
}
