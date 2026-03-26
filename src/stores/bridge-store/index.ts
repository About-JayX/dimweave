import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { BridgeState } from "./types";
import {
  initListeners,
  logError,
  nextLogId,
  clearUnlisteners,
} from "./helpers";
import { syncStatusSnapshot } from "./sync";

export type { TerminalLine, BridgeState } from "./types";

export const useBridgeStore = create<BridgeState>((set, get) => {
  initListeners(set);
  void syncStatusSnapshot(set);

  return {
    // Daemon is always available (embedded in Tauri process)
    connected: true,
    messages: [],
    agents: {
      claude: {
        name: "claude",
        displayName: "Claude Code",
        status: "disconnected",
      },
      codex: { name: "codex", displayName: "Codex", status: "disconnected" },
    },
    terminalLines: [],
    claudeTerminalChunks: [],
    claudeTerminalRunning: false,
    claudeTerminalExitCode: undefined,
    claudeTerminalDetail: undefined,
    permissionPrompts: [],
    claudeNeedsAttention: false,
    claudeRole: "lead",
    codexRole: "coder",
    codexStream: {
      thinking: false,
      currentDelta: "",
      lastMessage: "",
      turnStatus: "",
    },
    draft: "",

    setDraft: (text) => set({ draft: text }),

    sendToCodex: (content, target) => {
      const { claudeRole, codexRole } = get();
      const sendOne = (to: string) =>
        invoke("daemon_send_message", {
          msg: {
            id: `user_${Date.now()}_${to}`,
            from: "user",
            to,
            content,
            timestamp: Date.now(),
          },
        }).catch(logError(set));

      if (target && target !== "auto") {
        sendOne(target);
      } else {
        // Auto: broadcast to all connected agents
        sendOne(claudeRole);
        sendOne(codexRole);
      }
    },

    clearMessages: () => set({ messages: [] }),

    launchCodexTui: async () => {
      const { codexRole } = get();
      try {
        await invoke("daemon_launch_codex", {
          roleId: codexRole,
          cwd: ".",
          model: null,
        });
      } catch (error) {
        logError(set)(error);
        throw error;
      }
    },

    stopCodexTui: () => invoke("daemon_stop_codex").catch(logError(set)),

    respondToPermission: async (requestId, behavior) => {
      try {
        await invoke("daemon_respond_permission", { requestId, behavior });
        set((s) => ({
          permissionPrompts: s.permissionPrompts.filter(
            (prompt) => prompt.requestId !== requestId,
          ),
        }));
      } catch (error) {
        set((s) => ({
          terminalLines: [
            ...s.terminalLines.slice(-200),
            {
              id: nextLogId(),
              agent: "system",
              kind: "error",
              line: `[Permission] ${String(error)}`,
              timestamp: Date.now(),
            },
          ],
        }));
        throw error;
      }
    },

    applyConfig: async (config) => {
      const { codexRole } = get();
      try {
        await invoke("daemon_launch_codex", {
          roleId: codexRole,
          cwd: config.cwd ?? ".",
          model: config.model ?? null,
        });
      } catch (error) {
        logError(set)(error);
        throw error;
      }
    },

    setRole: (agent, role) => {
      if (agent === "claude") {
        set({ claudeRole: role });
        invoke("daemon_set_claude_role", { role }).catch(logError(set));
      } else {
        set({ codexRole: role });
      }
    },

    cleanup: () => {
      clearUnlisteners();
    },
  };
});

if (import.meta.hot) {
  import.meta.hot.dispose(() => {
    useBridgeStore.getState().cleanup();
  });
}
