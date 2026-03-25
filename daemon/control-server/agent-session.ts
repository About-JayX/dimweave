import type { ServerWebSocket } from "bun";
import { state, broadcastToGui, type ControlSocketData } from "../daemon-state";
import type { ControlServerDeps } from "./types";
import { sendProtocolMessage, sendStatus } from "./message-routing";
import type { BridgeMessage } from "../types";

function sendToClient(
  ws: ServerWebSocket<ControlSocketData>,
  msg: BridgeMessage,
) {
  sendProtocolMessage(ws, { type: "routed_message", message: msg });
}

export function attachAgent(
  ws: ServerWebSocket<ControlSocketData>,
  agentId: string,
  deps: ControlServerDeps,
) {
  const existing = state.attachedAgents.get(agentId);
  if (existing && existing !== ws) {
    existing.close(4001, `replaced by a newer ${agentId} session`);
  }

  state.attachedAgents.set(agentId, ws);
  ws.data.attached = true;
  deps.log(`Agent ${agentId} attached (#${ws.data.clientId})`);
  broadcastToGui({
    type: "agent_status",
    payload: { agent: agentId, status: "connected" },
    timestamp: Date.now(),
  });

  sendStatus(ws, deps);

  // Flush buffered messages
  if (state.bufferedMessages.length > 0) {
    for (const msg of state.flushBufferedMessages()) {
      sendToClient(ws, msg);
    }
  }
}

export function detachAgent(
  ws: ServerWebSocket<ControlSocketData>,
  agentId: string,
  reason: string,
  deps: ControlServerDeps,
) {
  if (state.attachedAgents.get(agentId) !== ws) return;
  state.attachedAgents.delete(agentId);
  ws.data.attached = false;
  deps.log(`Agent ${agentId} detached (#${ws.data.clientId}, ${reason})`);
  broadcastToGui({
    type: "agent_status",
    payload: { agent: agentId, status: "disconnected" },
    timestamp: Date.now(),
  });
}
