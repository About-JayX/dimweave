import { appendFileSync } from "node:fs";
import type { ControlServerMessage } from "../control-protocol";
import type { DaemonClientEvents } from "./types";
import type { EventEmitter } from "node:events";

const LOG_FILE = "/tmp/agentbridge.log";

/**
 * Internal state interface shared between connection helpers
 * and the DaemonClient class.
 */
export interface ClientState {
  ws: WebSocket | null;
  connectingPromise: Promise<void> | null;
  intentionalDisconnect: boolean;
  reconnectTimer: ReturnType<typeof setTimeout> | null;
  reconnectAttempts: number;
  pendingReplies: Map<
    string,
    {
      resolve: (value: { success: boolean; error?: string }) => void;
      timer: ReturnType<typeof setTimeout>;
    }
  >;
  maxReconnectAttempts: number;
  url: string;
  emitter: EventEmitter<DaemonClientEvents>;
  /** Send a typed message over the socket */
  send: (message: any) => void;
  /** Connect method reference for reconnection */
  connect: () => Promise<void>;
  /** Re-attach Claude after reconnect */
  attachAgent: () => void;
}

/**
 * Attach WebSocket event handlers for message routing and auto-reconnect.
 */
export function attachSocketHandlers(state: ClientState, ws: WebSocket): void {
  ws.onmessage = (event) => {
    const raw =
      typeof event.data === "string" ? event.data : event.data.toString();

    let message: ControlServerMessage;
    try {
      message = JSON.parse(raw);
    } catch {
      return;
    }

    switch (message.type) {
      case "routed_message":
        state.emitter.emit("routedMessage", message.message);
        return;
      case "route_result": {
        const pending = state.pendingReplies.get(message.requestId);
        if (!pending) return;
        clearTimeout(pending.timer);
        state.pendingReplies.delete(message.requestId);
        pending.resolve({ success: message.success, error: message.error });
        return;
      }
      case "fetch_messages_result": {
        const pending = state.pendingReplies.get(message.requestId);
        if (!pending) return;
        clearTimeout(pending.timer);
        state.pendingReplies.delete(message.requestId);
        pending.resolve({ messages: message.messages } as any);
        return;
      }
      case "status":
        state.emitter.emit("status", message.status);
        return;
    }
  };

  ws.onclose = () => {
    if (state.ws === ws) {
      state.ws = null;
    }
    rejectPendingReplies(state, "AgentBridge daemon disconnected.");
    state.emitter.emit("disconnect");
    // Only auto-reconnect if not intentionally disconnected
    if (!state.intentionalDisconnect) {
      state.reconnectTimer = setTimeout(() => tryReconnect(state), 2000);
    }
  };

  ws.onerror = () => {};
}

/**
 * Reject all pending reply promises with the given error message.
 */
export function rejectPendingReplies(state: ClientState, error: string): void {
  for (const [requestId, pending] of state.pendingReplies.entries()) {
    clearTimeout(pending.timer);
    pending.resolve({ success: false, error });
    state.pendingReplies.delete(requestId);
  }
}

/**
 * Attempt to reconnect to the daemon with exponential back-off.
 */
export function tryReconnect(state: ClientState): void {
  if (state.intentionalDisconnect) return;
  if (state.ws?.readyState === WebSocket.OPEN) return;
  if (state.reconnectAttempts >= state.maxReconnectAttempts) {
    log(
      `Reconnect failed after ${state.reconnectAttempts} attempts, giving up`,
    );
    return;
  }
  state.reconnectAttempts++;
  state
    .connect()
    .then(() => {
      state.reconnectAttempts = 0;
      state.attachAgent();
      log("Reconnected to daemon");
    })
    .catch(() => {
      if (state.intentionalDisconnect) return;
      state.reconnectTimer = setTimeout(() => tryReconnect(state), 3000);
    });
}

export function log(msg: string): void {
  const line = `[${new Date().toISOString()}] [DaemonClient] ${msg}\n`;
  process.stderr.write(line);
  try {
    appendFileSync(LOG_FILE, line);
  } catch {}
}
