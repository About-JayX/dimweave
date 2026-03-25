import { create } from "zustand";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type {
  BridgeMessage,
  PermissionBehavior,
  PermissionPrompt,
} from "@/types";
import type { BridgeState } from "./types";

export type { TerminalLine, BridgeState } from "./types";

// Tauri event payload shapes emitted by the Rust daemon (camelCase from serde)
interface AgentMessagePayload {
  payload: BridgeMessage;
  timestamp: number;
}
interface SystemLogPayload {
  level: string;
  message: string;
}
interface AgentStatusPayload {
  agent: string;
  online: boolean;
  exitCode?: number;
}
interface PermissionPromptPayload extends PermissionPrompt {}

let _unlisteners: UnlistenFn[] = []; // cleanup() to prevent leaks during HMR
let _logId = 0; // monotonic ID for TerminalLine keys

function initListeners(
  set: (fn: (s: BridgeState) => Partial<BridgeState>) => void,
) {
  Promise.all([
    listen<AgentMessagePayload>("agent_message", (e) => {
      set((s) => ({
        messages: [...s.messages.slice(-999), e.payload.payload],
      }));
    }),
    listen<SystemLogPayload>("system_log", (e) => {
      const { level, message } = e.payload;
      set((s) => ({
        terminalLines: [
          ...s.terminalLines.slice(-200),
          {
            id: ++_logId,
            agent: "system",
            kind: level === "error" ? ("error" as const) : ("text" as const),
            line: message,
            timestamp: Date.now(),
          },
        ],
      }));
    }),
    listen<AgentStatusPayload>("agent_status", (e) => {
      const { agent, online } = e.payload;
      set((s) => ({
        agents: {
          ...s.agents,
          [agent]: {
            ...s.agents[agent],
            name: agent,
            displayName: s.agents[agent]?.displayName ?? agent,
            status: online ? ("connected" as const) : ("disconnected" as const),
          },
        },
      }));
    }),
    listen<PermissionPromptPayload>("permission_prompt", (e) => {
      set((s) => ({
        permissionPrompts: [
          ...s.permissionPrompts.filter(
            (prompt) => prompt.requestId !== e.payload.requestId,
          ),
          e.payload,
        ],
      }));
    }),
  ]).then((fns) => {
    _unlisteners.forEach((fn) => fn());
    _unlisteners = fns;
  });
}

function logError(set: (fn: (s: BridgeState) => Partial<BridgeState>) => void) {
  return (e: unknown) =>
    set((s) => ({
      terminalLines: [
        ...s.terminalLines.slice(-200),
        {
          id: ++_logId,
          agent: "system",
          kind: "error" as const,
          line: `[Error] ${String(e)}`,
          timestamp: Date.now(),
        },
      ],
    }));
}

export const useBridgeStore = create<BridgeState>((set, get) => {
  initListeners(set);

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
    permissionPrompts: [],
    claudeRole: "lead",
    codexRole: "coder",
    draft: "",

    setDraft: (text) => set({ draft: text }),

    sendToCodex: (content) => {
      const { codexRole } = get();
      invoke("daemon_send_message", {
        msg: {
          id: `user_${Date.now()}`,
          from: "user",
          to: codexRole,
          content,
          timestamp: Date.now(),
        },
      }).catch(logError(set));
    },

    clearMessages: () => set({ messages: [] }),

    launchCodexTui: () => {
      const { codexRole } = get();
      invoke("daemon_launch_codex", {
        roleId: codexRole,
        cwd: ".",
        model: null,
      }).catch(logError(set));
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
              id: ++_logId,
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

    applyConfig: (config) => {
      const { codexRole } = get();
      invoke("daemon_launch_codex", {
        roleId: codexRole,
        cwd: config.cwd ?? ".",
        model: config.model ?? null,
      }).catch(logError(set));
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
      _unlisteners.forEach((fn) => fn());
      _unlisteners = [];
    },
  };
});
