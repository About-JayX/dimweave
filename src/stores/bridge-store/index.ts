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

export type { TerminalLine, UiError, BridgeState } from "./types";

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
    uiErrors: [],
    permissionPrompts: [],
    permissionError: null,
    runtimeHealth: null,
    claudeNeedsAttention: false,
    claudeRole: "",
    codexRole: "",
    claudeStream: {
      thinking: false,
      previewText: "",
      thinkingText: "",
      blockType: "idle",
      toolName: "",
      lastUpdatedAt: 0,
    },
    codexStream: {
      thinking: false,
      currentDelta: "",
      lastMessage: "",
      turnStatus: "",
      activity: "",
      reasoning: "",
      commandOutput: "",
    },
    claudeStreamsByTask: {},
    codexStreamsByTask: {},
    draft: "",

    setDraft: (text) => set({ draft: text }),
    clearClaudeAttention: () => set({ claudeNeedsAttention: false }),

    sendToCodex: (message, target, attachments, taskId) => {
      invoke("daemon_send_user_input", {
        message,
        target: target ?? "auto",
        attachments: attachments?.length ? attachments : null,
        taskId: taskId ?? null,
      }).catch(logError(set));
    },

    clearMessages: () => set({ messages: [] }),

    stopCodexTui: () => invoke("daemon_stop_codex").catch(logError(set)),

    respondToPermission: async (requestId, behavior) => {
      try {
        set({ permissionError: null });
        await invoke("daemon_respond_permission", { requestId, behavior });
        set((s) => ({
          permissionPrompts: s.permissionPrompts.filter(
            (prompt) => prompt.requestId !== requestId,
          ),
          permissionError:
            s.permissionError?.requestId === requestId
              ? null
              : s.permissionError,
        }));
      } catch (error) {
        const message = String(error);
        set((s) => ({
          terminalLines: [
            ...s.terminalLines.slice(-200),
            {
              id: nextLogId(),
              agent: "system",
              kind: "error",
              line: `[Permission] ${message}`,
              timestamp: Date.now(),
            },
          ],
          permissionError: {
            requestId,
            message,
          },
        }));
        throw error;
      }
    },

    applyConfig: async (config) => {
      const { codexRole } = get();
      if (!config.cwd?.trim()) {
        const error = new Error("Select a project before connecting Codex");
        logError(set)(error);
        throw error;
      }
      if (!codexRole.trim()) {
        const error = new Error("Select Codex role before connecting");
        logError(set)(error);
        throw error;
      }
      try {
        await invoke("daemon_launch_codex", {
          roleId: codexRole,
          cwd: config.cwd,
          model: config.model ?? null,
          reasoningEffort: config.reasoningEffort ?? null,
          resumeThreadId: config.resumeThreadId ?? null,
          taskId: config.taskId ?? null,
        });
      } catch (error) {
        logError(set)(error);
        throw error;
      }
    },

    pushUiError: (message, componentStack) => {
      set((s) => ({
        uiErrors: [
          ...s.uiErrors,
          { id: nextLogId(), message, componentStack, timestamp: Date.now() },
        ],
      }));
    },
    clearUiErrors: () => set({ uiErrors: [] }),

    setRole: (agent, role) => {
      const cmd =
        agent === "claude" ? "daemon_set_claude_role" : "daemon_set_codex_role";
      const key = agent === "claude" ? "claudeRole" : "codexRole";
      invoke(cmd, { role })
        .then(() => set({ [key]: role }))
        .catch(logError(set));
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
