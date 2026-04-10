import { useEffect } from "react";
import { ClaudePanel } from "@/components/ClaudePanel";
import { useBridgeStore } from "@/stores/bridge-store";
import { selectAgents, selectConnected } from "@/stores/bridge-store/selectors";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { StatusDot } from "./StatusDot";
import { CodexPanel } from "./CodexPanel";

export function AgentStatusPanel() {
  const agents = useBridgeStore(selectAgents);
  const connected = useBridgeStore(selectConnected);
  const stopCodexTui = useBridgeStore((s) => s.stopCodexTui);
  const profile = useCodexAccountStore((s) => s.profile);
  const usage = useCodexAccountStore((s) => s.usage);
  const refreshing = useCodexAccountStore((s) => s.refreshing);
  const fetchProfile = useCodexAccountStore((s) => s.fetchProfile);
  const fetchUsage = useCodexAccountStore((s) => s.fetchUsage);
  const refreshUsage = useCodexAccountStore((s) => s.refreshUsage);

  // Derive agent states from the agents map (populated via agent_status Tauri events)
  const claudeConnected = agents.claude?.status === "connected";
  const codexConnected = agents.codex?.status === "connected";

  useEffect(() => {
    fetchProfile();
  }, [fetchProfile]);
  useEffect(() => {
    if (codexConnected) fetchUsage();
  }, [codexConnected, fetchUsage]);

  return (
    <section className="space-y-3">
      <div className="rounded-2xl border border-border/40 bg-card/55 px-4 py-3">
        <div className="flex items-center gap-2">
          <div className="flex-1">
            <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
              Providers
            </div>
            <div className="mt-0.5 text-sm font-semibold text-foreground">
              Runtime control
            </div>
          </div>
          <StatusDot
            status={connected ? "connected" : "error"}
            variant="generic"
          />
          <span className="text-[11px] text-muted-foreground">
            {connected ? "Daemon online" : "Daemon offline"}
          </span>
        </div>
        <div className="mt-2 text-[12px] text-muted-foreground/65">
          Keep Claude and Codex available here, but push message work back to
          the central timeline.
        </div>
      </div>

      <div className="space-y-3">
        <ClaudePanel
          connected={claudeConnected}
          providerSession={agents.claude?.providerSession}
        />
        <CodexPanel
          codexTuiRunning={codexConnected}
          stopCodexTui={stopCodexTui}
          profile={profile}
          usage={usage}
          refreshing={refreshing}
          refreshUsage={refreshUsage}
          providerSession={agents.codex?.providerSession}
        />
        {Object.entries(agents).some(
          ([key]) => key !== "claude" && key !== "codex",
        ) && (
          <div className="rounded-2xl border border-border/40 bg-card/45 px-4 py-3">
            <div className="mb-2 text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
              Additional agents
            </div>
            <div className="space-y-2">
              {Object.entries(agents)
                .filter(([key]) => key !== "claude" && key !== "codex")
                .map(([key, agent]) => (
                  <div
                    key={key}
                    className="rounded-xl border border-border/35 bg-background/35 px-3 py-2"
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
        )}
      </div>
    </section>
  );
}
