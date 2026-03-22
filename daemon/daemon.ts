#!/usr/bin/env bun

import { appendFileSync, unlinkSync, writeFileSync } from "node:fs";
import type { ServerWebSocket } from "bun";
import { CodexAdapter } from "./adapters/codex-adapter";
import { TuiConnectionState } from "./tui-connection-state";
import type { ControlClientMessage, ControlServerMessage, DaemonStatus } from "./control-protocol";
import type { BridgeMessage } from "./types";

interface ControlSocketData {
  clientId: number;
  attached: boolean;
}

interface GuiSocketData {
  clientId: number;
}

// GUI WebSocket event types
export interface GuiEvent {
  type: "agent_message" | "agent_status" | "system_log" | "daemon_status";
  payload: any;
  timestamp: number;
}

const CODEX_APP_PORT = parseInt(process.env.CODEX_WS_PORT ?? "4500", 10);
const CODEX_PROXY_PORT = parseInt(process.env.CODEX_PROXY_PORT ?? "4501", 10);
const CONTROL_PORT = parseInt(process.env.AGENTBRIDGE_CONTROL_PORT ?? "4502", 10);
const GUI_PORT = parseInt(process.env.AGENTBRIDGE_GUI_PORT ?? "4503", 10);
const PID_FILE = process.env.AGENTBRIDGE_PID_FILE ?? `/tmp/agentbridge-daemon-${CONTROL_PORT}.pid`;
const LOG_FILE = "/tmp/agentbridge.log";
const TUI_DISCONNECT_GRACE_MS = parseInt(process.env.TUI_DISCONNECT_GRACE_MS ?? "2500", 10);
const MAX_BUFFERED_MESSAGES = parseInt(process.env.AGENTBRIDGE_MAX_BUFFERED_MESSAGES ?? "100", 10);

const codex = new CodexAdapter(CODEX_APP_PORT, CODEX_PROXY_PORT);
const attachCmd = `codex --enable tui_app_server --remote ${codex.proxyUrl}`;

let controlServer: ReturnType<typeof Bun.serve> | null = null;
let guiServer: ReturnType<typeof Bun.serve> | null = null;
let attachedClaude: ServerWebSocket<ControlSocketData> | null = null;
let nextControlClientId = 0;
let nextGuiClientId = 0;
let nextSystemMessageId = 0;
let codexBootstrapped = false;
let shuttingDown = false;

const bufferedMessages: BridgeMessage[] = [];
const guiClients = new Set<ServerWebSocket<GuiSocketData>>();

const tuiConnectionState = new TuiConnectionState({
  disconnectGraceMs: TUI_DISCONNECT_GRACE_MS,
  log,
  onDisconnectPersisted: (connId) => {
    emitToClaude(
      systemMessage("system_tui_disconnected",
        `Codex TUI disconnected (conn #${connId}). Codex is still running in the background.`),
    );
  },
  onReconnectAfterNotice: (connId) => {
    emitToClaude(
      systemMessage("system_tui_reconnected",
        `Codex TUI reconnected (conn #${connId}). Bridge restored.`),
    );
    codex.injectMessage("Claude Code is still online, bridge restored. Bidirectional communication can continue.");
  },
});

// ── Codex events ─────────────────────────────────────────

codex.on("agentMessage", (msg: BridgeMessage) => {
  if (msg.source !== "codex") return;
  log(`Forwarding Codex -> Claude (${msg.content.length} chars)`);
  emitToClaude(msg);
  broadcastToGui({ type: "agent_message", payload: msg, timestamp: Date.now() });
});

codex.on("turnCompleted", () => {
  log("Codex turn completed");
});

