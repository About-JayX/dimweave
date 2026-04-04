import { invoke } from "@tauri-apps/api/core";
import type { RuntimeHealthInfo } from "@/types";
import type { BridgeState } from "./types";
import { logError } from "./helpers";

interface AgentRuntimeStatusPayload {
  agent: string;
  online: boolean;
  providerSession?: {
    provider: "claude" | "codex";
    externalSessionId: string;
    cwd: string;
    connectionMode: "new" | "resumed";
  };
}
interface DaemonStatusSnapshotPayload {
  agents: AgentRuntimeStatusPayload[];
  runtimeHealth?: RuntimeHealthInfo | null;
  claudeRole: string;
  codexRole: string;
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
          providerSession:
            snapshot.agents.find((item) => item.agent === agent)?.providerSession,
        };
      }

      for (const { agent, online, providerSession } of snapshot.agents) {
        nextAgents[agent] = {
          ...(nextAgents[agent] ?? {
            name: agent,
            displayName: agent,
          }),
          name: agent,
          displayName: nextAgents[agent]?.displayName ?? agent,
          status: online ? "connected" : "disconnected",
          providerSession,
        };
      }

      return {
        agents: nextAgents,
        runtimeHealth: snapshot.runtimeHealth ?? null,
        claudeRole: snapshot.claudeRole,
        codexRole: snapshot.codexRole,
      };
    });
  } catch (error) {
    logError(set)(error);
  }
}
