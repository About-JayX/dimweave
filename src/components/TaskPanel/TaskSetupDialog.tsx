import { useState } from "react";
import { AgentStatusPanel } from "@/components/AgentStatus";
import type { Provider } from "@/stores/task-store/types";

export type TaskSetupMode = "create" | "edit";

export interface TaskSetupSubmitPayload {
  leadProvider: Provider;
  coderProvider: Provider;
}

interface TaskSetupDialogProps {
  mode: TaskSetupMode;
  workspace: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (payload: TaskSetupSubmitPayload) => void;
  initialLeadProvider?: Provider;
  initialCoderProvider?: Provider;
}

const PROVIDERS: Provider[] = ["claude", "codex"];

function ProviderSelect({
  label,
  value,
  onChange,
}: {
  label: string;
  value: Provider;
  onChange: (v: Provider) => void;
}) {
  return (
    <label className="flex items-center justify-between gap-2">
      <span className="text-xs text-muted-foreground">{label}</span>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value as Provider)}
        className="rounded-lg border border-border/50 bg-background px-2 py-1 text-xs text-foreground outline-none focus:border-primary/40"
      >
        {PROVIDERS.map((p) => (
          <option key={p} value={p}>
            {p}
          </option>
        ))}
      </select>
    </label>
  );
}

export function TaskSetupDialog({
  mode,
  open,
  onOpenChange,
  onSubmit,
  initialLeadProvider = "claude",
  initialCoderProvider = "codex",
}: TaskSetupDialogProps) {
  const [leadProvider, setLeadProvider] =
    useState<Provider>(initialLeadProvider);
  const [coderProvider, setCoderProvider] =
    useState<Provider>(initialCoderProvider);

  if (!open) return null;

  const heading = mode === "create" ? "New Task" : "Edit Task";
  const submitLabel = mode === "create" ? "Create" : "Save";

  const handleSubmit = () => {
    onSubmit({ leadProvider, coderProvider });
    onOpenChange(false);
  };

  return (
    <div className="rounded-xl border border-border/50 bg-card/80 p-4 space-y-3">
      <h3 className="text-sm font-semibold text-foreground">{heading}</h3>

      <div className="space-y-2">
        <ProviderSelect
          label="Lead provider"
          value={leadProvider}
          onChange={setLeadProvider}
        />
        <ProviderSelect
          label="Coder provider"
          value={coderProvider}
          onChange={setCoderProvider}
        />
      </div>

      <div className="flex items-center justify-end gap-2 pt-1">
        <button
          type="button"
          onClick={() => onOpenChange(false)}
          className="rounded-lg px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
        >
          Cancel
        </button>
        <button
          type="button"
          onClick={handleSubmit}
          className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
        >
          {submitLabel}
        </button>
      </div>

      {mode === "edit" && <AgentStatusPanel />}
    </div>
  );
}
