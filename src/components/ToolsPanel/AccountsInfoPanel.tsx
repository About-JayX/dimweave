import { useEffect, useState } from "react";
import { Settings2 } from "lucide-react";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import {
  useClaudeAccountStore,
  type ClaudeProfile,
} from "@/stores/claude-account-store";
import { useProviderAuthStore } from "@/stores/provider-auth-store";
import { ClaudeIcon, CodexIcon } from "@/components/AgentStatus/BrandIcons";
import { MiniMeter } from "@/components/CodexAccountPanel/MiniMeter";
import { windowLabel } from "@/components/CodexAccountPanel/helpers";
import { ProviderAuthDialog } from "./ProviderAuthDialog";
import { cn } from "@/lib/utils";

function AccountCard({
  icon,
  title,
  mode,
  children,
}: {
  icon: React.ReactNode;
  title: string;
  mode?: "subscription" | "api_key" | null;
  children: React.ReactNode;
}) {
  return (
    <div className="min-w-0 overflow-hidden rounded-lg border border-border/40 bg-card/40 px-3 py-2.5 text-[11px]">
      <div className="mb-1.5 flex items-center gap-1.5 text-foreground">
        {icon}
        <span className="font-semibold">{title}</span>
        {mode && (
          <span
            className={cn(
              "ml-auto rounded-full px-1.5 py-px text-[9px] font-semibold uppercase",
              mode === "api_key"
                ? "bg-amber-500/10 text-amber-500"
                : "bg-emerald-500/10 text-emerald-500",
            )}
            title={
              mode === "api_key"
                ? "Next launch uses the stored API key + endpoint."
                : "Next launch uses the provider's subscription credentials."
            }
          >
            {mode === "api_key" ? "API Key" : "Subscription"}
          </span>
        )}
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
    <div className="flex min-w-0 items-center justify-between gap-2 py-0.5">
      <span className="shrink-0 text-[10px] text-muted-foreground/70">
        {label}
      </span>
      <span
        className={cn(
          "min-w-0 flex-1 truncate text-right text-foreground/80",
          mono && "font-mono text-[10px]",
        )}
        title={typeof value === "string" ? value : undefined}
      >
        {value}
      </span>
    </div>
  );
}

function ClaudeUsageMeters({
  usage,
}: {
  usage: NonNullable<
    ReturnType<typeof useClaudeAccountStore.getState>["usage"]
  >;
}) {
  const windows: Array<
    [string, { utilization: number; resetsAt: number | null } | null]
  > = [
    ["5h", usage.fiveHour],
    ["7d", usage.sevenDay],
  ];
  return (
    <div className="mt-2 grid grid-cols-2 gap-2">
      {windows.map(([label, w]) => {
        const used = w ? Math.round(w.utilization * 100) : 0;
        const remaining = w ? Math.max(0, 100 - used) : 100;
        return (
          <MiniMeter
            key={label}
            label={label}
            used={used}
            remaining={remaining}
          />
        );
      })}
    </div>
  );
}

