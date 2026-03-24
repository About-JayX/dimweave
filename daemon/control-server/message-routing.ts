import type { ServerWebSocket } from "bun";
import type { ControlServerMessage } from "../control-protocol";
import type { BridgeMessage } from "../types";
import { state, broadcastToGui, type ControlSocketData } from "../daemon-state";
import type { ControlServerDeps } from "./types";

export interface RouteTarget {
  agent: "claude" | "codex";
  online: boolean;
}

/**
 * Resolve which agent(s) a message should be routed to based on the `to` field.
 * Returns empty array for "user" (GUI only) or unknown roles.
 */
export function resolveTarget(
  to: string,
  deps: ControlServerDeps,
): RouteTarget[] {
  if (to === "user") return [];
  const targets: RouteTarget[] = [];
  if (state.claudeRole === to) {
    targets.push({
      agent: "claude",
      online:
        state.attachedClaude !== null &&
        state.attachedClaude.readyState === WebSocket.OPEN,
    });
  }
  if (state.codexRole === to) {
    targets.push({
      agent: "codex",
      online: deps.codex.activeThreadId !== null,
    });
  }
  return targets;
}

/**
 * Route a message to its target agent(s).
 * - Matches target by role → agent mapping
 * - Buffers if target is offline
 * - Broadcasts to GUI
 * - Returns system error message if no target found
 */
export function routeMessage(
  msg: BridgeMessage,
  deps: ControlServerDeps,
  opts?: { skipSender?: "claude" | "codex"; skipGuiBroadcast?: boolean },
): { success: boolean; error?: string } {
  const { to } = msg;

  // to: "user" → GUI only
  if (to === "user") {
    broadcastToGui({
      type: "agent_message",
      payload: msg,
      timestamp: Date.now(),
    });
    return { success: true };
  }

  const targets = resolveTarget(to, deps);

  if (targets.length === 0) {
    // No agent has this role — system error
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
    return { success: false, error: `${to} role is not online` };
  }

  let routed = false;

  for (const target of targets) {
    // Skip sending back to sender
    if (opts?.skipSender === target.agent) continue;

    if (!target.online) {
      state.bufferMessage(msg);
      continue;
    }

    if (target.agent === "claude") {
      if (state.attachedClaude) {
        sendProtocolMessage(state.attachedClaude, {
          type: "routed_message",
          message: msg,
        });
        routed = true;
      } else {
        state.bufferMessage(msg);
      }
    } else if (target.agent === "codex") {
      if (deps.codex.activeThreadId) {
        deps.codex.injectMessage(msg.content);
        routed = true;
      } else {
        state.bufferMessage(msg);
      }
    }
  }

  // Broadcast to GUI (skip if caller already handled it, e.g. streaming)
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