codex.on("ready", (threadId: string) => {
  tuiConnectionState.markBridgeReady();
  log(`Codex ready - thread ${threadId}`);
  log("Bridge fully operational");

  const readyMsg = systemMessage("system_ready",
    `Codex TUI connected, session thread created (${threadId}). Bridge is fully operational.`);
  emitToClaude(readyMsg);
  broadcastToGui({ type: "agent_status", payload: { agent: "codex", status: "connected", threadId }, timestamp: Date.now() });

  if (attachedClaude) {
    notifyCodexClaudeOnline();
  }
});

codex.on("tuiConnected", (connId: number) => {
  tuiConnectionState.handleTuiConnected(connId);
  log(`Codex TUI connected (conn #${connId})`);
  broadcastStatus();
});

codex.on("tuiDisconnected", (connId: number) => {
  tuiConnectionState.handleTuiDisconnected(connId);
  log(`Codex TUI disconnected (conn #${connId})`);
  broadcastToGui({ type: "agent_status", payload: { agent: "codex", status: "disconnected" }, timestamp: Date.now() });
  broadcastStatus();
});

codex.on("error", (err: Error) => {
  log(`Codex error: ${err.message}`);
  broadcastToGui({ type: "agent_status", payload: { agent: "codex", status: "error", error: err.message }, timestamp: Date.now() });
});

codex.on("exit", (code: number | null) => {
  log(`Codex process exited (code ${code})`);
  tuiConnectionState.handleCodexExit();
  emitToClaude(
    systemMessage("system_codex_exit",
      `Codex app-server exited (code ${code ?? "unknown"}).`),
  );
  broadcastToGui({ type: "agent_status", payload: { agent: "codex", status: "disconnected", exitCode: code }, timestamp: Date.now() });
  broadcastStatus();
});

// ── Control server (bridge <-> daemon) ───────────────────

function startControlServer() {
  controlServer = Bun.serve({
    port: CONTROL_PORT,
    hostname: "127.0.0.1",
    fetch(req, server) {
      const url = new URL(req.url);
      if (url.pathname === "/healthz" || url.pathname === "/readyz") {
        return Response.json(currentStatus());
      }
      if (url.pathname === "/ws" && server.upgrade(req, { data: { clientId: 0, attached: false } })) {
        return undefined;
      }
      return new Response("AgentBridge daemon");
    },
    websocket: {
      open: (ws: ServerWebSocket<ControlSocketData>) => {
        ws.data.clientId = ++nextControlClientId;
        log(`Frontend socket opened (#${ws.data.clientId})`);
      },
      close: (ws: ServerWebSocket<ControlSocketData>) => {
        log(`Frontend socket closed (#${ws.data.clientId})`);
        if (attachedClaude === ws) {
          detachClaude(ws, "frontend socket closed");
        }
      },
      message: (ws: ServerWebSocket<ControlSocketData>, raw) => {
        handleControlMessage(ws, raw);
      },
    },
  });
}

// ── GUI WebSocket server (daemon -> GUI) ─────────────────

function startGuiServer() {
  guiServer = Bun.serve({
    port: GUI_PORT,
    hostname: "127.0.0.1",
    fetch(req, server) {
      const url = new URL(req.url);

      // CORS headers for Tauri/browser dev
      const corsHeaders = {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET, OPTIONS",
        "Access-Control-Allow-Headers": "*",
      };

      if (req.method === "OPTIONS") {
        return new Response(null, { headers: corsHeaders });
      }

      if (url.pathname === "/healthz") {
        return Response.json({ ok: true, pid: process.pid }, { headers: corsHeaders });
      }

      if (url.pathname === "/status") {
        return Response.json(currentStatus(), { headers: corsHeaders });
      }

      if (server.upgrade(req, { data: { clientId: 0 } })) return undefined;
      return new Response("AgentBridge GUI Server", { headers: corsHeaders });
    },
    websocket: {
      open: (ws: ServerWebSocket<GuiSocketData>) => {
        ws.data.clientId = ++nextGuiClientId;
        guiClients.add(ws);
        log(`GUI client connected (#${ws.data.clientId})`);
        // Send current status on connect
        sendGuiEvent(ws, { type: "daemon_status", payload: currentStatus(), timestamp: Date.now() });
      },
      close: (ws: ServerWebSocket<GuiSocketData>) => {
        guiClients.delete(ws);
        log(`GUI client disconnected (#${ws.data.clientId})`);
      },
      message: (ws: ServerWebSocket<GuiSocketData>, raw) => {
        handleGuiMessage(ws, raw);
      },
    },
  });
}

