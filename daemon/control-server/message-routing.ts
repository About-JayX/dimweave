import type { ServerWebSocket } from "bun";
import type { ControlServerMessage } from "../control-protocol";
import type { BridgeMessage } from "../types";
import { state, broadcastToGui, type ControlSocketData } from "../daemon-state";
import type { ControlServerDeps } from "./types";

/** Log MCP routing result to GUI Logs tab */
function logRoute(msg: BridgeMessage, status: string, detail: string) {
  const preview =
    msg.content.length > 80 ? msg.content.slice(0, 80) + "..." : msg.content;
  broadcastToGui({
    type: "system_log",
    payload: {
      level: status === "error" ? "error" : "info",
      message: `[MCP Route] ${msg.from} → ${msg.to} [${status}] ${detail} | "${preview}"`,
    },
    timestamp: Date.now(),
  });
}

export interface RouteTarget {
  agent: string;
  online: boolean;
}

/**
 * Resolve which agent(s) a message should be routed to based on the `to` field.
 */
export function resolveTarget(to: string): RouteTarget[] {
  if (to === "user") return [];
  const targets: RouteTarget[] = [];
  if (state.claudeRole === to) {
    const ws = state.attachedAgents.get("claude");
    targets.push({
      agent: "claude",
      online: ws !== undefined && ws.readyState === WebSocket.OPEN,
    });
  }
  if (state.codexRole === to) {
    const ws = state.attachedAgents.get("codex");
    targets.push({
      agent: "codex",
      online: ws !== undefined && ws.readyState === WebSocket.OPEN,
    });
  }
  return targets;
}

/**
 * Route a message to its target agent(s) via MCP bridge WebSocket.
 * Both Claude and Codex receive messages through the same protocol.
 */
export function routeMessage(
  msg: BridgeMessage,
  opts?: { skipSender?: string; skipGuiBroadcast?: boolean },
): { success: boolean; error?: string } {
  const { to } = msg;

  // to: "user" → GUI only
  if (to === "user") {
    broadcastToGui({
      type: "agent_message",
      payload: msg,
      timestamp: Date.now(),
    });
    logRoute(msg, "delivered", "→ GUI (user)");
    return { success: true };
  }

  const targets = resolveTarget(to);

  if (targets.length === 0) {
    const errorMsg = state.systemMessage(
      "system_route_error",
      `${to} role is not online`,
      msg.from,
    );
    broadcastToGui({
      type: "agent_message",
      payload: errorMsg,
      timestamp: Date.now(),
    });
    logRoute(msg, "error", `${to} role is not online`);
    return { success: false, error: `${to} role is not online` };
  }

  let routed = false;

  for (const target of targets) {
    if (opts?.skipSender === target.agent) continue;

    const ws = state.attachedAgents.get(target.agent);
    if (ws && ws.readyState === WebSocket.OPEN) {
      sendProtocolMessage(ws, { type: "routed_message", message: msg });
      logRoute(msg, "delivered", `→ ${target.agent} (${to})`);
      routed = true;
    } else {
      state.bufferMessage(msg);
      logRoute(msg, "buffered", `${target.agent} offline, buffered`);
    }
  }

  if (!opts?.skipGuiBroadcast) {
    broadcastToGui({
      type: "agent_message",
      payload: msg,
      timestamp: Date.now(),
    });
  }

  if (!routed) {
    return {
      success: false,
      error: `${to} role is not online (message buffered)`,
    };
  }

  return { success: true };
}

export function sendStatus(
  ws: ServerWebSocket<ControlSocketData>,
  deps: ControlServerDeps,
) {
  sendProtocolMessage(ws, { type: "status", status: deps.currentStatus() });
}

export function sendProtocolMessage(
  ws: ServerWebSocket<ControlSocketData>,
  message: ControlServerMessage,
) {
  try {
    ws.send(JSON.stringify(message));
  } catch {}
}
