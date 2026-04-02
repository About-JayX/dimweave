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
  return (
    state.agents.codex?.status === "connected" ||
    state.agents.claude?.status === "connected"
  );
}