function handleGuiMessage(ws: ServerWebSocket<GuiSocketData>, raw: string | Buffer) {
  let message: any;
  try {
    const text = typeof raw === "string" ? raw : raw.toString();
    message = JSON.parse(text);
  } catch { return; }

  switch (message.type) {
    case "send_to_codex": {
      if (!tuiConnectionState.canReply()) {
        sendGuiEvent(ws, { type: "system_log", payload: { level: "error", message: "Codex is not ready." }, timestamp: Date.now() });
        return;
      }
      const injected = codex.injectMessage(message.content);
      if (injected) {
        broadcastToGui({
          type: "agent_message",
          payload: { id: `gui_${Date.now()}`, source: "claude", content: message.content, timestamp: Date.now() },
          timestamp: Date.now(),
        });
      }
      return;
    }
    case "get_status":
      sendGuiEvent(ws, { type: "daemon_status", payload: currentStatus(), timestamp: Date.now() });
      return;
    case "launch_codex_tui": {
      if (!codexBootstrapped) {
        sendGuiEvent(ws, { type: "system_log", payload: { level: "error", message: "Codex app-server is not ready yet." }, timestamp: Date.now() });
        return;
      }
      if (codex.activeThreadId) {
        sendGuiEvent(ws, { type: "system_log", payload: { level: "warn", message: "Codex session is already active." }, timestamp: Date.now() });
        return;
      }
      log("Initializing Codex session from GUI...");
      broadcastToGui({ type: "system_log", payload: { level: "info", message: "Connecting to Codex..." }, timestamp: Date.now() });

      codex.initSession().then((result) => {
        if (result.success) {
          log("Codex session initialized successfully");
          tuiConnectionState.markBridgeReady();
          broadcastToGui({ type: "agent_status", payload: { agent: "codex", status: "connected", threadId: codex.activeThreadId }, timestamp: Date.now() });
          broadcastToGui({ type: "system_log", payload: { level: "info", message: `Codex connected! Thread: ${codex.activeThreadId}` }, timestamp: Date.now() });
          broadcastStatus();
        } else {
          log(`Codex session init failed: ${result.error}`);
          broadcastToGui({ type: "system_log", payload: { level: "error", message: `Codex connection failed: ${result.error}` }, timestamp: Date.now() });
        }
      });
      return;
    }
    case "stop_codex_tui": {
      log("Disconnecting Codex from GUI...");
      codex.disconnect();
      tuiConnectionState.handleCodexExit();
      broadcastToGui({ type: "agent_status", payload: { agent: "codex", status: "disconnected" }, timestamp: Date.now() });
      broadcastToGui({ type: "system_log", payload: { level: "info", message: "Codex disconnected." }, timestamp: Date.now() });
      broadcastStatus();
      return;
    }
  }
}

function sendGuiEvent(ws: ServerWebSocket<GuiSocketData>, event: GuiEvent) {
  try { ws.send(JSON.stringify(event)); } catch {}
}

function broadcastToGui(event: GuiEvent) {
  const data = JSON.stringify(event);
  for (const ws of guiClients) {
    try { ws.send(data); } catch {}
  }
}

// ── Control message handling ─────────────────────────────

