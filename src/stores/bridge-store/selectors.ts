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
 * Filter messages to only those belonging to a specific task.
 *
 * Strict match: messages that lack a taskId are EXCLUDED when a task is
 * active. Untagged messages leaked across task views when we allowed
 * them through (daemon diagnostics, pre-task chatter, etc. would bleed
 * into every task's chat history).
 *
 * When no task is active (`taskId == null`), show everything — there
 * is no task scope to preserve.
 */
export function filterMessagesByTaskId(
  messages: readonly { taskId?: string }[],
  taskId: string | null,
): typeof messages {
  if (!taskId) return messages;
  return messages.filter((m) => m.taskId === taskId);
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
