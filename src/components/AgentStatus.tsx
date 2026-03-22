import { useEffect } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ClaudePanel } from "@/components/ClaudePanel";
import { CodexAccountPanel } from "@/components/CodexAccountPanel";
import { useBridgeStore } from "@/stores/bridge-store";

const ROLE_OPTIONS = [
  { value: "lead", label: "Lead" },
  { value: "coder", label: "Coder" },
  { value: "reviewer", label: "Reviewer" },
  { value: "tester", label: "Tester" },
];

function RoleSelect({ agent }: { agent: "claude" | "codex" }) {
  const role = useBridgeStore((s) =>
    agent === "claude" ? s.claudeRole : s.codexRole,
  );
  const setRole = useBridgeStore((s) => s.setAgentRole);
  return (
    <select
      value={role}
      onChange={(e) => setRole(agent, e.target.value)}
      className="rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-foreground border border-input outline-none cursor-pointer"
    >
      {ROLE_OPTIONS.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
import { useCodexAccountStore } from "@/stores/codex-account-store";
import type { AgentInfo, DaemonStatus } from "@/types";

interface AgentStatusProps {
  agents: Record<string, AgentInfo>;
  daemonStatus: DaemonStatus | null;
  connected: boolean;
}

const statusDotColor: Record<string, string> = {
  connected: "bg-codex",
  connecting: "bg-yellow-500",
  disconnected: "bg-muted-foreground",
  error: "bg-destructive",
};

function StatusDot({ status }: { status: string }) {
  return (
    <span
      className={cn(
        "inline-block size-2 shrink-0 rounded-full",
        statusDotColor[status] ?? "bg-muted-foreground",
      )}
    />
  );
}

export function AgentStatusPanel({
  agents,
  daemonStatus,
  connected,
}: AgentStatusProps) {
  const launchCodexTui = useBridgeStore((s) => s.launchCodexTui);
  const stopCodexTui = useBridgeStore((s) => s.stopCodexTui);
  const profile = useCodexAccountStore((s) => s.profile);
  const usage = useCodexAccountStore((s) => s.usage);
  const refreshing = useCodexAccountStore((s) => s.refreshing);
  const fetchProfile = useCodexAccountStore((s) => s.fetchProfile);
  const fetchUsage = useCodexAccountStore((s) => s.fetchUsage);
  const refreshUsage = useCodexAccountStore((s) => s.refreshUsage);

  const codexTuiRunning = daemonStatus?.codexTuiRunning ?? false;
  const codexReady = daemonStatus?.codexBootstrapped ?? false;
  const claudeConnected = daemonStatus?.claudeConnected ?? false;

  // Fetch profile on mount, usage when codex connects
  useEffect(() => {
    fetchProfile();
  }, [fetchProfile]);
  useEffect(() => {
    if (codexTuiRunning) fetchUsage();
  }, [codexTuiRunning, fetchUsage]);

  return (
    <div className="flex flex-1 flex-col gap-3 p-4 overflow-y-auto min-h-0">
      {/* Daemon connection */}
      <div className="flex items-center gap-2 pb-3 border-b border-border">
        <h3 className="flex-1 m-0 text-sm font-semibold text-foreground">
          AgentBridge
        </h3>
        <span
          className={cn(
            "inline-block size-2 shrink-0 rounded-full",
            connected ? "bg-codex" : "bg-destructive",
          )}
        />
        <span className="text-xs text-secondary-foreground">
          {connected ? "Online" : "Offline"}
        </span>
      </div>

      <div className="flex flex-col gap-2">
        {/* Claude Code */}
        <ClaudePanel connected={claudeConnected} />

        {/* Codex */}
        <div className="rounded-lg border border-input bg-card p-3">
          <div className="flex items-center gap-2">
            <span
              className={cn(
                "inline-block size-2 shrink-0 rounded-full",
                codexTuiRunning
                  ? "bg-codex"
                  : codexReady
                    ? "bg-yellow-500"
                    : "bg-muted-foreground",
              )}
            />
            <span className="flex-1 text-[13px] font-medium text-card-foreground">
              Codex
            </span>
            <RoleSelect agent="codex" />
            <span className="text-[11px] uppercase text-secondary-foreground">
              {codexTuiRunning
                ? "connected"
                : codexReady
                  ? "ready"
                  : "starting..."}
            </span>
          </div>

          {daemonStatus?.threadId && (
            <div className="mt-1 font-mono text-[11px] text-muted-foreground">
              Thread: {daemonStatus.threadId.slice(0, 16)}...
            </div>
          )}

          <div className="mt-2">
            {!codexTuiRunning ? (
              <Button
                className="w-full bg-codex text-white hover:bg-codex/80"
                size="sm"
                disabled={!codexReady}
                onClick={launchCodexTui}
              >
                Connect Codex
              </Button>
            ) : (
              <Button
                className="w-full"
                variant="secondary"
                size="sm"
                onClick={stopCodexTui}
              >
                Disconnect Codex
              </Button>
            )}
          </div>

          <CodexAccountPanel
            profile={profile}
            usage={usage}
            refreshing={refreshing}
            onRefresh={refreshUsage}
            protocolData={daemonStatus?.codexAccount}
          />

          {!codexReady && (
            <div className="mt-1.5 text-[11px] text-muted-foreground">
              Codex app-server is starting...
            </div>
          )}
        </div>

        {/* Other agents */}
        {Object.entries(agents)
          .filter(([key]) => key !== "claude" && key !== "codex")
          .map(([key, agent]) => (
            <div
              key={key}
              className="rounded-lg border border-input bg-card p-3"
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

      {/* Daemon info */}
      {daemonStatus && (
        <div className="mt-auto rounded-md bg-muted p-2.5">
          <div className="mb-1 text-[11px] font-semibold uppercase text-muted-foreground">
            Daemon
          </div>
          <div className="font-mono text-[11px] text-muted-foreground">
            PID: {daemonStatus.pid}
          </div>
          <div className="font-mono text-[11px] text-muted-foreground">
            Queued: {daemonStatus.queuedMessageCount}
          </div>
          <div className="font-mono text-[11px] text-muted-foreground">
            Proxy: {daemonStatus.proxyUrl}
          </div>
        </div>
      )}
    </div>
  );
}
