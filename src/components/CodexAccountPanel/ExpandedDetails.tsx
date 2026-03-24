import { cn } from "@/lib/utils";
import { MiniMeter } from "./MiniMeter";
import { shortenPath, windowLabel } from "./helpers";
import type { CodexProfile, UsageSnapshot } from "./types";
import type { CodexAccountInfo } from "@/types";

interface ExpandedDetailsProps {
  profile: CodexProfile | null;
  usage: UsageSnapshot | null;
  refreshing: boolean;
  onRefresh: () => void;
  onCwd: () => void;
  protocolData?: CodexAccountInfo;
  locked?: boolean;
}

export function ExpandedDetails({
  profile,
  usage,
  refreshing,
  onRefresh,
  onCwd,
  protocolData,
  locked = false,
}: ExpandedDetailsProps) {
  return (
    <div className="border-t border-border/50 px-3 py-2 space-y-2.5">
      {/* Identity */}
      {(profile?.name || profile?.planType) && (
        <div className="flex items-center justify-between text-[11px]">
          <span className="text-foreground font-medium">{profile?.name}</span>
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
            disabled={locked}
            className={cn(
              "inline-flex items-center gap-1 rounded px-1 py-0.5 font-mono text-[11px] text-secondary-foreground transition-colors truncate max-w-44",
              locked
                ? "opacity-50 cursor-not-allowed"
                : "hover:bg-accent hover:text-primary cursor-pointer",
            )}
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
              onClick={onRefresh}
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
    </div>
  );
}
