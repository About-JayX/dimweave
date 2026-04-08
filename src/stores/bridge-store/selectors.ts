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
