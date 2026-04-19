import { useEffect } from "react";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import {
  useClaudeAccountStore,
  type ClaudeProfile,
} from "@/stores/claude-account-store";
import { ClaudeIcon, CodexIcon } from "@/components/AgentStatus/BrandIcons";
import { MiniMeter } from "@/components/CodexAccountPanel/MiniMeter";
import { windowLabel } from "@/components/CodexAccountPanel/helpers";
import { cn } from "@/lib/utils";

function AccountCard({
  icon,
  title,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border border-border/40 bg-card/40 px-3 py-2.5 text-[11px]">
      <div className="mb-1.5 flex items-center gap-1.5 text-foreground">
        {icon}
        <span className="font-semibold">{title}</span>
      </div>
      {children}
    </div>
  );
}

function Row({
  label,
  value,
  mono,
}: {
  label: string;
  value: React.ReactNode;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center justify-between gap-2 py-0.5">
      <span className="text-[10px] text-muted-foreground/70">{label}</span>
      <span
        className={cn(
          "truncate text-foreground/80",
          mono && "font-mono text-[10px]",
        )}
      >
        {value}
      </span>
    </div>
  );
}

function ClaudeCard({ profile }: { profile: ClaudeProfile | null }) {
  const profileError = useClaudeAccountStore((s) => s.profileError);
  if (!profile) {
    return (
      <AccountCard
        icon={<ClaudeIcon className="size-3.5 shrink-0" />}
        title="Claude"
      >
        <p className="text-[10px] text-muted-foreground/60">
          {profileError ? `Error: ${profileError}` : "Loading…"}
        </p>
      </AccountCard>
    );
  }
  const tierBadge = profile.subscriptionTier.toUpperCase();
  return (
    <AccountCard
      icon={<ClaudeIcon className="size-3.5 shrink-0" />}
      title="Claude"
    >
      <div className="flex items-center gap-1.5 mb-1">
        <span
          className={cn(
            "rounded-full px-1.5 py-px text-[9px] font-semibold",
            profile.subscriptionStatus === "active"
              ? "bg-emerald-500/10 text-emerald-500"
              : "bg-destructive/10 text-destructive",
          )}
        >
          {tierBadge}
        </span>
        <span className="text-[10px] text-muted-foreground/70">
          {profile.subscriptionStatus}
        </span>
      </div>
      <Row label="Email" value={profile.email || "—"} />
      <Row label="Name" value={profile.displayName || "—"} />
      <Row label="Rate limit" value={profile.rateLimitTier || "—"} mono />
    </AccountCard>
  );
}

function CodexCard() {
  const profile = useCodexAccountStore((s) => s.profile);
  const usage = useCodexAccountStore((s) => s.usage);
  const refreshing = useCodexAccountStore((s) => s.refreshing);
  const refreshUsage = useCodexAccountStore((s) => s.refreshUsage);
  if (!profile) {
    return (
      <AccountCard
        icon={<CodexIcon className="size-3.5 shrink-0" />}
        title="Codex"
      >
        <p className="text-[10px] text-muted-foreground/60">Not signed in</p>
      </AccountCard>
    );
  }
  const limited = usage && (usage.limitReached || !usage.allowed);
  return (
    <AccountCard
      icon={<CodexIcon className="size-3.5 shrink-0" />}
      title="Codex"
    >
      <div className="mb-1 flex items-center gap-1.5">
        {profile.planType && (
          <span
            className={cn(
              "rounded-full px-1.5 py-px text-[9px] font-semibold",
              limited
                ? "bg-destructive/10 text-destructive"
                : "bg-codex/10 text-codex",
            )}
          >
            {profile.planType.toUpperCase()}
          </span>
        )}
        {usage && (
          <span className="text-[10px] text-muted-foreground/70">
            {limited ? "limited" : "healthy"}
          </span>
        )}
        <button
          type="button"
          disabled={refreshing}
          onClick={() => void refreshUsage()}
          className={cn(
            "ml-auto text-[10px] text-muted-foreground hover:text-foreground transition-colors",
            refreshing && "opacity-50",
          )}
        >
          {refreshing ? "…" : "Refresh"}
        </button>
      </div>
      <Row label="Email" value={profile.email || "—"} />
      <Row label="Name" value={profile.name || "—"} />
      {usage && (
        <div className="mt-2 grid grid-cols-2 gap-2">
          <MiniMeter
            label={windowLabel(usage.primary?.windowMinutes ?? null, "Short")}
            used={usage.primary?.usedPercent ?? 0}
            remaining={usage.primary?.remainingPercent ?? 100}
          />
          <MiniMeter
            label={windowLabel(usage.secondary?.windowMinutes ?? null, "Long")}
            used={usage.secondary?.usedPercent ?? 0}
            remaining={usage.secondary?.remainingPercent ?? 100}
          />
        </div>
      )}
    </AccountCard>
  );
}

export function AccountsInfoPanel() {
  const claudeProfile = useClaudeAccountStore((s) => s.profile);
  const fetchClaudeProfile = useClaudeAccountStore((s) => s.fetchProfile);
  const codexProfile = useCodexAccountStore((s) => s.profile);
  const fetchCodexProfile = useCodexAccountStore((s) => s.fetchProfile);
  const fetchCodexUsage = useCodexAccountStore((s) => s.fetchUsage);

  useEffect(() => {
    if (!claudeProfile) fetchClaudeProfile();
    if (!codexProfile) fetchCodexProfile();
    fetchCodexUsage();
  }, [
    claudeProfile,
    codexProfile,
    fetchClaudeProfile,
    fetchCodexProfile,
    fetchCodexUsage,
  ]);

  return (
    <div className="space-y-2">
      <ClaudeCard profile={claudeProfile} />
      <CodexCard />
    </div>
  );
}
