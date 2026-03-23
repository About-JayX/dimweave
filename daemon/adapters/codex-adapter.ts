import { spawn, type ChildProcess } from "node:child_process";
import { createInterface } from "node:readline";
import { EventEmitter } from "node:events";
import { appendFileSync } from "node:fs";
import type { ServerWebSocket } from "bun";
import type { CodexAccountInfo, TuiSocketData, IdMapping } from "./codex-types";
import { CodexMessageHandler } from "./codex-message-handler";
import { patchResponse } from "./codex-response-patcher";
import { ensurePortsFree } from "./codex-port-utils";

export type { CodexAccountInfo } from "./codex-types";

const LOG_FILE = "/tmp/agentbridge.log";

export class CodexAdapter extends EventEmitter {
  private proc: ChildProcess | null = null;
  private appServerWs: WebSocket | null = null;
  private tuiWs: ServerWebSocket<TuiSocketData> | null = null;
  private proxyServer: ReturnType<typeof Bun.serve> | null = null;
  private appPort: number;
  private proxyPort: number;
  private tuiConnId = 0;
  private nextInjectionId = 900000;
  private nextProxyId = 100000;
  private upstreamToClient = new Map<number, IdMapping>();
  private intentionalDisconnect = false;
  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  private handler: CodexMessageHandler;

  private static readonly MAX_RECONNECT_ATTEMPTS = 10;
  private static readonly RECONNECT_BASE_DELAY_MS = 1000;

  constructor(appPort = 4500, proxyPort = 4501) {
    super();
    this.appPort = appPort;
    this.proxyPort = proxyPort;
    this.handler = new CodexMessageHandler(() => this.tuiConnId, {
      log: (msg) => this.log(msg),
      emitAgentMessage: (msg) => this.emit("agentMessage", msg),
      emitAgentMessageStarted: (id) => this.emit("agentMessageStarted", id),
      emitAgentMessageDelta: (id, delta) =>
        this.emit("agentMessageDelta", id, delta),
      emitPhaseChanged: (phase) => this.emit("phaseChanged", phase),
      emitTurnCompleted: () => this.emit("turnCompleted"),
      emitReady: (tid) => this.emit("ready", tid),
      emitAccountInfoUpdated: (info) => this.emit("accountInfoUpdated", info),
    });
  }

  get appServerUrl() {
    return `ws://127.0.0.1:${this.appPort}`;
  }
  get proxyUrl() {
    return `ws://127.0.0.1:${this.proxyPort}`;
  }
  get activeThreadId() {
    return this.handler.activeThreadId;
  }
  get accountInfo(): CodexAccountInfo {
    return this.handler.accountInfo;
  }

  // ── Lifecycle ────────────────────────────────────────────

  async start() {
    this.intentionalDisconnect = false;
    await ensurePortsFree([this.appPort, this.proxyPort], (m) => this.log(m));
    this.log(`Spawning codex app-server on ${this.appServerUrl}`);
    this.proc = spawn("codex", ["app-server", "--listen", this.appServerUrl], {
      stdio: ["pipe", "pipe", "pipe"],
    });

    this.proc.on("error", (err) => this.emit("error", err));
    this.proc.on("exit", (code) => this.emit("exit", code));

    const stderrRl = createInterface({ input: this.proc.stderr! });
    stderrRl.on("line", (l) => this.log(`[codex-server] ${l}`));
    const stdoutRl = createInterface({ input: this.proc.stdout! });
    stdoutRl.on("line", (l) => this.log(`[codex-stdout] ${l}`));

    await this.waitForHealthy();
    await this.connectToAppServer();
    this.startProxy();
    this.log(`Proxy ready on ${this.proxyUrl}`);
  }

  disconnect() {
    this.intentionalDisconnect = true;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    this.appServerWs?.close();
    this.appServerWs = null;
    this.proxyServer?.stop();
    this.proxyServer = null;
    this.handler.reset();
  }

  async ensureConnected(): Promise<void> {
    if (this.appServerWs?.readyState === WebSocket.OPEN) {
      // App-server is connected; ensure proxy is also running
      if (!this.proxyServer) this.startProxy();
      return;
    }
    this.intentionalDisconnect = false;
    await this.connectToAppServer(true);
    // Restart proxy if it was closed by a previous disconnect()
    if (!this.proxyServer) {
      this.startProxy();
      this.log(`Proxy restarted on ${this.proxyUrl}`);
    }
  }

