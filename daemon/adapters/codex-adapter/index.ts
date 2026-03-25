import { EventEmitter } from "node:events";
import { appendFileSync } from "node:fs";
import { CodexMessageHandler } from "../codex-message-handler";
import type {
  AdapterState,
  CodexAccountInfo,
  CodexStartOptions,
  InitSessionOptions,
} from "./types";
import { LOG_FILE } from "./types";
import {
  startCodex,
  disconnectCodex,
  ensureConnected,
  stopCodex,
} from "./lifecycle";
import { initSession, injectMessage } from "./session";
import { setDynamicToolHandler, type DynamicToolHandler } from "./app-server";

export type { CodexAccountInfo } from "./types";
export type { CodexStartOptions } from "./types";
export type { DynamicToolHandler } from "./app-server";

export class CodexAdapter extends EventEmitter {
  private state: AdapterState;

  constructor(appPort = 4500, proxyPort = 4501) {
    super();
    this.state = {
      proc: null,
      appServerWs: null,
      tuiWs: null,
      proxyServer: null,
      appPort,
      proxyPort,
      tuiConnId: 0,
      nextInjectionId: 900000,
      nextProxyId: 100000,
      upstreamToClient: new Map(),
      intentionalDisconnect: false,
      reconnectAttempts: 0,
      reconnectTimer: null,
      emitter: this,
      handler: new CodexMessageHandler(() => this.state.tuiConnId, {
        log: (msg) => this.log(msg),
        emitAgentMessage: (msg) => this.emit("agentMessage", msg),
        emitAgentMessageStarted: (id) => this.emit("agentMessageStarted", id),
        emitAgentMessageDelta: (id, delta) =>
          this.emit("agentMessageDelta", id, delta),
        emitPhaseChanged: (phase) => this.emit("phaseChanged", phase),
        emitTurnCompleted: () => this.emit("turnCompleted"),
        emitReady: (tid) => this.emit("ready", tid),
        emitAccountInfoUpdated: (info) => this.emit("accountInfoUpdated", info),
      }),
    };
  }

  get appServerUrl() {
    return `ws://127.0.0.1:${this.state.appPort}`;
  }
  get proxyUrl() {
    return `ws://127.0.0.1:${this.state.proxyPort}`;
  }
  get activeThreadId() {
    return this.state.handler.activeThreadId;
  }
  get accountInfo(): CodexAccountInfo {
    return this.state.handler.accountInfo;
  }

  async start(opts?: CodexStartOptions) {
    return startCodex(this.state, this, (m) => this.log(m), opts);
  }

  disconnect() {
    disconnectCodex(this.state);
  }

  async ensureConnected(): Promise<void> {
    return ensureConnected(this.state, this, (m) => this.log(m));
  }

  stop() {
    stopCodex(this.state);
  }

  async initSession(
    opts?: InitSessionOptions,
  ): Promise<{ success: boolean; error?: string }> {
    return initSession(this.state, this, (m) => this.log(m), opts);
  }

  injectMessage(text: string): boolean {
    return injectMessage(this.state, (m) => this.log(m), text);
  }

  setDynamicToolHandler(handler: DynamicToolHandler) {
    setDynamicToolHandler(handler);
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [CodexAdapter] ${msg}\n`;
    process.stderr.write(line);
    try {
      appendFileSync(LOG_FILE, line);
    } catch {}
  }
}