function handleControlMessage(ws: ServerWebSocket<ControlSocketData>, raw: string | Buffer) {
  let message: ControlClientMessage;
  try {
    const text = typeof raw === "string" ? raw : raw.toString();
    message = JSON.parse(text);
  } catch { return; }

  switch (message.type) {
    case "claude_connect":
      attachClaude(ws);
      return;
    case "claude_disconnect":
      detachClaude(ws, "frontend requested disconnect");
      return;
    case "status":
      sendStatus(ws);
      return;
    case "claude_to_codex": {
      if (message.message.source !== "claude") {
        sendProtocolMessage(ws, {
          type: "claude_to_codex_result", requestId: message.requestId,
          success: false, error: "Invalid message source",
        });
        return;
      }
      if (!tuiConnectionState.canReply()) {
        sendProtocolMessage(ws, {
          type: "claude_to_codex_result", requestId: message.requestId,
          success: false, error: "Codex is not ready. Wait for TUI to connect and create a thread.",
        });
        return;
      }
      log(`Forwarding Claude -> Codex (${message.message.content.length} chars)`);
      const injected = codex.injectMessage(message.message.content);

      // Broadcast to GUI
      broadcastToGui({
        type: "agent_message",
        payload: message.message,
        timestamp: Date.now(),
      });

      sendProtocolMessage(ws, {
        type: "claude_to_codex_result", requestId: message.requestId,
        success: injected, error: injected ? undefined : "Injection failed.",
      });
      return;
    }
  }
}

function attachClaude(ws: ServerWebSocket<ControlSocketData>) {
  if (attachedClaude && attachedClaude !== ws) {
    attachedClaude.close(4001, "replaced by a newer Claude session");
  }

  attachedClaude = ws;
  ws.data.attached = true;
  log(`Claude frontend attached (#${ws.data.clientId})`);
  broadcastToGui({ type: "agent_status", payload: { agent: "claude", status: "connected" }, timestamp: Date.now() });

  sendStatus(ws);

  if (bufferedMessages.length > 0) {
    flushBufferedMessages(ws);
  } else if (tuiConnectionState.canReply()) {
    sendBridgeMessage(ws, systemMessage("system_ready", currentReadyMessage()));
  } else if (codexBootstrapped) {
    sendBridgeMessage(ws, systemMessage("system_waiting", currentWaitingMessage()));
    sendBridgeMessage(ws, systemMessage("system_attach_cmd", attachCmd));
  }

  if (tuiConnectionState.canReply()) {
    notifyCodexClaudeOnline();
  }
}

function detachClaude(ws: ServerWebSocket<ControlSocketData>, reason: string) {
  if (attachedClaude !== ws) return;
  attachedClaude = null;
  ws.data.attached = false;
  log(`Claude frontend detached (#${ws.data.clientId}, ${reason})`);
  broadcastToGui({ type: "agent_status", payload: { agent: "claude", status: "disconnected" }, timestamp: Date.now() });

  if (tuiConnectionState.canReply()) {
    codex.injectMessage("Claude Code went offline. AgentBridge is still running.");
  }
}

function emitToClaude(message: BridgeMessage) {
  if (attachedClaude && attachedClaude.readyState === WebSocket.OPEN) {
    sendBridgeMessage(attachedClaude, message);
    return;
  }
  bufferedMessages.push(message);
  if (bufferedMessages.length > MAX_BUFFERED_MESSAGES) {
    bufferedMessages.splice(0, bufferedMessages.length - MAX_BUFFERED_MESSAGES);
  }
}

function flushBufferedMessages(ws: ServerWebSocket<ControlSocketData>) {
  const messages = bufferedMessages.splice(0, bufferedMessages.length);
  for (const message of messages) {
    sendBridgeMessage(ws, message);
  }
}

function sendBridgeMessage(ws: ServerWebSocket<ControlSocketData>, message: BridgeMessage) {
  sendProtocolMessage(ws, { type: "codex_to_claude", message });
}

function sendStatus(ws: ServerWebSocket<ControlSocketData>) {
  sendProtocolMessage(ws, { type: "status", status: currentStatus() });
}

function broadcastStatus() {
  if (attachedClaude) sendStatus(attachedClaude);
  broadcastToGui({ type: "daemon_status", payload: currentStatus(), timestamp: Date.now() });
}

