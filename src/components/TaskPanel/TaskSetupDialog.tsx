import { useState } from "react";
import type { Provider } from "@/stores/task-store/types";

export type TaskSetupMode = "create" | "edit";

export interface TaskSetupSubmitPayload {
  title: string;
  leadProvider: Provider;
  coderProvider: Provider;
}

interface TaskSetupDialogProps {
  mode: TaskSetupMode;
  workspace: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (payload: TaskSetupSubmitPayload) => void;
  initialTitle?: string;
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
  workspace,
  open,
  onOpenChange,
  onSubmit,
  initialTitle = "",
  initialLeadProvider = "claude",
  initialCoderProvider = "codex",
}: TaskSetupDialogProps) {
  const [title, setTitle] = useState(initialTitle);
  const [leadProvider, setLeadProvider] =
    useState<Provider>(initialLeadProvider);
  const [coderProvider, setCoderProvider] =
    useState<Provider>(initialCoderProvider);

  if (!open) return null;

  const heading = mode === "create" ? "New Task" : "Edit Task";
  const submitLabel = mode === "create" ? "Create" : "Save";

  const handleSubmit = () => {
    const finalTitle =
      title.trim() ||
      workspace
        .split(/[\\/]/)
        .filter(Boolean)
        .at(-1) ||
      "Untitled";
    onSubmit({
      title: finalTitle,
      leadProvider,
      coderProvider,
    });
    onOpenChange(false);
  };

  return (
    <div className="rounded-xl border border-border/50 bg-card/80 p-4 space-y-3">
      <h3 className="text-sm font-semibold text-foreground">{heading}</h3>

      <div className="space-y-2">
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted-foreground">Title</span>
          <input
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={
              workspace
                .split(/[\\/]/)
                .filter(Boolean)
                .at(-1) || "Task title"
            }
            className="rounded-lg border border-border/50 bg-background px-2.5 py-1.5 text-xs text-foreground outline-none placeholder:text-muted-foreground/50 focus:border-primary/40"
          />
        </label>

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
    </div>
  );
}
