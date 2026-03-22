import { spawn, execSync, type ChildProcess } from "node:child_process";
import { createInterface } from "node:readline";
import { EventEmitter } from "node:events";
import { appendFileSync } from "node:fs";
import type { BridgeMessage, CodexItem } from "../types";
import type { ServerWebSocket } from "bun";

interface TuiSocketData {
  connId: number;
}

const LOG_FILE = "/tmp/agentbridge.log";
const TRACKED_REQUEST_METHODS = new Set(["thread/start", "thread/resume", "turn/start"]);

type TrackedRequestMethod = "thread/start" | "thread/resume" | "turn/start";

interface PendingRequest {
  method: TrackedRequestMethod;
  threadId?: string;
}

export class CodexAdapter extends EventEmitter {
  private proc: ChildProcess | null = null;
  private appServerWs: WebSocket | null = null;
  private tuiWs: ServerWebSocket<TuiSocketData> | null = null;
  private proxyServer: ReturnType<typeof Bun.serve> | null = null;
  private threadId: string | null = null;
  private nextInjectionId = 900000;
  private appPort: number;
  private proxyPort: number;
  private tuiConnId = 0;

  private agentMessageBuffers = new Map<string, string[]>();
  private pendingRequests = new Map<string, PendingRequest>();
  private activeTurnIds = new Set<string>();
  private turnInProgress = false;

  private nextProxyId = 100000;
  private upstreamToClient = new Map<number, { connId: number; clientId: number | string }>();
  private intentionalDisconnect = false;

  constructor(appPort = 4500, proxyPort = 4501) {
    super();
    this.appPort = appPort;
    this.proxyPort = proxyPort;
  }

  get appServerUrl() { return `ws://127.0.0.1:${this.appPort}`; }
  get proxyUrl() { return `ws://127.0.0.1:${this.proxyPort}`; }
  get activeThreadId() { return this.threadId; }

  async start() {
    this.intentionalDisconnect = false;
    await this.checkPorts();
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
    this.threadId = null;
    this.activeTurnIds.clear();
    this.turnInProgress = false;
    this.agentMessageBuffers.clear();
    this.pendingRequests.clear();
  }

  /** Reconnect to a running app-server (after disconnect). */
  async ensureConnected(): Promise<void> {
    if (this.appServerWs?.readyState === WebSocket.OPEN) return;
    this.intentionalDisconnect = false;
    await this.connectToAppServer(true);
  }

  stop() {
    this.intentionalDisconnect = true;
    this.disconnect();
    if (this.proc) {
      const proc = this.proc;
      this.proc = null;
      proc.kill("SIGTERM");
      const killTimer = setTimeout(() => {
        try { proc.kill("SIGKILL"); } catch {}
      }, 2000);
      proc.on("exit", () => clearTimeout(killTimer));
    }
  }

  /** Initialize a session directly via app-server (no TUI needed). */
  async initSession(): Promise<{ success: boolean; error?: string }> {
    if (this.threadId) {
      return { success: true };
    }

    // Reconnect if needed
    try {
      await this.ensureConnected();
    } catch (err: any) {
      return { success: false, error: `Cannot connect to app-server: ${err.message}` };
    }

    if (!this.appServerWs || this.appServerWs.readyState !== WebSocket.OPEN) {
      return { success: false, error: "App-server WebSocket not connected" };
    }

    return new Promise((resolve) => {
      const timeout = setTimeout(() => resolve({ success: false, error: "Timeout waiting for thread creation" }), 10000);

      // Step 1: initialize
      const initId = this.nextInjectionId++;
      const threadId_rpcId = this.nextInjectionId++;

      const handleMessage = (event: MessageEvent) => {
        const data = typeof event.data === "string" ? event.data : event.data.toString();
        try {
          const msg = JSON.parse(data);
          if (msg.id === initId) {
            if (msg.error) {
              // "Already initialized" is fine
              if (!msg.error.message?.includes("Already initialized")) {
                this.log(`Initialize warning: ${msg.error.message}`);
              }
            }
            // Step 2: create thread
            this.appServerWs!.send(JSON.stringify({
              method: "thread/start",
              id: threadId_rpcId,
              params: {},
            }));
          }
          if (msg.id === threadId_rpcId) {
            clearTimeout(timeout);
            this.appServerWs!.removeEventListener("message", handleMessage);
            const tid = msg.result?.thread?.id;
            if (tid) {
              this.setActiveThreadId(tid, "initSession");
              this.emit("ready", tid);
              resolve({ success: true });
            } else {
              resolve({ success: false, error: msg.error?.message ?? "Failed to create thread" });
            }
          }
        } catch {}
      };

      this.appServerWs!.addEventListener("message", handleMessage);

      this.appServerWs!.send(JSON.stringify({
        method: "initialize",
        id: initId,
        params: {
          clientInfo: { name: "agentbridge", version: "0.1.0" },
          protocolVersion: "0.1.0",
        },
      }));
    });
  }