function sendProtocolMessage(ws: ServerWebSocket<ControlSocketData>, message: ControlServerMessage) {
  try { ws.send(JSON.stringify(message)); } catch (err: any) {
    log(`Failed to send control message: ${err.message}`);
  }
}

function currentStatus() {
  const snapshot = tuiConnectionState.snapshot();
  return {
    bridgeReady: tuiConnectionState.canReply(),
    tuiConnected: snapshot.tuiConnected,
    threadId: codex.activeThreadId,
    queuedMessageCount: bufferedMessages.length,
    proxyUrl: codex.proxyUrl,
    appServerUrl: codex.appServerUrl,
    pid: process.pid,
    codexBootstrapped,
    codexTuiRunning: tuiConnectionState.canReply() || codex.activeThreadId !== null,
    claudeConnected: attachedClaude !== null,
  };
}

function currentWaitingMessage() {
  return "AgentBridge started, waiting for Codex TUI to connect.";
}

function currentReadyMessage() {
  return `Codex TUI connected, session thread created (${codex.activeThreadId}). Bridge is fully operational.`;
}

function notifyCodexClaudeOnline() {
  codex.injectMessage("AgentBridge connected to Claude Code. You can now communicate with Claude bidirectionally.");
}

function systemMessage(idPrefix: string, content: string): BridgeMessage {
  return {
    id: `${idPrefix}_${++nextSystemMessageId}`,
    source: "codex",
    content,
    timestamp: Date.now(),
  };
}

function writePidFile() {
  writeFileSync(PID_FILE, `${process.pid}\n`, "utf-8");
}

function removePidFile() {
  try { unlinkSync(PID_FILE); } catch {}
}

async function bootCodex() {
  log("Starting AgentBridge daemon...");
  log(`Codex app-server: ${codex.appServerUrl}`);
  log(`Codex proxy: ${codex.proxyUrl}`);
  log(`Control server: ws://127.0.0.1:${CONTROL_PORT}/ws`);
  log(`GUI server: ws://127.0.0.1:${GUI_PORT}`);

  try {
    await codex.start();
    codexBootstrapped = true;

    emitToClaude(systemMessage("system_waiting", currentWaitingMessage()));
    emitToClaude(systemMessage("system_attach_cmd", attachCmd));
    broadcastStatus();
  } catch (err: any) {
    log(`Failed to start Codex: ${err.message}`);
    emitToClaude(
      systemMessage("system_codex_start_failed", `AgentBridge failed to start Codex app-server: ${err.message}`),
    );
    broadcastToGui({ type: "agent_status", payload: { agent: "codex", status: "error", error: err.message }, timestamp: Date.now() });
    broadcastStatus();
  }
}

function shutdown(reason: string) {
  if (shuttingDown) return;
  shuttingDown = true;
  log(`Shutting down daemon (${reason})...`);
  tuiConnectionState.dispose(`daemon shutdown (${reason})`);
  controlServer?.stop();
  guiServer?.stop();
  controlServer = null;
  guiServer = null;
  codex.stop();
  removePidFile();
  process.exit(0);
}

process.on("SIGINT", () => shutdown("SIGINT"));
process.on("SIGTERM", () => shutdown("SIGTERM"));
process.on("exit", () => removePidFile());
process.on("uncaughtException", (err) => {
  log(`UNCAUGHT EXCEPTION: ${err.stack ?? err.message}`);
});
process.on("unhandledRejection", (reason: any) => {
  log(`UNHANDLED REJECTION: ${reason?.stack ?? reason}`);
});

function log(msg: string) {
  const line = `[${new Date().toISOString()}] [AgentBridgeDaemon] ${msg}\n`;
  process.stderr.write(line);
  try { appendFileSync(LOG_FILE, line); } catch {}
}

writePidFile();
startControlServer();
startGuiServer();
void bootCodex();
