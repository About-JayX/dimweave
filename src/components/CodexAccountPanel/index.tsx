import { useState, useEffect, useCallback } from "react";
import { cn } from "@/lib/utils";
import { useBridgeStore } from "@/stores/bridge-store";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { InlineSelect } from "./InlineSelect";
import { ExpandedDetails } from "./ExpandedDetails";
import type { DropdownOption, CodexAccountPanelProps } from "./types";

export type { CodexAccountPanelProps } from "./types";

export function CodexAccountPanel({
  profile,
  usage,
  refreshing,
  onRefresh,
  protocolData,
  locked = false,
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
      {/* Always visible: Model + Reasoning selectors */}
      <div className="px-3 py-2 space-y-1.5">
        {protocolData?.model && (
          <div className="flex items-center justify-between">
            <span className="text-[10px] text-muted-foreground">Model</span>
            {modelOpts.length > 0 ? (
              <InlineSelect
                value={protocolData.model}
                options={modelOpts}
                onSelect={onModel}
                disabled={locked}
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
                disabled={locked}
              />
            ) : (
              <span className="text-[11px] font-medium text-foreground">
                {protocolData.reasoningEffort}
              </span>
            )}
          </div>
        )}
      </div>

      {/* Expand toggle */}
      <button
        type="button"
        onClick={() => setExpanded(!expanded)}
        className="flex w-full items-center justify-center gap-1 border-t border-border/50 py-1 text-[10px] text-muted-foreground hover:text-foreground hover:bg-muted/30 transition-colors"
      >
        <span>{expanded ? "\u6536\u8D77" : "\u66F4\u591A"}</span>
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

      {/* Expanded details */}
      {expanded && (
        <ExpandedDetails
          profile={profile}
          usage={usage}
          refreshing={refreshing}
          onRefresh={onRefresh}
          onCwd={onCwd}
          protocolData={protocolData}
          locked={locked}
        />
      )}
    </div>
  );
}
