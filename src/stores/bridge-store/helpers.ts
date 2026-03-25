import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type { BridgeMessage, PermissionPrompt } from "@/types";
import type { BridgeState } from "./types";

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
interface AgentRuntimeStatusPayload {
  agent: string;
  online: boolean;
}
interface DaemonStatusSnapshotPayload {
  agents: AgentRuntimeStatusPayload[];
  claudeRole: string;
  codexRole: string;
}

export let _unlisteners: UnlistenFn[] = [];
export let _logId = 0;
export function nextLogId(): number {
  return ++_logId;
}
export function clearUnlisteners() {
  _unlisteners.forEach((fn) => fn());
  _unlisteners = [];
}
export function setUnlisteners(fns: UnlistenFn[]) {
  _unlisteners.forEach((fn) => fn());
  _unlisteners = fns;
}

export function initListeners(
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
            id: nextLogId(),
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
    setUnlisteners(fns);
  });
}

export function logError(
  set: (fn: (s: BridgeState) => Partial<BridgeState>) => void,
) {
  return (e: unknown) =>
    set((s) => ({
      terminalLines: [
        ...s.terminalLines.slice(-200),
        {
          id: nextLogId(),
          agent: "system",
          kind: "error" as const,
          line: `[Error] ${String(e)}`,
          timestamp: Date.now(),
        },
      ],
    }));
}

export async function syncStatusSnapshot(
  set: (fn: (s: BridgeState) => Partial<BridgeState>) => void,
) {
  try {
    const snapshot = await invoke<DaemonStatusSnapshotPayload>(
      "daemon_get_status_snapshot",
    );
    set((s) => {
      const onlineAgents = new Set(
        snapshot.agents
          .filter((agent) => agent.online)
          .map((agent) => agent.agent),
      );
      const nextAgents = { ...s.agents };

      for (const [agent, info] of Object.entries(nextAgents)) {
        nextAgents[agent] = {
          ...info,
          name: agent,
          displayName: info.displayName ?? agent,
          status: onlineAgents.has(agent) ? "connected" : "disconnected",
        };
      }

      for (const { agent, online } of snapshot.agents) {
        nextAgents[agent] = {
          ...(nextAgents[agent] ?? {
            name: agent,
            displayName: agent,
          }),
          name: agent,
          displayName: nextAgents[agent]?.displayName ?? agent,
          status: online ? "connected" : "disconnected",
        };
      }

      return {
        agents: nextAgents,
        claudeRole: snapshot.claudeRole,
        codexRole: snapshot.codexRole,
      };
    });
  } catch (error) {
    logError(set)(error);
  }
}
