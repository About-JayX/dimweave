import { EventEmitter } from "node:events";
import type { BridgeMessage } from "../types";
import type { ControlClientMessage } from "../control-protocol";
import type { DaemonClientEvents } from "./types";
import {
  attachSocketHandlers,
  rejectPendingReplies,
  log,
  type ClientState,
} from "./connection";

export type { DaemonClientEvents } from "./types";

export class DaemonClient extends EventEmitter<DaemonClientEvents> {
  private static readonly MAX_RECONNECT_ATTEMPTS = 10;
  private ws: WebSocket | null = null;
  private connectingPromise: Promise<void> | null = null;
  private intentionalDisconnect = false;
  private agentId = "claude";
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private reconnectAttempts = 0;
  private nextRequestId = 1;
  private pendingReplies = new Map<
    string,
    {
      resolve: (value: { success: boolean; error?: string }) => void;
      timer: ReturnType<typeof setTimeout>;
    }
  >();

  constructor(private readonly url: string) {
    super();
  }

  async connect() {
    if (this.ws?.readyState === WebSocket.OPEN) return;
    // Deduplicate concurrent connect attempts — reuse in-flight promise
    if (this.connectingPromise) return this.connectingPromise;

    this.intentionalDisconnect = false;
    this.connectingPromise = new Promise<void>((resolve, reject) => {
      const ws = new WebSocket(this.url);
      let settled = false;

      ws.onopen = () => {
        settled = true;
        this.connectingPromise = null;
        // If disconnect() was called while this socket was connecting, close it
        if (this.intentionalDisconnect) {
          ws.close();
          reject(new Error("Connection cancelled by disconnect()"));
          return;
        }
        this.ws = ws;
        attachSocketHandlers(this.buildClientState(), ws);
        resolve();
      };

      ws.onerror = () => {
        if (settled) return;
        settled = true;
        this.connectingPromise = null;
        reject(
          new Error(`Failed to connect to AgentBridge daemon at ${this.url}`),
        );
      };

      ws.onclose = () => {
        if (settled) return;
        settled = true;
        this.connectingPromise = null;
        reject(
          new Error(
            `AgentBridge daemon closed the connection during startup (${this.url})`,
          ),
        );
      };
    });
    return this.connectingPromise;
  }

  attachAgent(agentId: string = "claude") {
    this.agentId = agentId;
    this.send({ type: "agent_connect", agentId });
  }

  async disconnect() {
    this.intentionalDisconnect = true;
    this.connectingPromise = null;

    // Cancel any pending reconnect
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }

    if (!this.ws) return;

    try {
      this.send({ type: "agent_disconnect", agentId: this.agentId });
    } catch {}

    try {
      this.ws.close();
    } catch {}

    this.ws = null;
    rejectPendingReplies(this.buildClientState(), "Daemon connection closed");
  }

  async fetchMessages(): Promise<BridgeMessage[]> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) return [];

    const requestId = `fetch_${Date.now()}_${this.nextRequestId++}`;
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        this.pendingReplies.delete(requestId);
        resolve([]);
      }, 10000);

      this.pendingReplies.set(requestId, {
        resolve: (result) => resolve((result as any).messages ?? []),
        timer,
      });
      this.send({ type: "fetch_messages", requestId });
    });
  }

  async sendReply(
    message: BridgeMessage,
  ): Promise<{ success: boolean; error?: string }> {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return { success: false, error: "AgentBridge daemon is not connected." };
    }

    const requestId = `reply_${Date.now()}_${this.nextRequestId++}`;
    return new Promise((resolve) => {
      const timer = setTimeout(() => {
        this.pendingReplies.delete(requestId);
        resolve({
          success: false,
          error: "Timed out waiting for AgentBridge daemon reply.",
        });
      }, 15000);

      this.pendingReplies.set(requestId, { resolve, timer });
      this.send({
        type: "route_message",
        requestId,
        message,
      });
    });
  }

  // ── Internal helpers ──────────────────────────────────────

  private buildClientState(): ClientState {
    return {
      ws: this.ws,
      connectingPromise: this.connectingPromise,
      intentionalDisconnect: this.intentionalDisconnect,
      reconnectTimer: this.reconnectTimer,
      reconnectAttempts: this.reconnectAttempts,
      pendingReplies: this.pendingReplies,
      maxReconnectAttempts: DaemonClient.MAX_RECONNECT_ATTEMPTS,
      url: this.url,
      emitter: this,
      send: (msg) => this.send(msg),
      connect: () => this.connect(),
      attachAgent: () => this.attachAgent(this.agentId),
    };
  }

  private send(message: ControlClientMessage) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error("AgentBridge daemon socket is not open.");
    }

    this.ws.send(JSON.stringify(message));
  }
}
