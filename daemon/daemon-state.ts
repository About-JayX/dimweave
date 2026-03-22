import type { ServerWebSocket } from "bun";
import type { BridgeMessage } from "./types";

export interface ControlSocketData {
  clientId: number;
  attached: boolean;
}

export interface GuiSocketData {
  clientId: number;
}

export interface GuiEvent {
  type:
    | "agent_message"
    | "agent_message_started"
    | "agent_message_delta"
    | "codex_phase"
    | "terminal_output"
    | "claude_rate_limit"
    | "agent_status"
    | "system_log"
    | "daemon_status";
  payload: any;
  timestamp: number;
}

const MAX_BUFFERED_MESSAGES = parseInt(
  process.env.AGENTBRIDGE_MAX_BUFFERED_MESSAGES ?? "100",
  10,
);

/** Shared mutable state for the daemon. */
class DaemonState {
  controlServer: ReturnType<typeof Bun.serve> | null = null;
  guiServer: ReturnType<typeof Bun.serve> | null = null;
  attachedClaude: ServerWebSocket<ControlSocketData> | null = null;

  nextControlClientId = 0;
  nextGuiClientId = 0;
  nextSystemMessageId = 0;
  codexBootstrapped = false;
  shuttingDown = false;

  readonly bufferedMessages: BridgeMessage[] = [];
  readonly guiClients = new Set<ServerWebSocket<GuiSocketData>>();

  bufferMessage(message: BridgeMessage) {
    this.bufferedMessages.push(message);
    if (this.bufferedMessages.length > MAX_BUFFERED_MESSAGES) {
      this.bufferedMessages.splice(
        0,
        this.bufferedMessages.length - MAX_BUFFERED_MESSAGES,
      );
    }
  }

  flushBufferedMessages(): BridgeMessage[] {
    return this.bufferedMessages.splice(0, this.bufferedMessages.length);
  }

  systemMessage(idPrefix: string, content: string): BridgeMessage {
    return {
      id: `${idPrefix}_${++this.nextSystemMessageId}`,
      source: "codex",
      content,
      timestamp: Date.now(),
    };
  }
}

export const state = new DaemonState();

// ── Broadcast helpers ──────────────────────────────────────

export function sendGuiEvent(
  ws: ServerWebSocket<GuiSocketData>,
  event: GuiEvent,
) {
  try {
    ws.send(JSON.stringify(event));
  } catch {}
}

export function broadcastToGui(event: GuiEvent) {
  const data = JSON.stringify(event);
  for (const ws of state.guiClients) {
    try {
      ws.send(data);
    } catch {}
  }
}
