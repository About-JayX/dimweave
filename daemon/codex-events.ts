import type { CodexAdapter } from "./adapters/codex-adapter";
import type { TuiConnectionState } from "./tui-connection-state";
import type { BridgeMessage } from "./types";
import type { GuiEvent } from "./daemon-state";
import type { state as DaemonState } from "./daemon-state";
import { ROLES } from "./role-config";

export interface CodexEventDeps {
  codex: CodexAdapter;
  tuiState: TuiConnectionState;
  broadcastToGui: (event: GuiEvent) => void;
  broadcastStatus: () => void;
  state: typeof DaemonState;
  log: (msg: string) => void;
}

/**
 * Register Codex EventEmitter handlers.
 * With MCP integration, Codex communicates via MCP tools (reply/check_messages).
 * These handlers only manage GUI display and lifecycle events.
 */
export function registerCodexEvents(deps: CodexEventDeps): void {
  const { codex, tuiState, broadcastToGui, broadcastStatus, state, log } = deps;

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
      payload: {
        id,
        from: state.codexRole,
        to: ROLES[state.codexRole].defaultTarget,
        content: "",
        timestamp: Date.now(),
      },
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
    msg.from = state.codexRole;
    msg.to = ROLES[state.codexRole].defaultTarget;
    log(
      `Codex agentMessage [from:${msg.from} to:${msg.to}] (${msg.content.length} chars)`,
    );

    // GUI display + log
    broadcastToGui({
      type: "agent_message",
      payload: msg,
      timestamp: Date.now(),
    });
    const preview =
      msg.content.length > 80 ? msg.content.slice(0, 80) + "..." : msg.content;
    broadcastToGui({
      type: "system_log",
      payload: {
        level: "info",
        message: `[Codex] ${msg.from} → ${msg.to} | "${preview}"`,
      },
      timestamp: Date.now(),
    });
  });

  codex.on("turnCompleted", () => {
    log("Codex turn completed");
  });

  codex.on("ready", (threadId: string) => {
    tuiState.markBridgeReady();
    log(`Codex ready - thread ${threadId}. Bridge fully operational`);
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
    broadcastToGui({
      type: "agent_status",
      payload: { agent: "codex", status: "disconnected", exitCode: code },
      timestamp: Date.now(),
    });
    broadcastStatus();
  });
}
