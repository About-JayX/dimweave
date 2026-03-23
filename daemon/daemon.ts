#!/usr/bin/env bun

import { appendFileSync, unlinkSync, writeFileSync } from "node:fs";
import { CodexAdapter } from "./adapters/codex-adapter";
import { TuiConnectionState } from "./tui-connection-state";
import { state, broadcastToGui } from "./daemon-state";
import { startControlServer, emitToClaude } from "./control-server";
import { startGuiServer, sendToClaudePty } from "./gui-server";
import type { BridgeMessage } from "./types";

// ── Config ─────────────────────────────────────────────────

const CODEX_APP_PORT = parseInt(process.env.CODEX_WS_PORT ?? "4500", 10);
const CODEX_PROXY_PORT = parseInt(process.env.CODEX_PROXY_PORT ?? "4501", 10);
const CONTROL_PORT = parseInt(
  process.env.AGENTBRIDGE_CONTROL_PORT ?? "4502",
  10,
);
const GUI_PORT = parseInt(process.env.AGENTBRIDGE_GUI_PORT ?? "4503", 10);
const PID_FILE =
  process.env.AGENTBRIDGE_PID_FILE ??
  `/tmp/agentbridge-daemon-${CONTROL_PORT}.pid`;
const LOG_FILE = "/tmp/agentbridge.log";
const TUI_DISCONNECT_GRACE_MS = parseInt(
  process.env.TUI_DISCONNECT_GRACE_MS ?? "2500",
  10,
);

// ── Core instances ─────────────────────────────────────────

const codex = new CodexAdapter(CODEX_APP_PORT, CODEX_PROXY_PORT);
const attachCmd = `codex --enable tui_app_server --remote ${codex.proxyUrl}`;

const tuiState = new TuiConnectionState({
  disconnectGraceMs: TUI_DISCONNECT_GRACE_MS,
  log,
  onDisconnectPersisted: (connId) => {
    emitToClaude(
      state.systemMessage(
        "system_tui_disconnected",
        `Codex TUI disconnected (conn #${connId}). Codex is still running in the background.`,
      ),
    );
  },
  onReconnectAfterNotice: (connId) => {
    emitToClaude(
      state.systemMessage(
        "system_tui_reconnected",
        `Codex TUI reconnected (conn #${connId}). Bridge restored.`,
      ),
    );
    codex.injectMessage(
      "Claude Code is still online, bridge restored. Bidirectional communication can continue.",
    );
  },
});

// ── Shared deps for servers ────────────────────────────────

function currentStatus() {
  const snapshot = tuiState.snapshot();
  return {
    bridgeReady: tuiState.canReply(),
    tuiConnected: snapshot.tuiConnected,
    threadId: codex.activeThreadId,
    queuedMessageCount: state.bufferedMessages.length,
    proxyUrl: codex.proxyUrl,
    appServerUrl: codex.appServerUrl,
    pid: process.pid,
    codexBootstrapped: state.codexBootstrapped,
    codexTuiRunning: tuiState.canReply() || codex.activeThreadId !== null,
    claudeConnected: state.attachedClaude !== null,
    codexAccount: codex.accountInfo,
  };
}

function broadcastStatus() {
  if (state.attachedClaude) {
    try {
      state.attachedClaude.send(
        JSON.stringify({ type: "status", status: currentStatus() }),
      );
    } catch {}
  }
  broadcastToGui({
    type: "daemon_status",
    payload: currentStatus(),
    timestamp: Date.now(),
  });
}

import { ROLES } from "./role-config";

const serverDeps = {
  codex,
  tuiState,
  currentStatus,
  broadcastStatus,
  log,
  attachCmd,
};

// ── Codex events ───────────────────────────────────────────

codex.on("phaseChanged", (phase: string) => {
  broadcastToGui({
    type: "codex_phase",
    payload: { phase },
    timestamp: Date.now(),
  });
});

codex.on("agentMessageStarted", (id: string) => {
  broadcastToGui({
    type: "agent_message_started",
    payload: { id, source: "codex", content: "", timestamp: Date.now() },
    timestamp: Date.now(),
  });
});

codex.on("agentMessageDelta", (id: string, delta: string) => {
  broadcastToGui({
    type: "agent_message_delta",
    payload: { id, delta },
    timestamp: Date.now(),
  });
});

// Buffer the last agentMessage per turn — only forward to Claude on turnCompleted
let lastCodexMessage: BridgeMessage | null = null;

codex.on("agentMessage", (msg: BridgeMessage) => {
  if (msg.source !== "codex") return;
  log(
    `Codex agentMessage (${msg.content.length} chars) — buffered for turn end`,
  );
  lastCodexMessage = msg; // Overwrite: only the last one matters

  broadcastToGui({
    type: "agent_message",
    payload: msg,
    timestamp: Date.now(),
  });
});

