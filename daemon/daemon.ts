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

/** Inject collaboration protocol into Codex exactly once for each new session. */
function injectCodexProtocol() {
  codex.injectMessage(
    `AgentBridge is active. You are connected to Claude Code via a bridge.

## Your Role:
- You are a CODE REVIEWER and PLAN GENERATOR only.
- DO NOT modify, create, or delete any files directly.
- DO NOT run shell commands that change the codebase.
- Your job: analyze code, generate plans, review changes, suggest improvements.
- If code changes are needed, describe WHAT to change and include "@claude" so Claude Code executes the changes.

## Collaboration Protocol:
- Include "@claude" when you have a plan or review that requires code changes.
- Respond normally WITHOUT "@claude" when your analysis is complete and no action is needed.
- Example: "I found a SQL injection vulnerability in auth.ts line 42. @claude please fix by using parameterized queries."
- Example (no trigger): "Code review complete. The implementation looks correct, no changes needed."`,
  );
}

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

codex.on("agentMessage", (msg: BridgeMessage) => {
  if (msg.source !== "codex") return;
  log(`Forwarding Codex -> Claude (${msg.content.length} chars)`);
  emitToClaude(msg);

  // Only forward to Claude PTY if Codex explicitly requests it.
  // Codex includes "@claude" or "[need_review]" to signal Claude should act.
  // Otherwise the message is display-only — prevents infinite conversation loops.
  const needsClaude =
    /(@claude|@Claude|\[need_review\]|\[needs_action\])/i.test(msg.content);
  if (needsClaude) {
    const sent = sendToClaudePty(
      `[Codex requests your review] Respond, then stop:\n${msg.content}`,
    );
    if (sent) log("Forwarded Codex message to Claude PTY (explicit request)");
  }

  broadcastToGui({
    type: "agent_message",
    payload: msg,
    timestamp: Date.now(),
  });
});

codex.on("turnCompleted", () => log("Codex turn completed"));

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
  injectCodexProtocol();
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