  stop() {
    this.intentionalDisconnect = true;
    this.disconnect();
    if (this.proc) {
      const proc = this.proc;
      this.proc = null;
      proc.kill("SIGTERM");
      const killTimer = setTimeout(() => {
        try {
          proc.kill("SIGKILL");
        } catch {}
      }, 2000);
      proc.on("exit", () => clearTimeout(killTimer));
    }
  }

  // ── Session & messaging ──────────────────────────────────

  async initSession(opts?: {
    model?: string;
    reasoningEffort?: string;
    cwd?: string;
    developerInstructions?: string;
    sandboxMode?: string;
    approvalPolicy?: string;
  }): Promise<{ success: boolean; error?: string }> {
    if (this.handler.activeThreadId) return { success: true };

    try {
      await this.ensureConnected();
    } catch (err: any) {
      return {
        success: false,
        error: `Cannot connect to app-server: ${err.message}`,
      };
    }

    if (!this.appServerWs || this.appServerWs.readyState !== WebSocket.OPEN) {
      return { success: false, error: "App-server WebSocket not connected" };
    }

    return new Promise((resolve) => {
      const timeout = setTimeout(
        () =>
          resolve({
            success: false,
            error: "Timeout waiting for thread creation",
          }),
        10000,
      );
      const initId = this.nextInjectionId++;
      const threadRpcId = this.nextInjectionId++;

      const handleMessage = (event: MessageEvent) => {
        const data =
          typeof event.data === "string" ? event.data : event.data.toString();
        try {
          const msg = JSON.parse(data);
          if (msg.id === initId) {
            if (msg.error) {
              if (!msg.error.message?.includes("Already initialized")) {
                this.log(`Initialize warning: ${msg.error.message}`);
              }
              // Capture patched init data (userAgent, platformOs, etc.)
              const patched = patchResponse(msg, data, (m) => this.log(m));
              if (patched !== data) {
                try {
                  this.handler.intercept(JSON.parse(patched));
                } catch {}
              }
            } else {
              this.handler.intercept(msg);
            }
            this.appServerWs!.send(
              JSON.stringify({
                method: "thread/start",
                id: threadRpcId,
                params: {
                  ...(opts?.model && { model: opts.model }),
                  ...(opts?.reasoningEffort && {
                    reasoningEffort: opts.reasoningEffort,
                  }),
                  ...(opts?.cwd && { cwd: opts.cwd }),
                  ...(opts?.sandboxMode && {
                    sandbox: opts.sandboxMode,
                  }),
                  ...(opts?.approvalPolicy && {
                    approvalPolicy: opts.approvalPolicy,
                  }),
                  ...(opts?.developerInstructions && {
                    settings: {
                      developer_instructions: opts.developerInstructions,
                    },
                  }),
                },
              }),
            );
          }
          if (msg.id === threadRpcId) {
            clearTimeout(timeout);
            this.appServerWs!.removeEventListener("message", handleMessage);
            // Capture model, modelProvider, serviceTier etc. from thread/start response
            this.handler.intercept(msg);
            const tid = msg.result?.thread?.id;
            if (tid) {
              this.handler.setActiveThreadId(tid, "initSession");
              resolve({ success: true });
            } else {
              resolve({
                success: false,
                error: msg.error?.message ?? "Failed to create thread",
              });
            }
          }
        } catch {}
      };

      this.appServerWs!.addEventListener("message", handleMessage);
      this.appServerWs!.send(
        JSON.stringify({
          method: "initialize",
          id: initId,
          params: {
            clientInfo: { name: "agentbridge", version: "0.1.0" },
            protocolVersion: "0.1.0",
          },
        }),
      );
    });
  }

