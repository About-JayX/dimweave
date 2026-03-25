import type { ChildProcess } from "node:child_process";
import type { EventEmitter } from "node:events";
import type { ServerWebSocket } from "bun";
import type { CodexAccountInfo, TuiSocketData, IdMapping } from "./codex-types";
import type { CodexMessageHandler } from "../codex-message-handler";

export type { CodexAccountInfo } from "./codex-types";

export interface CodexStartOptions {
  /** CODEX_HOME temp directory (from SessionManager) */
  codexHome?: string;
  /** Codex sandbox mode: "read-only" | "workspace-write" | "danger-full-access" */
  sandboxMode?: string;
  /** Codex approval policy */
  approvalPolicy?: string;
  /** Disable apply_patch_freeform feature flag */
  disableApplyPatch?: boolean;
  /** Absolute path to bridge.ts for MCP injection */
  bridgePath?: string;
  /** Control port for MCP bridge */
  controlPort?: number;
}

export interface InitSessionOptions {
  model?: string;
  reasoningEffort?: string;
  cwd?: string;
  developerInstructions?: string;
  sandboxMode?: string;
  approvalPolicy?: string;
}

export interface AdapterState {
  proc: ChildProcess | null;
  appServerWs: WebSocket | null;
  tuiWs: ServerWebSocket<TuiSocketData> | null;
  proxyServer: ReturnType<typeof Bun.serve> | null;
  appPort: number;
  proxyPort: number;
  tuiConnId: number;
  nextInjectionId: number;
  nextProxyId: number;
  upstreamToClient: Map<number, IdMapping>;
  intentionalDisconnect: boolean;
  reconnectAttempts: number;
  reconnectTimer: ReturnType<typeof setTimeout> | null;
  handler: CodexMessageHandler;
  emitter: EventEmitter | null;
}

export const MAX_RECONNECT_ATTEMPTS = 10;
export const RECONNECT_BASE_DELAY_MS = 1000;
export const LOG_FILE = "/tmp/agentbridge.log";
