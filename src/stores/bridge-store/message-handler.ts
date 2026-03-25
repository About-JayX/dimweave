import { invoke } from "@tauri-apps/api/core";
import type { GuiEvent, BridgeMessage, DaemonStatus } from "@/types";
import type { CodexPhase, BridgeState } from "./types";

type SetFn = (
  partial:
    | Partial<BridgeState>
    | ((state: BridgeState) => Partial<BridgeState>),
) => void;

export function handleGuiEvent(guiEvent: GuiEvent, set: SetFn) {
  switch (guiEvent.type) {
    case "agent_message_started":
      set((s) => ({
        messages: [
          ...s.messages,
          {
            id: guiEvent.payload.id,
            from: guiEvent.payload.from ?? guiEvent.payload.source ?? "unknown",
            to: guiEvent.payload.to ?? "",
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
      // Final complete message -- replace the streaming placeholder or add new
      set((s) => {
        const idx = s.messages.findIndex((m) => m.id === guiEvent.payload.id);
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

    case "pty_inject":
      // Daemon wants to write to Claude PTY (e.g. Codex turn completed)
      invoke("pty_write", { data: guiEvent.payload.data }).catch((err) => {
        console.warn("[pty_inject] Failed to write to PTY:", err);
      });
      break;

    case "terminal_output":
      if (guiEvent.payload.agent === "claude_clear") {
        set((s) => ({
          terminalLines: s.terminalLines.filter((l) => l.agent !== "claude"),
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
      }));
      break;
    }

    case "role_sync": {
      const { claudeRole, codexRole } = guiEvent.payload;
      set({ claudeRole, codexRole });
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
            from: "system",
            to: "user",
            content: guiEvent.payload.message,
            timestamp: guiEvent.timestamp,
          },
        ],
        terminalLines: [
          ...s.terminalLines.slice(-200),
          {
            agent: "system",
            kind: guiEvent.payload.level === "error" ? "error" : "text",
            line: guiEvent.payload.message,
            timestamp: guiEvent.timestamp,
          },
        ],
      }));
      break;
  }
}
