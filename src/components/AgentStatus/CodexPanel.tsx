import { useState, useEffect, useCallback, useMemo } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { useBridgeStore } from "@/stores/bridge-store";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { MiniMeter } from "@/components/CodexAccountPanel/MiniMeter";
import { windowLabel } from "@/components/CodexAccountPanel/helpers";
import { RoleSelect } from "./RoleSelect";
import type { CodexAccountInfo } from "@/types";

function shortenPath(p: string): string {
  const idx = p.indexOf("/Users/");
  if (idx >= 0) {
    const rest = p.slice(idx + 7);
    const slash = rest.indexOf("/");
    return slash >= 0 ? `~${rest.slice(slash)}` : "~";
  }
  return p;
}

interface CodexPanelProps {
  codexTuiRunning: boolean;
  codexReady: boolean;
  threadId: string | null;
  launchCodexTui: () => void;
  stopCodexTui: () => void;
  profile: { name?: string; planType?: string } | null;
  usage: {
    allowed: boolean;
    limitReached: boolean;
    primary: {
      usedPercent: number;
      remainingPercent: number;
      windowMinutes: number | null;
    } | null;
    secondary: {
      usedPercent: number;
      remainingPercent: number;
      windowMinutes: number | null;
    } | null;
  } | null;
  refreshing: boolean;
  refreshUsage: () => void;
  codexAccount: CodexAccountInfo | undefined;
}