codex.on("turnCompleted", () => {
  log("Codex turn completed");

  if (lastCodexMessage) {
    // Buffer for MCP check_messages (Claude pulls when ready)
    emitToClaude(lastCodexMessage);

    // Inject Codex output into Claude PTY
    // Short messages: inject full content directly
    // Long messages: inject truncated summary + pointer to check_messages
    const content = lastCodexMessage.content;
    const codexRole = ROLES[state.codexRole];
    const MAX_INJECT_LEN = 500;

    const replyReminder =
      "You MUST respond using the agentbridge reply tool so your response reaches the other agent.";
    let inject: string;
    if (content.length <= MAX_INJECT_LEN) {
      inject = `${codexRole.label} says: ${content}\n\n${replyReminder}`;
    } else {
      const summary = content.slice(0, MAX_INJECT_LEN).trimEnd();
      inject = `${codexRole.label} says: ${summary}... (truncated, use check_messages for full content)\n\n${replyReminder}`;
    }
    const injected = sendToClaudePty(inject);

    // Notify GUI
    broadcastToGui({
      type: "system_log",
      payload: {
        level: injected ? "info" : "warn",
        message: injected
          ? `Codex (${state.codexRole}) completed. ${content.length > MAX_INJECT_LEN ? "Summary" : "Full content"} injected to Claude.`
          : `Codex (${state.codexRole}) completed but no GUI client available — output not delivered to Claude.`,
      },
      timestamp: Date.now(),
    });

    lastCodexMessage = null;
  }
});

codex.on("ready", (threadId: string) => {
  tuiState.markBridgeReady();
  log(`Codex ready - thread ${threadId}. Bridge fully operational`);
  emitToClaude(
    state.systemMessage(
      "system_ready",
      `Codex TUI connected, session thread created (${threadId}). Bridge is fully operational.`,
    ),
  );
  broadcastToGui({
    type: "agent_status",
    payload: { agent: "codex", status: "connected", threadId },
    timestamp: Date.now(),
  });
});

codex.on("tuiConnected", (connId: number) => {
  tuiState.handleTuiConnected(connId);
  log(`Codex TUI connected (conn #${connId})`);
  broadcastStatus();
});

codex.on("tuiDisconnected", (connId: number) => {
  tuiState.handleTuiDisconnected(connId);
  log(`Codex TUI disconnected (conn #${connId})`);
  broadcastToGui({
    type: "agent_status",
    payload: { agent: "codex", status: "disconnected" },
    timestamp: Date.now(),
  });
  broadcastStatus();
});

codex.on("error", (err: Error) => {
  log(`Codex error: ${err.message}`);
  broadcastToGui({
    type: "agent_status",
    payload: { agent: "codex", status: "error", error: err.message },
    timestamp: Date.now(),
  });
});

codex.on("accountInfoUpdated", () => broadcastStatus());

codex.on("exit", (code: number | null) => {
  log(`Codex process exited (code ${code})`);
  tuiState.handleCodexExit();
  emitToClaude(
    state.systemMessage(
      "system_codex_exit",
      `Codex app-server exited (code ${code ?? "unknown"}).`,
    ),
  );
  broadcastToGui({
    type: "agent_status",
    payload: { agent: "codex", status: "disconnected", exitCode: code },
    timestamp: Date.now(),
  });
  broadcastStatus();
});

// ── Bootstrap ──────────────────────────────────────────────

async function bootCodex() {
  log("Starting AgentBridge daemon...");
  log(
    `Codex: ${codex.appServerUrl} | Proxy: ${codex.proxyUrl} | Control: ws://127.0.0.1:${CONTROL_PORT}/ws | GUI: ws://127.0.0.1:${GUI_PORT}`,
  );

  try {
    await codex.start();
    state.codexBootstrapped = true;
    emitToClaude(
      state.systemMessage(
        "system_waiting",
        "AgentBridge started, waiting for Codex TUI to connect.",
      ),
    );
    emitToClaude(state.systemMessage("system_attach_cmd", attachCmd));
    broadcastStatus();
  } catch (err: any) {
    log(`Failed to start Codex: ${err.message}`);
    emitToClaude(
      state.systemMessage(
        "system_codex_start_failed",
        `AgentBridge failed to start Codex: ${err.message}`,
      ),
    );
    broadcastToGui({
      type: "agent_status",
      payload: { agent: "codex", status: "error", error: err.message },
      timestamp: Date.now(),
    });
    broadcastStatus();
  }
}

function shutdown(reason: string) {
  if (state.shuttingDown) return;
  state.shuttingDown = true;
  log(`Shutting down daemon (${reason})...`);
  tuiState.dispose(`daemon shutdown (${reason})`);
  state.controlServer?.stop();
  state.guiServer?.stop();
  codex.stop();
  try {
    unlinkSync(PID_FILE);
  } catch {}
  process.exit(0);
}

function log(msg: string) {
  const line = `[${new Date().toISOString()}] [AgentBridgeDaemon] ${msg}\n`;
  process.stderr.write(line);
  try {
    appendFileSync(LOG_FILE, line);
  } catch {}
}

// ── Process lifecycle ──────────────────────────────────────

process.on("SIGINT", () => shutdown("SIGINT"));
process.on("SIGTERM", () => shutdown("SIGTERM"));
process.on("exit", () => {
  try {
    unlinkSync(PID_FILE);
  } catch {}
});
process.on("uncaughtException", (err) =>
  log(`UNCAUGHT EXCEPTION: ${err.stack ?? err.message}`),
);
process.on("unhandledRejection", (reason: any) =>
  log(`UNHANDLED REJECTION: ${reason?.stack ?? reason}`),
);

writeFileSync(PID_FILE, `${process.pid}\n`, "utf-8");
startControlServer(CONTROL_PORT, serverDeps);
startGuiServer(GUI_PORT, serverDeps);
void bootCodex();
