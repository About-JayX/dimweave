#!/usr/bin/env bun

import { appendFileSync, unlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { CodexAdapter } from "./adapters/codex-adapter";
import { TuiConnectionState } from "./tui-connection-state";
import { SessionManager } from "./session-manager";
import { state, broadcastToGui } from "./daemon-state";
import { startControlServer, routeMessage } from "./control-server";
import { startGuiServer } from "./gui-server";
import { registerCodexEvents } from "./codex-events";

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
const sessionManager = new SessionManager();
const attachCmd = `codex --enable tui_app_server --remote ${codex.proxyUrl}`;

const tuiState = new TuiConnectionState({
  disconnectGraceMs: TUI_DISCONNECT_GRACE_MS,
  log,
  onDisconnectPersisted: (connId) => {
    routeMessage(
      state.systemMessage(
        "system_tui_disconnected",
        `Codex TUI disconnected (conn #${connId}). Codex is still running in the background.`,
        state.claudeRole,
      ),
    );
  },
  onReconnectAfterNotice: (connId) => {
    routeMessage(
      state.systemMessage(
        "system_tui_reconnected",
        `Codex TUI reconnected (conn #${connId}). Bridge restored.`,
        state.claudeRole,
      ),
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
    claudeConnected: state.attachedAgents.has("claude"),
    codexAccount: codex.accountInfo,
    claudeRole: state.claudeRole,
    codexRole: state.codexRole,
    claudeOnline: state.attachedAgents.has("claude"),
    codexOnline: state.attachedAgents.has("codex"),
  };
}

function broadcastStatus() {
  // Send status to all attached agents
  for (const [, ws] of state.attachedAgents) {
    try {
      ws.send(JSON.stringify({ type: "status", status: currentStatus() }));
    } catch {}
  }
  broadcastToGui({
    type: "daemon_status",
    payload: currentStatus(),
    timestamp: Date.now(),
  });
}

const serverDeps = {
  codex,
  tuiState,
  sessionManager,
  currentStatus,
  broadcastStatus,
  log,
  attachCmd,
};

// ── Dynamic tool handler (Codex calls reply/check_messages/get_status) ──

codex.setDynamicToolHandler(async (toolName, args) => {
  if (toolName === "reply") {
    const msg: import("./types").BridgeMessage = {
      id: `codex_${Date.now()}`,
      from: state.codexRole,
      to: args.to ?? "lead",
      content: args.text ?? "",
      timestamp: Date.now(),
    };
    const result = routeMessage(msg, { skipSender: "codex" });
    return {
      content: [
        {
          type: "text",
          text: result.success
            ? `Message routed to ${args.to}.`
            : `Error: ${result.error}`,
        },
      ],
    };
  }
  if (toolName === "check_messages") {
    const messages = state.flushBufferedMessages();
    if (messages.length === 0) {
      return { content: [{ type: "text", text: "No new messages." }] };
    }
    const formatted = messages
      .map(
        (m) =>
          `[${new Date(m.timestamp).toLocaleTimeString()}] ${m.from}: ${m.content}`,
      )
      .join("\n\n---\n\n");
    return {
      content: [
        {
          type: "text",
          text: `${messages.length} message(s):\n\n${formatted}`,
        },
      ],
    };
  }
  if (toolName === "get_status") {
    const s = currentStatus();
    const lines = [
      `Bridge ready: ${s.bridgeReady ? "yes" : "no"}`,
      "",
      "Available roles:",
      `  ${s.claudeRole} (claude) - ${s.claudeOnline ? "online" : "offline"}`,
      `  ${s.codexRole} (codex) - online`,
    ];
    return { content: [{ type: "text", text: lines.join("\n") }] };
  }
  return { content: [{ type: "text", text: `Unknown tool: ${toolName}` }] };
});

// ── Codex events (GUI display + lifecycle only) ───────────

registerCodexEvents({
  codex,
  tuiState,
  broadcastToGui,
  broadcastStatus,
  state,
  log,
});

// ── Bootstrap ──────────────────────────────────────────────

async function bootCodex() {
  log("Starting AgentBridge daemon...");
  log(
    `Codex: ${codex.appServerUrl} | Proxy: ${codex.proxyUrl} | Control: ws://127.0.0.1:${CONTROL_PORT}/ws | GUI: ws://127.0.0.1:${GUI_PORT}`,
  );

  try {
    // Create isolated CODEX_HOME with mcp.json for agent communication
    const bridgePath = join(import.meta.dir, "bridge.ts");
    const session = sessionManager.createSession({
      sessionId: `boot-${Date.now()}`,
      roleId: state.codexRole,
      bridgePath,
      controlPort: CONTROL_PORT,
    });
    state.codexHome = session.codexHome;
    await codex.start({
      codexHome: session.codexHome,
      bridgePath,
      controlPort: CONTROL_PORT,
    });
    state.codexBootstrapped = true;
    // MCP bridge will be spawned by app-server from CODEX_HOME/config.toml
    broadcastStatus();
  } catch (err: any) {
    log(`Failed to start Codex: ${err.message}`);
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
  sessionManager.cleanupAll();
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