export function CodexPanel({
  codexTuiRunning,
  codexReady,
  threadId,
  launchCodexTui,
  stopCodexTui,
  profile,
  usage,
  refreshing,
  refreshUsage,
  codexAccount,
}: CodexPanelProps) {
  const models = useCodexAccountStore((s) => s.models);
  const fetchModels = useCodexAccountStore((s) => s.fetchModels);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);
  const applyConfig = useBridgeStore((s) => s.applyConfig);

  const [selectedModel, setSelectedModel] = useState("");
  const [selectedReasoning, setSelectedReasoning] = useState("");
  const [cwd, setCwd] = useState("");

  const locked = codexTuiRunning;

  // Fetch models on mount
  useEffect(() => {
    fetchModels();
  }, [fetchModels]);

  // Set defaults when models first load
  useEffect(() => {
    if (models.length > 0 && !selectedModel) {
      const first = models[0];
      setSelectedModel(first.slug);
      setSelectedReasoning(
        first.defaultReasoningLevel || first.reasoningLevels[0]?.effort || "",
      );
    }
  }, [models, selectedModel]);

  // Sync from protocol data when connected
  useEffect(() => {
    if (codexAccount?.model) setSelectedModel(codexAccount.model);
    if (codexAccount?.reasoningEffort)
      setSelectedReasoning(codexAccount.reasoningEffort);
    if (codexAccount?.cwd) setCwd(codexAccount.cwd);
  }, [codexAccount?.model, codexAccount?.reasoningEffort, codexAccount?.cwd]);

  const currentModel = useMemo(
    () => models.find((m) => m.slug === selectedModel),
    [models, selectedModel],
  );
  const reasoningOptions = useMemo(
    () => currentModel?.reasoningLevels ?? [],
    [currentModel],
  );

  // Reset reasoning when model changes (only user-driven)
  const handleModelChange = useCallback(
    (slug: string) => {
      setSelectedModel(slug);
      const m = models.find((x) => x.slug === slug);
      if (m) {
        setSelectedReasoning(
          m.defaultReasoningLevel || m.reasoningLevels[0]?.effort || "",
        );
      }
    },
    [models],
  );

  const handlePickDir = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) setCwd(dir);
  }, [pickDirectory]);

  const handleConnect = useCallback(() => {
    applyConfig({
      model: selectedModel || undefined,
      reasoningEffort: selectedReasoning || undefined,
      cwd: cwd || undefined,
    });
    launchCodexTui();
  }, [applyConfig, selectedModel, selectedReasoning, cwd, launchCodexTui]);

  return (
    <div className="rounded-lg border border-input bg-card p-3">
      {/* Header */}
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
        <RoleSelect agent="codex" disabled={locked} />
        <span className="text-[11px] uppercase text-secondary-foreground">
          {codexTuiRunning ? "connected" : codexReady ? "ready" : "starting..."}
        </span>
      </div>

      {/* Thread ID */}
      {threadId && (
        <div className="mt-1 font-mono text-[11px] text-muted-foreground">
          Thread: {threadId.slice(0, 16)}...
        </div>
      )}

      {/* Usage (when connected) */}
      {locked && usage && (
        <div className="mt-2 rounded-md bg-muted/40 px-3 py-2 space-y-1.5">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-1.5">
              <span className="text-[10px] font-semibold uppercase text-muted-foreground">
                {"\u7528\u91CF"}
              </span>
              <span
                className={cn(
                  "rounded-full px-1.5 py-px text-[9px] font-semibold",
                  usage.limitReached || !usage.allowed
                    ? "bg-destructive/10 text-destructive"
                    : "bg-codex/10 text-codex",
                )}
              >
                {usage.limitReached || !usage.allowed
                  ? "\u53D7\u9650"
                  : "\u6B63\u5E38"}
              </span>
            </div>
            <button
              type="button"
              disabled={refreshing}
              onClick={refreshUsage}
              className={cn(
                "text-[10px] text-muted-foreground hover:text-foreground transition-colors",
                refreshing && "opacity-50",
              )}
            >
              {refreshing ? "\u5237\u65B0\u4E2D\u2026" : "\u5237\u65B0"}
            </button>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <MiniMeter
              label={windowLabel(
                usage.primary?.windowMinutes ?? null,
                "\u77ED\u671F",
              )}
              used={usage.primary?.usedPercent ?? 0}
              remaining={usage.primary?.remainingPercent ?? 100}
            />
            <MiniMeter
              label={windowLabel(
                usage.secondary?.windowMinutes ?? null,
                "\u957F\u671F",
              )}
              used={usage.secondary?.usedPercent ?? 0}
              remaining={usage.secondary?.remainingPercent ?? 100}
            />
          </div>
        </div>
      )}

      {/* Config rows — always visible, locked after connection */}
      <div className="mt-2 space-y-1.5">
        {/* Profile (when connected) */}
        {locked && profile?.name && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Account</span>
            <div className="flex items-center gap-1.5">
              <span className="text-[11px] font-medium text-foreground">
                {profile.name}
              </span>
              {profile.planType && (
                <span className="capitalize rounded bg-primary/10 px-1.5 py-0.5 text-[9px] font-semibold text-primary">
                  {profile.planType}
                </span>
              )}
            </div>
          </div>
        )}

        {/* Model */}
        {models.length > 0 && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Model</span>
            <select
              value={selectedModel}
              onChange={(e) => handleModelChange(e.target.value)}
              disabled={locked}
              className={cn(
                "rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-foreground border border-input outline-none",
                locked ? "opacity-50 cursor-not-allowed" : "cursor-pointer",
              )}
            >
              {models.map((m) => (
                <option key={m.slug} value={m.slug}>
                  {m.displayName}
                </option>
              ))}
            </select>
          </div>
        )}

        {/* Reasoning */}
        {reasoningOptions.length > 0 && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Reasoning</span>
            <select
              value={selectedReasoning}
              onChange={(e) => setSelectedReasoning(e.target.value)}
              disabled={locked}
              className={cn(
                "rounded bg-muted px-1.5 py-0.5 text-[10px] font-medium text-foreground border border-input outline-none",
                locked ? "opacity-50 cursor-not-allowed" : "cursor-pointer",
              )}
            >
              {reasoningOptions.map((r) => (
                <option key={r.effort} value={r.effort}>
                  {r.effort}
                </option>
              ))}
            </select>
          </div>
        )}

        {/* Project / CWD */}
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">Project</span>
          <button
            type="button"
            onClick={handlePickDir}
            disabled={locked}
            className={cn(
              "inline-flex items-center gap-1 rounded px-1 py-0.5 font-mono text-[11px] text-secondary-foreground transition-colors truncate max-w-44",
              locked
                ? "opacity-50 cursor-not-allowed"
                : "hover:bg-accent hover:text-primary cursor-pointer",
            )}
            title={cwd}
          >
            <svg
              width="10"
              height="10"
              viewBox="0 0 16 16"
              className="shrink-0 text-muted-foreground"
            >
              <path
                d="M2 4v8h12V6H8L6 4z"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.2"
              />
            </svg>
            {cwd ? shortenPath(cwd) : "Select project..."}
          </button>
        </div>
      </div>

      {/* Disconnect button (when connected) */}
      {locked && (
        <Button
          size="sm"
          variant="secondary"
          className="w-full mt-2"
          onClick={stopCodexTui}
        >
          Disconnect Codex
        </Button>
      )}

      {/* Connect button (when not connected) */}
      {!locked && (
        <Button
          className="w-full mt-2 bg-codex text-white hover:bg-codex/80"
          size="sm"
          disabled={!codexReady}
          onClick={handleConnect}
        >
          Connect Codex
        </Button>
      )}

      {/* Status */}
      {!codexReady && (
        <div className="mt-1.5 text-[11px] text-muted-foreground">
          Codex app-server is starting...
        </div>
      )}
    </div>
  );
}
