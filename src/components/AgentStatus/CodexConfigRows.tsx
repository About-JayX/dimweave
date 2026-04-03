import { shortenPath } from "@/lib/utils";
import { CyberSelect } from "@/components/ui/cyber-select";

interface CodexConfigRowsProps {
  locked: boolean;
  profile: { name?: string; planType?: string } | null;
  models: { slug: string }[];
  selectedModel: string;
  modelSelectOptions: { value: string; label: string }[];
  handleModelChange: (slug: string) => void;
  reasoningOptions: { effort: string }[];
  selectedReasoning: string;
  setSelectedReasoning: (v: string) => void;
  reasoningSelectOptions: { value: string; label: string }[];
  cwd: string;
}

export function CodexConfigRows({
  locked,
  profile,
  models,
  selectedModel,
  modelSelectOptions,
  handleModelChange,
  reasoningOptions,
  selectedReasoning,
  setSelectedReasoning,
  reasoningSelectOptions,
  cwd,
}: CodexConfigRowsProps) {
  return (
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
          <CyberSelect
            value={selectedModel}
            options={modelSelectOptions}
            onChange={handleModelChange}
            disabled={locked}
          />
        </div>
      )}

      {/* Reasoning */}
      {reasoningOptions.length > 0 && (
        <div className="flex items-center justify-between">
          <span className="text-[10px] text-muted-foreground">Reasoning</span>
          <CyberSelect
            value={selectedReasoning}
            options={reasoningSelectOptions}
            onChange={setSelectedReasoning}
            disabled={locked}
          />
        </div>
      )}

      {/* Project / CWD */}
      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">Project</span>
        <span
          className="max-w-44 truncate font-mono text-[11px] text-secondary-foreground"
          title={cwd || "Workspace required"}
        >
          {cwd ? shortenPath(cwd) : "Workspace required"}
        </span>
      </div>
    </div>
  );
}