  injectMessage(text: string): boolean {
    if (!this.handler.activeThreadId) {
      this.log("Cannot inject: no active thread");
      return false;
    }
    if (!this.appServerWs || this.appServerWs.readyState !== WebSocket.OPEN) {
      this.log("Cannot inject: app-server WebSocket not connected");
      return false;
    }
    if (this.handler.turnInProgress) {
      this.log(`WARNING: injecting while a turn is already active`);
    }
    this.log(`Injecting message into Codex (${text.length} chars)`);
    try {
      this.appServerWs.send(
        JSON.stringify({
          method: "turn/start",
          id: this.nextInjectionId++,
          params: {
            threadId: this.handler.activeThreadId,
            input: [{ type: "text", text }],
          },
        }),
      );
      return true;
    } catch (err: any) {
      this.log(`Injection send failed: ${err.message}`);
      return false;
    }
  }

  // ── App-server connection ────────────────────────────────

  private async waitForHealthy(maxRetries = 20, delayMs = 500) {
    for (let i = 0; i < maxRetries; i++) {
      try {
        const res = await fetch(`http://127.0.0.1:${this.appPort}/healthz`);
        if (res.ok) return;
      } catch {}
      await new Promise((r) => setTimeout(r, delayMs));
    }
    throw new Error("Codex app-server failed to become healthy");
  }

  private connectToAppServer(isReconnect = false): Promise<void> {
    return new Promise((resolve, reject) => {
      const appWs = new WebSocket(this.appServerUrl);
      let settled = false;

      appWs.onopen = () => {
        settled = true;
        this.appServerWs = appWs;
        this.intentionalDisconnect = false;
        this.reconnectAttempts = 0;
        this.log(
          isReconnect
            ? "Reconnected to app-server"
            : "Connected to app-server (persistent)",
        );
        resolve();
      };

      appWs.onmessage = (event) => this.handleAppServerMessage(event);

      appWs.onerror = () => {
        this.log("App-server connection error");
        if (!settled) {
          settled = true;
          reject(new Error("Failed to connect to app-server"));
        }
      };

      appWs.onclose = () => {
        this.log("App-server connection closed");
        this.appServerWs = null;
        // Only auto-reconnect for established connections that drop unexpectedly.
        // If the promise was rejected by onerror (settled && !open), the caller
        // (scheduleReconnect) handles retry — don't double-schedule.
        if (!settled && !this.intentionalDisconnect) {
          settled = true;
          reject(new Error("Connection closed before open"));
        } else if (
          appWs.readyState !== WebSocket.CONNECTING &&
          !this.intentionalDisconnect &&
          this.reconnectAttempts === 0
        ) {
          // Connection was established then dropped — schedule reconnect
          this.scheduleReconnect();
        }
      };
    });
  }

  private handleAppServerMessage(event: MessageEvent) {
    const data =
      typeof event.data === "string" ? event.data : event.data.toString();
    let forwarded = data;

    try {
      const parsed = JSON.parse(data);

      // Protocol discovery: log method + key params
      if (parsed.method) {
        const extra =
          parsed.method === "item/started"
            ? ` type=${parsed.params?.item?.type}`
            : parsed.method === "item/agentMessage/delta"
              ? ` itemId=${parsed.params?.itemId} len=${parsed.params?.delta?.length}`
              : "";
        this.log(`[proto] notification: ${parsed.method}${extra}`);
      } else if (parsed.result) {
        this.log(
          `[proto] response id=${parsed.id} keys=${Object.keys(parsed.result).join(",")}`,
        );
      }
      const mapping =
        parsed.id !== undefined
          ? this.upstreamToClient.get(parsed.id)
          : undefined;

      if (mapping) {
        this.upstreamToClient.delete(parsed.id);
        if (mapping.connId !== this.tuiConnId) {
          this.log(`Dropping stale response (upstream id ${parsed.id})`);
          return;
        }
        parsed.id = mapping.clientId;
        const raw = JSON.stringify(parsed);
        forwarded = patchResponse(parsed, raw, (m) => this.log(m));
        // If response was patched, intercept the patched version so captureAccountData sees result
        const interceptObj = forwarded !== raw ? JSON.parse(forwarded) : parsed;
        this.handler.intercept(interceptObj, mapping.connId);
      } else {
        forwarded = patchResponse(parsed, data, (m) => this.log(m));
        const interceptObj =
          forwarded !== data ? JSON.parse(forwarded) : parsed;
        this.handler.intercept(interceptObj);
      }
    } catch {}

    if (this.tuiWs) {
      try {
        this.tuiWs.send(forwarded);
      } catch {}
    }
  }

