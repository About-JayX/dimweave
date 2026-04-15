import { CyberSelect } from "@/components/ui/cyber-select";

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
  return (
    <div className="mt-2 space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">Model</span>
        <CyberSelect
          value={model}
          options={CLAUDE_MODEL_OPTIONS}
          onChange={onModelChange}
          disabled={disabled}
        />
      </div>

      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">Effort</span>
        <CyberSelect
          value={effort}
          options={CLAUDE_EFFORT_OPTIONS}
          onChange={onEffortChange}
          disabled={disabled}
        />
      </div>
    </div>
  );
}
