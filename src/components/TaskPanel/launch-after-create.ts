import { invoke } from "@tauri-apps/api/core";
import { buildClaudeLaunchRequest } from "@/components/ClaudePanel/launch-request";
import { buildCodexLaunchConfig } from "@/components/AgentStatus/codex-launch-config";
import type { AgentDraftConfig } from "@/components/AgentStatus/provider-session-view-model";

interface LaunchOpts {
  taskId: string;
  workspace: string;
  claudeRole: string;
  claudeConfig: AgentDraftConfig | null;
  codexConfig: AgentDraftConfig | null;
  resumeSession: (sessionId: string) => Promise<void>;
  applyConfig: (config: Record<string, string | undefined>) => Promise<void>;
}

export async function launchProvidersAfterCreate(opts: LaunchOpts) {
  const {
    taskId,
    workspace,
    claudeRole,
    claudeConfig,
    codexConfig,
    resumeSession,
    applyConfig,
  } = opts;

  if (claudeConfig) {
    const { historyAction } = claudeConfig;
    if (historyAction.kind === "resumeNormalized") {
      await resumeSession(historyAction.sessionId);
    } else {
      await invoke(
        "daemon_launch_claude_sdk",
        buildClaudeLaunchRequest({
          claudeRole,
          cwd: workspace,
          model: claudeConfig.model,
          effort: claudeConfig.effort,
          resumeSessionId:
            historyAction.kind === "resumeExternal"
              ? historyAction.externalId
              : undefined,
          taskId,
        }),
      );
    }
  }

  if (codexConfig) {
    const { historyAction } = codexConfig;
    if (historyAction.kind === "resumeNormalized") {
      await resumeSession(historyAction.sessionId);
    } else {
      await applyConfig(
        buildCodexLaunchConfig({
          model: codexConfig.model,
          reasoningEffort: codexConfig.effort,
          cwd: workspace,
          resumeThreadId:
            historyAction.kind === "resumeExternal"
              ? historyAction.externalId
              : undefined,
          taskId,
        }),
      );
    }
  }
}