  private scheduleReconnect() {
    if (!this.proc) return;
    // Clear any existing reconnect timer to prevent double-scheduling
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.reconnectAttempts >= CodexAdapter.MAX_RECONNECT_ATTEMPTS) {
      this.log(
        `App-server reconnect failed after ${this.reconnectAttempts} attempts.`,
      );
      this.emit(
        "error",
        new Error("App-server connection lost and reconnect failed"),
      );
      return;
    }
    const delay = Math.min(
      CodexAdapter.RECONNECT_BASE_DELAY_MS *
        Math.pow(2, this.reconnectAttempts),
      30000,
    );
    this.reconnectAttempts++;
    this.log(
      `Scheduling reconnect attempt ${this.reconnectAttempts}/${CodexAdapter.MAX_RECONNECT_ATTEMPTS} in ${delay}ms`,
    );
    this.reconnectTimer = setTimeout(async () => {
      try {
        await this.connectToAppServer(true);
      } catch {
        this.scheduleReconnect();
      }
    }, delay);
  }

  // ── TUI proxy ────────────────────────────────────────────

  private startProxy() {
    const self = this;
    this.proxyServer = Bun.serve({
      port: this.proxyPort,
      hostname: "127.0.0.1",
      fetch(req, server) {
        const url = new URL(req.url);
        if (url.pathname === "/healthz" || url.pathname === "/readyz") {
          return fetch(`http://127.0.0.1:${self.appPort}${url.pathname}`);
        }
        if (server.upgrade(req, { data: { connId: 0 } })) return undefined;
        return new Response("AgentBridge Codex Proxy");
      },
      websocket: {
        open: (ws: ServerWebSocket<TuiSocketData>) => self.onTuiConnect(ws),
        close: (ws: ServerWebSocket<TuiSocketData>) => self.onTuiDisconnect(ws),
        message: (ws: ServerWebSocket<TuiSocketData>, msg) =>
          self.onTuiMessage(ws, msg),
      },
    });
  }

  private onTuiConnect(ws: ServerWebSocket<TuiSocketData>) {
    this.tuiConnId++;
    ws.data.connId = this.tuiConnId;
    this.tuiWs = ws;
    this.log(`TUI connected (conn #${this.tuiConnId})`);
    this.emit("tuiConnected", this.tuiConnId);
  }

  private onTuiDisconnect(ws: ServerWebSocket<TuiSocketData>) {
    const connId = ws.data.connId;
    if (this.tuiWs === ws) {
      this.tuiWs = null;
      this.log(`TUI disconnected (conn #${connId})`);
      this.emit("tuiDisconnected", connId);
    }
    this.handler.cleanupConnection(connId);
    for (const [upId, m] of this.upstreamToClient.entries()) {
      if (m.connId === connId) this.upstreamToClient.delete(upId);
    }
  }

  private onTuiMessage(
    ws: ServerWebSocket<TuiSocketData>,
    msg: string | Buffer,
  ) {
    const data = typeof msg === "string" ? msg : msg.toString();
    const connId = ws.data.connId;

    if (connId !== this.tuiConnId) return;

    let forwarded = data;
    try {
      const parsed = JSON.parse(data);
      this.log(
        `TUI -> app-server: ${parsed.method ?? `response:${parsed.id}`}`,
      );

      if (parsed.id !== undefined && parsed.method) {
        const proxyId = this.nextProxyId++;
        this.upstreamToClient.set(proxyId, { connId, clientId: parsed.id });
        this.handler.trackRequest(parsed, connId);
        parsed.id = proxyId;
        forwarded = JSON.stringify(parsed);
      } else {
        this.handler.trackRequest(parsed, connId);
      }
    } catch {}

    if (this.appServerWs?.readyState === WebSocket.OPEN) {
      this.appServerWs.send(forwarded);
    }
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [CodexAdapter] ${msg}\n`;
    process.stderr.write(line);
    try {
      appendFileSync(LOG_FILE, line);
    } catch {}
  }
}