function ClaudeCard({ profile }: { profile: ClaudeProfile | null }) {
  const profileError = useClaudeAccountStore((s) => s.profileError);
  const usage = useClaudeAccountStore((s) => s.usage);
  const usageError = useClaudeAccountStore((s) => s.usageError);
  const usageRefreshing = useClaudeAccountStore((s) => s.usageRefreshing);
  const refreshUsage = useClaudeAccountStore((s) => s.refreshUsage);
  const auth = useProviderAuthStore((s) => s.configs.claude);
  const activeMode =
    auth?.activeMode ?? (auth?.apiKey ? "api_key" : "subscription");

  if (activeMode === "api_key") {
    return (
      <AccountCard
        icon={<ClaudeIcon className="size-3.5 shrink-0" />}
        title="Claude"
        mode="api_key"
      >
        <Row
          label="Endpoint"
          value={auth?.baseUrl || "api.anthropic.com"}
          mono
        />
        <Row
          label="Auth"
          value={auth?.authMode === "api_key" ? "x-api-key" : "Bearer"}
        />
        {auth?.apiKey && (
          <Row label="Key" value={`…${auth.apiKey.slice(-4)}`} mono />
        )}
      </AccountCard>
    );
  }
  if (!profile) {
    return (
      <AccountCard
        icon={<ClaudeIcon className="size-3.5 shrink-0" />}
        title="Claude"
        mode="subscription"
      >
        <p className="max-w-full overflow-hidden whitespace-pre-wrap break-all [overflow-wrap:anywhere] text-[10px] leading-relaxed text-muted-foreground/60">
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
      mode="subscription"
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
        <button
          type="button"
          disabled={usageRefreshing}
          onClick={() => void refreshUsage()}
          className={cn(
            "ml-auto text-[10px] text-muted-foreground hover:text-foreground transition-colors",
            usageRefreshing && "opacity-50",
          )}
          title="Sends a tiny ping (max_tokens=1) to read rate-limit headers"
        >
          {usageRefreshing ? "…" : usage ? "Refresh" : "Check usage"}
        </button>
      </div>
      <Row label="Email" value={profile.email || "—"} />
      <Row label="Name" value={profile.displayName || "—"} />
      <Row label="Rate limit" value={profile.rateLimitTier || "—"} mono />
      {usage && <ClaudeUsageMeters usage={usage} />}
      {usageError && (
        <p className="mt-1 text-[10px] text-destructive/80">{usageError}</p>
      )}
    </AccountCard>
  );
}

function CodexCard() {
  const profile = useCodexAccountStore((s) => s.profile);
  const usage = useCodexAccountStore((s) => s.usage);
  const refreshing = useCodexAccountStore((s) => s.refreshing);
  const refreshUsage = useCodexAccountStore((s) => s.refreshUsage);
  const auth = useProviderAuthStore((s) => s.configs.codex);
  const activeMode =
    auth?.activeMode ?? (auth?.apiKey ? "api_key" : "subscription");

  if (activeMode === "api_key") {
    return (
      <AccountCard
        icon={<CodexIcon className="size-3.5 shrink-0" />}
        title="Codex"
        mode="api_key"
      >
        <Row
          label="Endpoint"
          value={auth?.baseUrl || "api.openai.com/v1"}
          mono
        />
        <Row label="Wire API" value={auth?.wireApi ?? "chat"} />
        {auth?.providerName && (
          <Row label="Provider" value={auth.providerName} mono />
        )}
        {auth?.apiKey && (
          <Row label="Key" value={`…${auth.apiKey.slice(-4)}`} mono />
        )}
      </AccountCard>
    );
  }
  if (!profile?.email) {
    return (
      <AccountCard
        icon={<CodexIcon className="size-3.5 shrink-0" />}
        title="Codex"
        mode="subscription"
      >
        <p className="max-w-full overflow-hidden whitespace-pre-wrap break-all [overflow-wrap:anywhere] text-[10px] leading-relaxed text-muted-foreground/60">
          Not signed in — open settings to configure.
        </p>
      </AccountCard>
    );
  }
  const limited = usage && (usage.limitReached || !usage.allowed);
  return (
    <AccountCard
      icon={<CodexIcon className="size-3.5 shrink-0" />}
      title="Codex"
      mode="subscription"
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
  const fetchProviderAuth = useProviderAuthStore((s) => s.fetchAll);
  const [authOpen, setAuthOpen] = useState(false);

  useEffect(() => {
    if (!claudeProfile) fetchClaudeProfile();
    if (!codexProfile) fetchCodexProfile();
    fetchCodexUsage();
    fetchProviderAuth();
  }, [
    claudeProfile,
    codexProfile,
    fetchClaudeProfile,
    fetchCodexProfile,
    fetchCodexUsage,
    fetchProviderAuth,
  ]);

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-end">
        <button
          type="button"
          onClick={() => setAuthOpen(true)}
          className="rounded-lg p-1.5 text-muted-foreground/50 transition-colors hover:bg-muted hover:text-foreground active:opacity-70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          title="Provider Authentication"
          aria-label="Provider Authentication"
        >
          <Settings2 className="size-3.5" />
        </button>
      </div>
      <ClaudeCard profile={claudeProfile} />
      <CodexCard />
      <ProviderAuthDialog open={authOpen} onOpenChange={setAuthOpen} />
    </div>
  );
}
