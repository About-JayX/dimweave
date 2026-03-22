import { create } from "zustand";
import type { GuiEvent, BridgeMessage, AgentInfo, DaemonStatus } from "@/types";

const GUI_WS_URL = "ws://127.0.0.1:4503";
const RECONNECT_INTERVAL = 3000;

export type CodexPhase = "thinking" | "streaming" | "idle";

export interface TerminalLine {
  agent: string;
  kind: string;
  line: string;
  timestamp: number;
}

interface BridgeState {
  connected: boolean;
  messages: BridgeMessage[];
  agents: Record<string, AgentInfo>;
  daemonStatus: DaemonStatus | null;
  codexPhase: CodexPhase;
  terminalLines: TerminalLine[];
  claudeRateLimit: {
    status: string;
    rateLimitType: string;
    resetsAt: number;
  } | null;
  claudePtyRunning: boolean;
  claudeRole: string;
  codexRole: string;

  sendToCodex: (content: string) => void;
  clearMessages: () => void;
  launchCodexTui: () => void;
  stopCodexTui: () => void;
  applyConfig: (config: {
    model?: string;
    reasoningEffort?: string;
    cwd?: string;
  }) => void;
  launchClaude: (cwd?: string, cols?: number, rows?: number) => void;
  sendPtyInput: (data: string) => void;
  resizePty: (cols: number, rows: number) => void;
  stopClaude: () => void;
  setAgentRole: (agent: string, role: string) => void;
  onPtyData: ((data: string) => void) | null;
}

let ws: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

function sendWs(data: object) {
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(data));
  }
}

export const useBridgeStore = create<BridgeState>((set, get) => {
  function connect() {
    if (ws?.readyState === WebSocket.OPEN) return;

    const socket = new WebSocket(GUI_WS_URL);

    socket.onopen = () => {
      set({ connected: true });
      if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
      }
    };

    socket.onmessage = (event) => {
      let guiEvent: GuiEvent;
      try {
        guiEvent = JSON.parse(event.data);
      } catch {
        return;
      }

      switch (guiEvent.type) {
        case "agent_message_started":
          set((s) => ({
            messages: [
              ...s.messages,
              {
                id: guiEvent.payload.id,
                source: guiEvent.payload.source as BridgeMessage["source"],
                content: "",
                timestamp: guiEvent.payload.timestamp,
              },
            ],
          }));
          break;

        case "agent_message_delta":
          set((s) => ({
            messages: s.messages.map((m) =>
              m.id === guiEvent.payload.id
                ? { ...m, content: m.content + guiEvent.payload.delta }
                : m,
            ),
          }));
          break;

        case "agent_message":
          // Final complete message — replace the streaming placeholder or add new
          set((s) => {
            const idx = s.messages.findIndex(
              (m) => m.id === guiEvent.payload.id,
            );
            if (idx >= 0) {
              const updated = [...s.messages];
              updated[idx] = guiEvent.payload as BridgeMessage;
              return { messages: updated };
            }
            return {
              messages: [...s.messages, guiEvent.payload as BridgeMessage],
            };
          });
          break;

        case "codex_phase":
          set({ codexPhase: guiEvent.payload.phase as CodexPhase });
          break;

        case "claude_rate_limit":
          set({ claudeRateLimit: guiEvent.payload });
          break;

        case "pty_data": {
          const cb = get().onPtyData;
          if (cb) cb(guiEvent.payload.data);
          break;
        }

        case "terminal_output":
          if (guiEvent.payload.agent === "claude_clear") {
            set((s) => ({
              terminalLines: s.terminalLines.filter(
                (l) => l.agent !== "claude",
              ),
              claudeRateLimit: null,
            }));
          } else {
            set((s) => ({
              terminalLines: [
                ...s.terminalLines.slice(-200),
                {
                  agent: guiEvent.payload.agent,
                  kind: guiEvent.payload.kind ?? "text",
                  line: guiEvent.payload.line ?? "",
                  timestamp: guiEvent.timestamp,
                },
              ],
            }));
          }
          break;

        case "agent_status": {
          const { agent, status, error, threadId } = guiEvent.payload;
          set((s) => ({
            agents: {
              ...s.agents,
              [agent]: {
                ...s.agents[agent],
                name: agent,
                displayName: s.agents[agent]?.displayName ?? agent,
                status,
                error,
                threadId,
              },
            },
            ...(agent === "claude" && status === "disconnected"
              ? { claudePtyRunning: false }
              : {}),
          }));
          break;
        }

        case "daemon_status":
          set({ daemonStatus: guiEvent.payload as DaemonStatus });
          break;

        case "system_log":
          set((s) => ({
            messages: [
              ...s.messages,
              {
                id: `log_${Date.now()}`,
                source: "system" as const,
                content: guiEvent.payload.message,
                timestamp: guiEvent.timestamp,
              },
            ],
          }));
          break;
      }
    };

    socket.onclose = () => {
      set({ connected: false });
      ws = null;
      reconnectTimer = setTimeout(connect, RECONNECT_INTERVAL);
    };

    socket.onerror = () => {
      socket.close();
    };

    ws = socket;
  }

  // Auto-connect on store creation
  connect();

  return {
    connected: false,
    messages: [],
    agents: {
      claude: {
        name: "claude",
        displayName: "Claude Code",
        status: "disconnected",
      },
      codex: { name: "codex", displayName: "Codex", status: "disconnected" },
    },
    daemonStatus: null,
    codexPhase: "idle" as CodexPhase,
    terminalLines: [],
    claudeRateLimit: null,
    claudePtyRunning: false,
    claudeRole: "lead",
    codexRole: "coder",

    sendToCodex: (content) => sendWs({ type: "send_to_codex", content }),
    clearMessages: () => set({ messages: [] }),
    launchCodexTui: () => sendWs({ type: "launch_codex_tui" }),
    stopCodexTui: () => sendWs({ type: "stop_codex_tui" }),
    applyConfig: (config: {
      model?: string;
      reasoningEffort?: string;
      cwd?: string;
    }) => sendWs({ type: "apply_config", ...config }),
    launchClaude: (cwd?, cols?, rows?) => {
      sendWs({ type: "launch_claude", cwd, cols, rows });
      set({ claudePtyRunning: true });
    },
    sendPtyInput: (data) => sendWs({ type: "pty_input", data }),
    resizePty: (cols, rows) => sendWs({ type: "pty_resize", cols, rows }),
    stopClaude: () => {
      sendWs({ type: "stop_claude" });
      set({ claudePtyRunning: false });
    },
    setAgentRole: (agent, role) => {
      sendWs({ type: "set_agent_role", agent, role });
      if (agent === "claude") set({ claudeRole: role });
      else if (agent === "codex") set({ codexRole: role });
    },
    onPtyData: null,
  };
});