  injectMessage(text: string): boolean {
    if (!this.threadId) {
      this.log("Cannot inject: no active thread");
      return false;
    }
    if (!this.appServerWs || this.appServerWs.readyState !== WebSocket.OPEN) {
      this.log("Cannot inject: app-server WebSocket not connected");
      return false;
    }
    if (this.turnInProgress) {
      this.log(`WARNING: injecting while a turn is already active (thread ${this.threadId})`);
    }
    this.log(`Injecting message into Codex (${text.length} chars)`);
    try {
      this.appServerWs.send(JSON.stringify({
        method: "turn/start",
        id: this.nextInjectionId++,
        params: { threadId: this.threadId, input: [{ type: "text", text }] },
      }));
      return true;
    } catch (err: any) {
      this.log(`Injection send failed: ${err.message}`);
      return false;
    }
  }

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

      appWs.onopen = () => {
        this.appServerWs = appWs;
        this.intentionalDisconnect = false;
        this.reconnectAttempts = 0;
        this.log(isReconnect ? "Reconnected to app-server" : "Connected to app-server (persistent)");
        resolve();
      };

      appWs.onmessage = (event) => {
        const data = typeof event.data === "string" ? event.data : event.data.toString();

        let forwarded = data;
        try {
          const parsed = JSON.parse(data);
          const mapping = (parsed.id !== undefined) ? this.upstreamToClient.get(parsed.id) : undefined;
          if (mapping) {
            this.upstreamToClient.delete(parsed.id);
            if (mapping.connId !== this.tuiConnId) {
              this.log(`Dropping stale response (upstream id ${parsed.id}, from conn #${mapping.connId}, current #${this.tuiConnId})`);
              return;
            }
            parsed.id = mapping.clientId;
            forwarded = this.patchResponse(parsed, JSON.stringify(parsed));
            this.interceptServerMessage(parsed, mapping.connId);
          } else {
            forwarded = this.patchResponse(parsed, data);
            this.interceptServerMessage(parsed);
          }
        } catch {}

        if (this.tuiWs) {
          try { this.tuiWs.send(forwarded); } catch {}
        }
      };

      appWs.onerror = () => {
        this.log("App-server connection error");
        if (!isReconnect) reject(new Error("Failed to connect to app-server"));
      };

