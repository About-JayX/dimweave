import { useState, useRef, useEffect, useCallback } from "react";
import { cn } from "@/lib/utils";
import { useBridgeStore } from "@/stores/bridge-store";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import type { CodexAccountInfo } from "@/types";

// ── Inline dropdown (scroll-safe) ──────────────────────────

interface DropdownOption {
  value: string;
  label: string;
  description?: string;
}

function InlineSelect({
  value,
  options,
  onSelect,
}: {
  value: string;
  options: DropdownOption[];
  onSelect: (value: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node))
        setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  return (
    <div ref={ref} className="relative inline-flex">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="inline-flex items-center gap-0.5 rounded px-1 py-0.5 font-mono text-[11px] font-medium text-foreground hover:bg-accent transition-colors cursor-pointer"
      >
        {value}
        <svg
          width="8"
          height="8"
          viewBox="0 0 12 12"
          className="text-muted-foreground"
        >
          <path
            d="M3 5l3 3 3-3"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          />
        </svg>
      </button>
      {open && (
        <div className="absolute right-0 top-6 z-50 min-w-40 max-h-48 overflow-y-auto rounded-lg border border-border bg-popover p-1 shadow-xl">
          {options.map((opt) => (
            <button
              key={opt.value}
              type="button"
              onClick={() => {
                onSelect(opt.value);
                setOpen(false);
              }}
              className={cn(
                "flex w-full flex-col items-start rounded-md px-2.5 py-1.5 text-left text-[11px] transition-colors",
                "hover:bg-accent hover:text-accent-foreground",
                opt.value === value && "bg-accent/60 text-accent-foreground",
              )}
            >
              <span className="font-medium">{opt.label}</span>
              {opt.description && (
                <span className="text-[10px] text-muted-foreground">
                  {opt.description}
                </span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

// ── Helpers ────────────────────────────────────────────────

function shortenPath(p: string): string {
  const idx = p.indexOf("/Users/");
  if (idx >= 0) {
    const rest = p.slice(idx + 7);
    const slash = rest.indexOf("/");
    return slash >= 0 ? `~${rest.slice(slash)}` : "~";
  }
  return p;
}

function windowLabel(mins: number | null, fb: string): string {
  if (!mins) return fb;
  if (mins === 300) return "5h";
  if (mins === 10080) return "7d";
  return mins % 60 === 0 ? `${mins / 60}h` : `${mins}m`;
}

function barColor(used: number) {
  if (used >= 90) return "bg-destructive";
  if (used >= 75) return "bg-yellow-500";
  return "bg-codex";
}

// ── Types ──────────────────────────────────────────────────

interface CodexProfile {
  name?: string;
  planType?: string;
}
interface UsageWindow {
  usedPercent: number;
  remainingPercent: number;
  windowMinutes: number | null;
}
interface UsageSnapshot {
  source: string;
  allowed: boolean;
  limitReached: boolean;
  primary: UsageWindow | null;
  secondary: UsageWindow | null;
}

export interface CodexAccountPanelProps {
  profile: CodexProfile | null;
  usage: UsageSnapshot | null;
  refreshing: boolean;
  onRefresh: () => void;
  protocolData?: CodexAccountInfo;
}

// ── Component ──────────────────────────────────────────────

export function CodexAccountPanel({
  profile,
  usage,
  refreshing,
  onRefresh,
  protocolData,
}: CodexAccountPanelProps) {
  const [expanded, setExpanded] = useState(false);
  const models = useCodexAccountStore((s) => s.models);
  const fetchModels = useCodexAccountStore((s) => s.fetchModels);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);
  const applyConfig = useBridgeStore((s) => s.applyConfig);

  const shouldRender = !!(profile || usage || protocolData?.model);
  useEffect(() => {
    if (shouldRender) fetchModels();
  }, [fetchModels, shouldRender]);

  const currentModel = models.find((m) => m.slug === protocolData?.model);
  const modelOpts: DropdownOption[] = models.map((m) => ({
    value: m.slug,
    label: m.displayName,
  }));
  const reasoningOpts: DropdownOption[] = (
    currentModel?.reasoningLevels ?? []
  ).map((r) => ({
    value: r.effort,
    label: r.effort,
    description: r.description,
  }));

  const onModel = useCallback(
    (v: string) => applyConfig({ model: v }),
    [applyConfig],
  );
  const onReasoning = useCallback(
    (v: string) => applyConfig({ reasoningEffort: v }),
    [applyConfig],
  );
  const onCwd = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) applyConfig({ cwd: dir });
  }, [pickDirectory, applyConfig]);

  // Early return AFTER all hooks to comply with React Rules of Hooks
  if (!shouldRender) return null;

  return (
    <div className="mt-2 rounded-lg bg-muted/40">
      {/* ── Always visible: Model + Reasoning selectors ── */}
      <div className="px-3 py-2 space-y-1.5">
        {protocolData?.model && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Model</span>
            {modelOpts.length > 0 ? (
              <InlineSelect
                value={protocolData.model}
                options={modelOpts}
                onSelect={onModel}
              />
            ) : (
              <span className="font-mono text-[11px] font-medium text-foreground">
                {protocolData.model}
              </span>
            )}
          </div>
        )}

        {protocolData?.reasoningEffort && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Reasoning</span>
            {reasoningOpts.length > 0 ? (
              <InlineSelect
                value={protocolData.reasoningEffort}
                options={reasoningOpts}
                onSelect={onReasoning}
              />
            ) : (
              <span className="text-[11px] font-medium text-foreground">
                {protocolData.reasoningEffort}
              </span>
            )}
          </div>
        )}
      </div>

      {/* ── Expand toggle ── */}
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex w-full items-center justify-center gap-1 border-t border-border/50 py-1 text-[10px] text-muted-foreground hover:text-foreground hover:bg-muted/30 transition-colors"
      >
        <span>{expanded ? "收起" : "更多"}</span>
        <svg
          width="8"
          height="8"
          viewBox="0 0 12 12"
          className={cn(
            "transition-transform duration-150",
            !expanded && "rotate-180",
          )}
        >
          <path
            d="M3 7l3-3 3 3"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
          />
        </svg>
      </button>

      {/* ── Expanded details ── */}
      {expanded && (
        <div className="border-t border-border/50 px-3 py-2 space-y-2.5">
          {/* Identity */}
          {(profile?.name || profile?.planType) && (
            <div className="flex items-center justify-between text-[11px]">
              <span className="text-foreground font-medium">
                {profile?.name}
              </span>
              {profile?.planType && (
                <span className="capitalize rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-semibold text-primary">
                  {profile.planType}
                </span>
              )}
            </div>
          )}

          {/* Project */}
          {protocolData?.cwd && (
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-muted-foreground">Project</span>
              <button
                type="button"
                onClick={onCwd}
                className="inline-flex items-center gap-1 rounded px-1 py-0.5 font-mono text-[11px] text-secondary-foreground hover:bg-accent hover:text-primary transition-colors truncate max-w-44"
                title={protocolData.cwd}
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
                {shortenPath(protocolData.cwd)}
              </button>
            </div>
          )}

          {/* Usage */}
          {usage && (
            <div className="space-y-1.5">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-1.5">
                  <span className="text-[10px] font-semibold uppercase text-muted-foreground">
                    用量
                  </span>
                  <span
                    className={cn(
                      "rounded-full px-1.5 py-px text-[9px] font-semibold",
                      usage.limitReached || !usage.allowed
                        ? "bg-destructive/10 text-destructive"
                        : "bg-codex/10 text-codex",
                    )}
                  >
                    {usage.limitReached || !usage.allowed ? "受限" : "正常"}
                  </span>
                </div>
                <button
                  type="button"
                  disabled={refreshing}
                  onClick={onRefresh}
                  className={cn(
                    "text-[10px] text-muted-foreground hover:text-foreground transition-colors",
                    refreshing && "opacity-50",
                  )}
                >
                  {refreshing ? "刷新中…" : "刷新"}
                </button>
              </div>
              <div className="grid grid-cols-2 gap-3">
                <MiniMeter
                  label={windowLabel(
                    usage.primary?.windowMinutes ?? null,
                    "短期",
                  )}
                  used={usage.primary?.usedPercent ?? 0}
                  remaining={usage.primary?.remainingPercent ?? 100}
                />
                <MiniMeter
                  label={windowLabel(
                    usage.secondary?.windowMinutes ?? null,
                    "长期",
                  )}
                  used={usage.secondary?.usedPercent ?? 0}
                  remaining={usage.secondary?.remainingPercent ?? 100}
                />
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function MiniMeter({
  label,
  used,
  remaining,
}: {
  label: string;
  used: number;
  remaining: number;
}) {
  const u = Math.min(used, 100);
  return (
    <div>
      <div className="flex items-center justify-between text-[10px] mb-1">
        <span className="text-muted-foreground">{label}</span>
        <span
          className={cn(
            "font-mono font-semibold",
            u >= 90 ? "text-destructive" : "text-foreground",
          )}
        >
          {Math.round(remaining)}%
        </span>
      </div>
      <div className="h-1.5 rounded-full bg-secondary overflow-hidden">
        <div
          className={cn("h-full rounded-full transition-all", barColor(u))}
          style={{ width: `${u}%` }}
        />
      </div>
    </div>
  );
}
