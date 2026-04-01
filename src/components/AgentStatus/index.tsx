import { useEffect } from "react";
import { ClaudePanel } from "@/components/ClaudePanel";
import { useBridgeStore } from "@/stores/bridge-store";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { useTaskStore } from "@/stores/task-store";
import { ReviewGateBadge } from "@/components/TaskPanel/ReviewGateBadge";
import { getReviewBadge } from "@/components/TaskPanel/view-model";
import type { AgentInfo } from "@/types";
import { StatusDot } from "./StatusDot";
import { CodexPanel } from "./CodexPanel";

interface AgentStatusProps {
  agents: Record<string, AgentInfo>;
  connected: boolean;
}

export function AgentStatusPanel({ agents, connected }: AgentStatusProps) {
  const stopCodexTui = useBridgeStore((s) => s.stopCodexTui);
  const claudeTerminalRunning = useBridgeStore((s) => s.claudeTerminalRunning);
  const profile = useCodexAccountStore((s) => s.profile);
  const usage = useCodexAccountStore((s) => s.usage);
  const refreshing = useCodexAccountStore((s) => s.refreshing);
  const fetchProfile = useCodexAccountStore((s) => s.fetchProfile);
  const fetchUsage = useCodexAccountStore((s) => s.fetchUsage);
  const refreshUsage = useCodexAccountStore((s) => s.refreshUsage);
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const sessions = useTaskStore((s) => s.sessions);

  // Derive agent states from the agents map (populated via agent_status Tauri events)
  const claudeConnected = agents.claude?.status === "connected";
  const codexConnected = agents.codex?.status === "connected";
  const activeTask = activeTaskId ? tasks[activeTaskId] : null;
  const activeTaskSessions = activeTaskId ? sessions[activeTaskId] ?? [] : [];
  const activeCodexSession =
    activeTaskSessions.find(
      (session) =>
        session.provider === "codex" &&
        session.sessionId === activeTask?.currentCoderSessionId,
    ) ??
    activeTaskSessions.find(
      (session) =>
        session.provider === "codex" &&
        session.sessionId === activeTask?.leadSessionId,
    ) ??
    activeTaskSessions.find((session) => session.provider === "codex") ??
    null;
  const reviewBadge = getReviewBadge(activeTask?.reviewStatus);

  useEffect(() => {
    fetchProfile();
  }, [fetchProfile]);
  useEffect(() => {
    if (codexConnected) fetchUsage();
  }, [codexConnected, fetchUsage]);

  return (
    <div className="flex flex-1 flex-col gap-3 p-4 overflow-y-auto min-h-0">
      {/* Daemon connection status */}
      <div className="flex items-center gap-2 pb-3 border-b border-border/50 relative">
        <h3 className="flex-1 m-0 text-sm font-semibold text-foreground">
          AgentNexus
        </h3>
        <StatusDot status={connected ? "connected" : "error"} variant="codex" />
        <span className="text-xs text-secondary-foreground">
          {connected ? "Online" : "Offline"}
        </span>
        <div className="absolute bottom-0 left-0 right-0 h-px bg-linear-to-r from-transparent via-primary/20 to-transparent" />
      </div>

      <div className="flex flex-col gap-2">
        {activeTask && (
          <div className="rounded-lg border border-border/45 bg-background/35 px-3 py-2">
            <div className="text-[10px] uppercase tracking-[0.16em] text-muted-foreground/55">
              Active Task
            </div>
            <div className="truncate text-[13px] font-medium text-foreground">
              {activeTask.title}
            </div>
            <div className="mt-1 flex items-center gap-2">
              <span className="truncate text-[11px] text-muted-foreground/60">
                {activeTask.workspaceRoot}
              </span>
              {reviewBadge && <ReviewGateBadge badge={reviewBadge} />}
            </div>
          </div>
        )}
        {/* Claude Code */}
        <ClaudePanel
          connected={claudeConnected}
          terminalRunning={claudeTerminalRunning}
        />

        {/* Codex */}
        <CodexPanel
          codexTuiRunning={codexConnected}
          threadId={activeCodexSession?.externalSessionId ?? null}
          stopCodexTui={stopCodexTui}
          profile={profile}
          usage={usage}
          refreshing={refreshing}
          refreshUsage={refreshUsage}
        />

        {/* Other agents */}
        {Object.entries(agents)
          .filter(([key]) => key !== "claude" && key !== "codex")
          .map(([key, agent]) => (
            <div
              key={key}
              className="rounded-lg border border-input bg-card p-3 card-depth transition-all duration-300 hover:border-input/80"
            >
              <div className="flex items-center gap-2">
                <StatusDot status={agent.status} />
                <span className="flex-1 text-[13px] font-medium text-card-foreground">
                  {agent.displayName}
                </span>
                <span className="text-[11px] uppercase text-secondary-foreground">
                  {agent.status}
                </span>
              </div>
            </div>
          ))}
      </div>
    </div>
  );
}