      appWs.onclose = () => {
        this.log("App-server connection closed");
        this.appServerWs = null;
        if (!this.intentionalDisconnect) {
          this.scheduleReconnect();
        }
      };
    });
  }

  private reconnectAttempts = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private static readonly MAX_RECONNECT_ATTEMPTS = 10;
  private static readonly RECONNECT_BASE_DELAY_MS = 1000;

  private scheduleReconnect() {
    if (!this.proc) return;

    if (this.reconnectAttempts >= CodexAdapter.MAX_RECONNECT_ATTEMPTS) {
      this.log(`App-server reconnect failed after ${this.reconnectAttempts} attempts. Giving up.`);
      this.emit("error", new Error("App-server connection lost and reconnect failed"));
      return;
    }

    const delay = Math.min(
      CodexAdapter.RECONNECT_BASE_DELAY_MS * Math.pow(2, this.reconnectAttempts),
      30000,
    );
    this.reconnectAttempts++;
    this.log(`Scheduling app-server reconnect attempt ${this.reconnectAttempts}/${CodexAdapter.MAX_RECONNECT_ATTEMPTS} in ${delay}ms...`);

    this.reconnectTimer = setTimeout(async () => {
      try {
        await this.connectToAppServer(true);
        this.log("App-server reconnect successful");
      } catch {
        this.log("App-server reconnect attempt failed");
        this.scheduleReconnect();
      }
    }, delay);
  }

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
        message: (ws: ServerWebSocket<TuiSocketData>, msg) => self.onTuiMessage(ws, msg),
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
      this.log(`TUI disconnected (conn #${connId})`);
      this.tuiWs = null;
      this.emit("tuiDisconnected", connId);
    } else {
      this.log(`Stale TUI disconnected (conn #${connId}, current is #${this.tuiConnId})`);
    }

    const prefix = `${connId}:`;
    for (const key of this.pendingRequests.keys()) {
      if (key.startsWith(prefix)) this.pendingRequests.delete(key);
    }
    for (const [upId, mapping] of this.upstreamToClient.entries()) {
      if (mapping.connId === connId) this.upstreamToClient.delete(upId);
    }
  }

  private onTuiMessage(ws: ServerWebSocket<TuiSocketData>, msg: string | Buffer) {
    const data = typeof msg === "string" ? msg : msg.toString();
    const connId = ws.data.connId;

    if (connId !== this.tuiConnId) {
      this.log(`Dropping message from stale TUI conn #${connId} (current is #${this.tuiConnId})`);
      return;
    }

    let forwarded = data;
    try {
      const parsed = JSON.parse(data);
      const method = parsed.method ?? `response:${parsed.id}`;
      this.log(`TUI -> app-server: ${method}`);

      if (parsed.id !== undefined && parsed.method) {
        const proxyId = this.nextProxyId++;
        this.upstreamToClient.set(proxyId, { connId, clientId: parsed.id });
        this.trackPendingRequest(parsed, connId, proxyId);
        parsed.id = proxyId;
        forwarded = JSON.stringify(parsed);
      } else {
        this.trackPendingRequest(parsed, connId);
      }
    } catch {
      this.log(`TUI -> app-server: (unparseable)`);
    }

    if (this.appServerWs?.readyState === WebSocket.OPEN) {
      this.appServerWs.send(forwarded);
    } else {
      this.log(`WARNING: app-server not connected, dropping message`);
    }
  }

  private patchResponse(parsed: any, raw: string): string {
    if (parsed.error && parsed.id !== undefined) {
      const errMsg: string = parsed.error.message ?? "";
      if (errMsg.includes("rate limits") || errMsg.includes("rateLimits")) {
        this.log(`Patching rateLimits error -> mock success (id: ${parsed.id})`);
        return JSON.stringify({
          id: parsed.id,
          result: {
            rateLimits: {
              limitId: null, limitName: null,
              primary: { usedPercent: 0, windowDurationMins: 60, resetsAt: null },
              secondary: null, credits: null, planType: null,
            },
            rateLimitsByLimitId: null,
          },
        });
      }
      if (errMsg.includes("Already initialized")) {
        this.log(`Patching "Already initialized" error (id: ${parsed.id})`);
        return JSON.stringify({
          id: parsed.id,
          result: {
            userAgent: "agent_bridge/0.1.0",
            platformFamily: "unix",
            platformOs: "macos",
          },
        });
      }
    }
    return raw;
  }

  private interceptServerMessage(msg: any, connId?: number) {
    this.handleTrackedResponse(msg, connId);
    if (msg.method) this.handleServerNotification(msg);
  }

  private handleServerNotification(msg: any) {
    const { method, params } = msg;
    switch (method) {
      case "turn/started":
        this.markTurnStarted(params?.turn?.id);
        break;
      case "item/started": {
        const item: CodexItem = params?.item;
        if (item?.type === "agentMessage") this.agentMessageBuffers.set(item.id, []);
        break;
      }
      case "item/agentMessage/delta": {
        const buf = this.agentMessageBuffers.get(params?.itemId);
        if (buf && params?.delta) buf.push(params.delta);
        break;
      }
      case "item/completed": {
        const item: CodexItem = params?.item;
        if (item?.type === "agentMessage") {
          const content = this.extractContent(item);
          this.agentMessageBuffers.delete(item.id);
          if (content) {
            this.log(`Agent message completed (${content.length} chars)`);
            this.emit("agentMessage", {
              id: item.id, source: "codex" as const, content, timestamp: Date.now(),
            } satisfies BridgeMessage);
          }
        }
        break;
      }
      case "turn/completed":
        this.markTurnCompleted(params?.turn?.id);
        this.emit("turnCompleted");
        break;
    }
  }

  private extractContent(item: CodexItem): string {
    if (item.content?.length) {
      return item.content.filter((c) => c.type === "text" && c.text).map((c) => c.text!).join("");
    }
    return this.agentMessageBuffers.get(item.id)?.join("") ?? "";
  }

  private pendingKey(rpcId: unknown, connId?: number): string | null {
    const base = this.requestKey(rpcId);
    if (!base) return null;
    return `${connId ?? this.tuiConnId}:${base}`;
  }

  private trackPendingRequest(message: any, connId: number, _proxyId?: number) {
    const method = message?.method;
    const key = this.pendingKey(message?.id, connId);

    this.log(`[track] method=${method} id=${message?.id} (type=${typeof message?.id}) key=${key}`);

    if (!key || !TRACKED_REQUEST_METHODS.has(method)) return;

    const pending: PendingRequest = { method };
    if (method === "turn/start") {
      const threadId = message?.params?.threadId;
      if (typeof threadId === "string" && threadId.length > 0) {
        pending.threadId = threadId;
      }
    }

    if (this.pendingRequests.has(key)) {
      this.log(`WARNING: overwriting pending request for key ${key}`);
    }

    this.pendingRequests.set(key, pending);
  }

  private handleTrackedResponse(message: any, connId?: number) {
    const key = this.pendingKey(message?.id, connId);
    if (!key) return;

    const pending = this.pendingRequests.get(key);
    if (!pending) {
      if (message?.result?.thread?.id) {
        this.log(`[track-resp] Unmatched response with thread.id=${message.result.thread.id}, key=${key}`);
      }
      return;
    }

    this.pendingRequests.delete(key);

    if (message?.error) {
      this.log(`Tracked request failed (${pending.method}, id ${key}): ${message.error.message ?? "unknown error"}`);
      return;
    }

    switch (pending.method) {
      case "thread/start": {
        const threadId = message?.result?.thread?.id;
        if (typeof threadId === "string" && threadId.length > 0) {
          this.setActiveThreadId(threadId, `thread/start response ${key}`);
        }
        break;
      }
      case "thread/resume": {
        const threadId = message?.result?.thread?.id;
        if (typeof threadId === "string" && threadId.length > 0) {
          this.setActiveThreadId(threadId, `thread/resume response ${key}`);
        }
        break;
      }
      case "turn/start":
        if (pending.threadId) {
          this.setActiveThreadId(pending.threadId, `turn/start response ${key}`);
        }
        break;
    }
  }

  private setActiveThreadId(threadId: string, reason: string) {
    if (this.threadId === threadId) return;
    const previousThreadId = this.threadId;
    this.threadId = threadId;
    if (previousThreadId) {
      this.log(`Active thread changed: ${previousThreadId} -> ${threadId} (${reason})`);
      return;
    }
    this.log(`Thread detected: ${threadId} (${reason})`);
    this.emit("ready", threadId);
  }

  private markTurnStarted(turnId?: string) {
    if (typeof turnId === "string" && turnId.length > 0) {
      this.activeTurnIds.add(turnId);
    } else {
      this.activeTurnIds.add(`unknown:${Date.now()}`);
    }
    this.turnInProgress = this.activeTurnIds.size > 0;
  }

  private markTurnCompleted(turnId?: string) {
    if (typeof turnId === "string" && turnId.length > 0) {
      this.activeTurnIds.delete(turnId);
    } else {
      this.activeTurnIds.clear();
    }
    this.turnInProgress = this.activeTurnIds.size > 0;
  }

  private requestKey(id: unknown): string | null {
    if (typeof id === "number" || typeof id === "string") return String(id);
    return null;
  }

  private async checkPorts() {
    for (const port of [this.appPort, this.proxyPort]) {
      try {
        const pids = execSync(`lsof -ti :${port}`, { encoding: "utf-8" }).trim();
        if (!pids) continue;

        const pidList = pids.split("\n").map((p) => p.trim()).filter(Boolean);
        const staleCodexPids: string[] = [];
        const foreignPids: string[] = [];

        for (const pid of pidList) {
          try {
            const cmdline = execSync(`ps -p ${pid} -o args=`, { encoding: "utf-8" }).trim();
            if (cmdline.includes("codex") && cmdline.includes("app-server")) {
              staleCodexPids.push(pid);
            } else {
              foreignPids.push(pid);
            }
          } catch {}
        }

        if (staleCodexPids.length > 0) {
          this.log(`Cleaning up stale codex app-server on port ${port}: PID(s) ${staleCodexPids.join(", ")}`);
          for (const pid of staleCodexPids) {
            try { execSync(`kill ${pid}`, { encoding: "utf-8" }); } catch {}
          }
          await new Promise((r) => setTimeout(r, 500));
        }

        if (foreignPids.length > 0) {
          throw new Error(
            `Port ${port} is already in use by non-Codex process(es): PID(s) ${foreignPids.join(", ")}. ` +
            `Please stop the process or set a different port via ${port === this.appPort ? "CODEX_WS_PORT" : "CODEX_PROXY_PORT"} env var.`
          );
        }

        try {
          const remaining = execSync(`lsof -ti :${port}`, { encoding: "utf-8" }).trim();
          if (remaining) {
            throw new Error(`Port ${port} is still occupied after cleanup.`);
          }
        } catch (err: any) {
          if (err.message?.includes("Port")) throw err;
        }
      } catch (err: any) {
        if (err.message?.includes("Port") || err.message?.includes("non-Codex")) throw err;
      }
    }
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [CodexAdapter] ${msg}\n`;
    process.stderr.write(line);
    try { appendFileSync(LOG_FILE, line); } catch {}
  }
}
