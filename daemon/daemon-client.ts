import { EventEmitter } from "node:events";
import { appendFileSync } from "node:fs";
import type { BridgeMessage } from "./types";
import type {
  ControlClientMessage,
  ControlServerMessage,
  DaemonStatus,
} from "./control-protocol";

interface DaemonClientEvents {
  codexMessage: [BridgeMessage];
  disconnect: [];
  status: [DaemonStatus];
}

export class DaemonClient extends EventEmitter<DaemonClientEvents> {
  private static readonly MAX_RECONNECT_ATTEMPTS = 10;
  private ws: WebSocket | null = null;
  private connectingPromise: Promise<void> | null = null;
  private intentionalDisconnect = false;
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
        this.attachSocketHandlers(ws);
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

  attachClaude() {
    this.send({ type: "claude_connect" });
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
      this.send({ type: "claude_disconnect" });
    } catch {}

    try {
      this.ws.close();
    } catch {}

    this.ws = null;
    this.rejectPendingReplies("Daemon connection closed");
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
        type: "claude_to_codex",
        requestId,
        message,
      });
    });
  }

  private attachSocketHandlers(ws: WebSocket) {
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
        case "codex_to_claude":
          this.emit("codexMessage", message.message);
          return;
        case "claude_to_codex_result": {
          const pending = this.pendingReplies.get(message.requestId);
          if (!pending) return;
          clearTimeout(pending.timer);
          this.pendingReplies.delete(message.requestId);
          pending.resolve({ success: message.success, error: message.error });
          return;
        }
        case "fetch_messages_result": {
          const pending = this.pendingReplies.get(message.requestId);
          if (!pending) return;
          clearTimeout(pending.timer);
          this.pendingReplies.delete(message.requestId);
          pending.resolve({ messages: message.messages } as any);
          return;
        }
        case "status":
          this.emit("status", message.status);
          return;
      }
    };

    ws.onclose = () => {
      if (this.ws === ws) {
        this.ws = null;
      }
      this.rejectPendingReplies("AgentBridge daemon disconnected.");
      this.emit("disconnect");
      // Only auto-reconnect if not intentionally disconnected
      if (!this.intentionalDisconnect) {
        this.reconnectTimer = setTimeout(() => this.tryReconnect(), 2000);
      }
    };

    ws.onerror = () => {};
  }

  private rejectPendingReplies(error: string) {
    for (const [requestId, pending] of this.pendingReplies.entries()) {
      clearTimeout(pending.timer);
      pending.resolve({ success: false, error });
      this.pendingReplies.delete(requestId);
    }
  }

  private tryReconnect() {
    if (this.intentionalDisconnect) return;
    if (this.ws?.readyState === WebSocket.OPEN) return;
    if (this.reconnectAttempts >= DaemonClient.MAX_RECONNECT_ATTEMPTS) {
      this.log(
        `Reconnect failed after ${this.reconnectAttempts} attempts, giving up`,
      );
      return;
    }
    this.reconnectAttempts++;
    this.connect()
      .then(() => {
        this.reconnectAttempts = 0;
        this.attachClaude();
        this.log("Reconnected to daemon");
      })
      .catch(() => {
        if (this.intentionalDisconnect) return;
        this.reconnectTimer = setTimeout(() => this.tryReconnect(), 3000);
      });
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [DaemonClient] ${msg}\n`;
    process.stderr.write(line);
    try {
      appendFileSync("/tmp/agentbridge.log", line);
    } catch {}
  }

  private send(message: ControlClientMessage) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      throw new Error("AgentBridge daemon socket is not open.");
    }

    this.ws.send(JSON.stringify(message));
  }
}
