import { useEffect } from "react";
import { CyberSelect } from "@/components/ui/cyber-select";
import { useClaudeAccountStore } from "@/stores/claude-account-store";

/** Fallback used only when /v1/models fetch fails (offline, no auth, etc.). */
export const CLAUDE_MODEL_OPTIONS = [
  { value: "", label: "Default" },
  { value: "sonnet", label: "Sonnet (latest)" },
  { value: "opus", label: "Opus (latest)" },
  { value: "claude-sonnet-4-6", label: "Sonnet 4.6" },
  { value: "claude-opus-4-6", label: "Opus 4.6" },
  { value: "claude-haiku-4-5", label: "Haiku 4.5" },
];

export const CLAUDE_EFFORT_OPTIONS = [
  { value: "", label: "Default" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "max", label: "Max (Opus only)" },
];

interface ClaudeConfigRowsProps {
  model: string;
  effort: string;
  disabled: boolean;
  onModelChange: (v: string) => void;
  onEffortChange: (v: string) => void;
}

export function ClaudeConfigRows({
  model,
  effort,
  disabled,
  onModelChange,
  onEffortChange,
}: ClaudeConfigRowsProps) {
  const models = useClaudeAccountStore((s) => s.models);
  const fetchModels = useClaudeAccountStore((s) => s.fetchModels);
  useEffect(() => {
    if (models.length === 0) fetchModels();
  }, [models.length, fetchModels]);
  const modelOptions = models.length
    ? [
        { value: "", label: "Default" },
        ...models.map((m) => ({ value: m.slug, label: m.displayName })),
      ]
    : CLAUDE_MODEL_OPTIONS;
  const selected = models.find((m) => m.slug === model);
  const effortOptions = selected
    ? [
        { value: "", label: "Default" },
        ...selected.supportedEfforts.map((e) => ({ value: e, label: e })),
      ]
    : CLAUDE_EFFORT_OPTIONS;
  return (
    <div className="mt-2 space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">Model</span>
        <CyberSelect
          value={model}
          options={modelOptions}
          onChange={onModelChange}
          disabled={disabled}
        />
      </div>

      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">Effort</span>
        <CyberSelect
          value={effort}
          options={effortOptions}
          onChange={onEffortChange}
          disabled={disabled}
        />
      </div>
    </div>
  );
}
