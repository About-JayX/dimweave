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
  routeMessage: (
    msg: BridgeMessage,
    opts?: { skipGuiBroadcast?: boolean },
  ) => void;
  sendToClaudePty: (text: string) => boolean;
  state: typeof DaemonState;
  log: (msg: string) => void;
}

/**
 * Register all Codex EventEmitter handlers.
 * Extracted from daemon.ts to keep the entry point thin.
 */
export function registerCodexEvents(deps: CodexEventDeps): void {
  const {
    codex,
    tuiState,
    broadcastToGui,
    broadcastStatus,
    routeMessage,
    sendToClaudePty,
    state,
    log,
  } = deps;

  // Buffer the last agentMessage per turn — only forward on turnCompleted
  let lastCodexMessage: BridgeMessage | null = null;

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
    // Enrich with role-based from/to
    msg.from = state.codexRole;
    msg.to = ROLES[state.codexRole].defaultTarget;
    log(
      `Codex agentMessage (${msg.content.length} chars) — buffered for turn end`,
    );
    lastCodexMessage = msg;

    broadcastToGui({
      type: "agent_message",
      payload: msg,
      timestamp: Date.now(),
    });
  });

  codex.on("turnCompleted", () => {
    log("Codex turn completed");

    if (lastCodexMessage) {
      const content = lastCodexMessage.content;
      const codexRole = ROLES[state.codexRole];
      const targetRole = lastCodexMessage.to || "lead";

      // Route via bridge (skip GUI broadcast — already done during streaming)
      routeMessage(lastCodexMessage, { skipGuiBroadcast: true });

      // Also inject into Claude PTY if target matches claudeRole
      if (state.claudeRole === targetRole && state.attachedClaude) {
        const MAX_INJECT_LEN = 500;
        const replyReminder =
          "You MUST respond using the agentbridge reply tool so your response reaches the other agent.";
        const preamble = codexRole.forwardPrompt
          ? `${codexRole.forwardPrompt}\n`
          : "";
        let inject: string;
        if (content.length <= MAX_INJECT_LEN) {
          inject = `${preamble}${codexRole.label} says: ${content}\n\n${replyReminder}`;
        } else {
          const summary = content.slice(0, MAX_INJECT_LEN).trimEnd();
          inject = `${preamble}${codexRole.label} says: ${summary}... (truncated, use check_messages for full content)\n\n${replyReminder}`;
        }
        const injected = sendToClaudePty(inject);

        broadcastToGui({
          type: "system_log",
          payload: {
            level: injected ? "info" : "warn",
            message: injected
              ? `Codex (${state.codexRole}) → ${targetRole}. ${content.length > MAX_INJECT_LEN ? "Summary" : "Full content"} injected.`
              : `Codex (${state.codexRole}) → ${targetRole} but PTY inject failed.`,
          },
          timestamp: Date.now(),
        });
      } else {
        broadcastToGui({
          type: "system_log",
          payload: {
            level: "info",
            message: `Codex (${state.codexRole}) → ${targetRole}. Routed via bridge.`,
          },
          timestamp: Date.now(),
        });
      }

      lastCodexMessage = null;
    }
  });

  codex.on("ready", (threadId: string) => {
    tuiState.markBridgeReady();
    log(`Codex ready - thread ${threadId}. Bridge fully operational`);
    routeMessage(
      state.systemMessage(
        "system_ready",
        `Codex TUI connected, session thread created (${threadId}). Bridge is fully operational.`,
        state.claudeRole,
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
    routeMessage(
      state.systemMessage(
        "system_codex_exit",
        `Codex app-server exited (code ${code ?? "unknown"}).`,
        state.claudeRole,
      ),
    );
    broadcastToGui({
      type: "agent_status",
      payload: { agent: "codex", status: "disconnected", exitCode: code },
      timestamp: Date.now(),
    });
    broadcastStatus();
  });
}
