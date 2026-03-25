import { cn, shortenPath } from "@/lib/utils";
import { CyberSelect } from "@/components/ui/cyber-select";

const MODEL_OPTIONS = [
  { value: "", label: "Default" },
  { value: "sonnet", label: "Sonnet (latest)" },
  { value: "opus", label: "Opus (latest)" },
  { value: "claude-sonnet-4-6", label: "Sonnet 4.6" },
  { value: "claude-opus-4-6", label: "Opus 4.6" },
  { value: "claude-haiku-4-5", label: "Haiku 4.5" },
];

const EFFORT_OPTIONS = [
  { value: "", label: "Default" },
  { value: "low", label: "Low" },
  { value: "medium", label: "Medium" },
  { value: "high", label: "High" },
  { value: "max", label: "Max (Opus only)" },
];

interface ClaudeConfigRowsProps {
  model: string;
  effort: string;
  cwd: string;
  disabled: boolean;
  onModelChange: (v: string) => void;
  onEffortChange: (v: string) => void;
  onPickDir: () => void;
}

export function ClaudeConfigRows({
  model,
  effort,
  cwd,
  disabled,
  onModelChange,
  onEffortChange,
  onPickDir,
}: ClaudeConfigRowsProps) {
  return (
    <div className="mt-2 space-y-1.5">
      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">Model</span>
        <CyberSelect
          value={model}
          options={MODEL_OPTIONS}
          onChange={onModelChange}
          disabled={disabled}
        />
      </div>

      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">Effort</span>
        <CyberSelect
          value={effort}
          options={EFFORT_OPTIONS}
          onChange={onEffortChange}
          disabled={disabled}
        />
      </div>

      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">Project</span>
        <button
          type="button"
          onClick={onPickDir}
          disabled={disabled}
          className={cn(
            "inline-flex items-center gap-1 rounded px-1 py-0.5 font-mono text-[11px] text-secondary-foreground transition-colors truncate max-w-44",
            disabled
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
  );
}
