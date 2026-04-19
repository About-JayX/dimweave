import type { BridgeState } from "./types";

export function selectConnected(state: BridgeState) {
  return state.connected;
}

export function selectAgents(state: BridgeState) {
  return state.agents;
}

export function selectMessages(state: BridgeState) {
  return state.messages;
}

/**
 * Filter messages to the active task.
 *
 * Rules:
 * - `taskId === taskId` → kept (per-task agent/user message).
 * - `source.kind === "system"` → kept regardless of taskId. The daemon
 *   emits genuinely global notices (startup, agent-offline broadcasts,
 *   permission-queue origins, pre-task chatter) with `source = System`
 *   and no taskId on purpose; they must remain visible in every task
 *   view. See `state_task_flow.rs::stamp_message_context` — it returns
 *   early when there's no active task, which is correct for system
 *   messages but caused the bleed we closed with strict match.
 * - Everything else (per-task diagnostics that forgot to stamp) is
 *   dropped; the backend callers must stamp — that invariant was
 *   tightened in the Codex session_event.rs error paths.
 *
 * When no task is active (`taskId == null`), show everything.
 */
export function filterMessagesByTaskId(
  messages: readonly {
    taskId?: string;
    source?: { kind?: string };
  }[],
  taskId: string | null,
): typeof messages {
  if (!taskId) return messages;
  return messages.filter(
    (m) => m.taskId === taskId || m.source?.kind === "system",
  );
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
