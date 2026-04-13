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
 * Messages without a taskId are included (user-sent before task stamping).
 */
export function filterMessagesByTaskId(
  messages: readonly { taskId?: string }[],
  taskId: string | null,
): typeof messages {
  if (!taskId) return messages;
  return messages.filter((m) => !m.taskId || m.taskId === taskId);
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
