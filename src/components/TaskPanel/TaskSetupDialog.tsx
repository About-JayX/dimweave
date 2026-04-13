import { useState, useEffect, useCallback } from "react";
import { AgentStatusPanel } from "@/components/AgentStatus";
import type { AgentDraftConfig } from "@/components/AgentStatus/provider-session-view-model";
import { useBridgeStore } from "@/stores/bridge-store";
import type { Provider } from "@/stores/task-store/types";

export type TaskSetupMode = "create" | "edit";

export interface TaskSetupSubmitPayload {
  leadProvider: Provider;
  coderProvider: Provider;
  claudeConfig: AgentDraftConfig | null;
  codexConfig: AgentDraftConfig | null;
  claudeRole: string;
  codexRole: string;
  requestLaunch: boolean;
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
  workspace,
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
  const [claudeConfig, setClaudeConfig] = useState<AgentDraftConfig | null>(
    null,
  );
  const [codexConfig, setCodexConfig] = useState<AgentDraftConfig | null>(null);
  const initClaudeRole = useBridgeStore((s) => s.claudeRole);
  const initCodexRole = useBridgeStore((s) => s.codexRole);
  const [draftClaudeRole, setDraftClaudeRole] = useState(initClaudeRole);
  const [draftCodexRole, setDraftCodexRole] = useState(initCodexRole);

  const handleClose = useCallback(() => onOpenChange(false), [onOpenChange]);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [open, handleClose]);

  if (!open) return null;

  const heading = mode === "create" ? "New Task" : "Edit Task";
  const submitLabel = mode === "create" ? "Create" : "Save";
  const draftMode = mode === "create";

  const submit = (launch: boolean) => {
    onSubmit({
      leadProvider,
      coderProvider,
      claudeConfig,
      codexConfig,
      claudeRole: draftClaudeRole,
      codexRole: draftCodexRole,
      requestLaunch: launch,
    });
    onOpenChange(false);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={handleClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        className="relative z-10 w-full max-w-lg overflow-y-auto max-h-[90vh] rounded-xl border border-border/50 bg-card p-4 shadow-xl space-y-3"
      >
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
            onClick={handleClose}
            className="rounded-lg px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            Cancel
          </button>
          {draftMode && (
            <button
              type="button"
              onClick={() => submit(true)}
              className="rounded-lg border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20"
            >
              Create &amp; Connect
            </button>
          )}
          <button
            type="button"
            onClick={() => submit(false)}
            className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90"
          >
            {submitLabel}
          </button>
        </div>

        <AgentStatusPanel
          workspace={workspace}
          draftMode={draftMode}
          draftLeadProvider={leadProvider}
          draftCoderProvider={coderProvider}
          onClaudeDraftChange={setClaudeConfig}
          onCodexDraftChange={setCodexConfig}
          draftClaudeRole={draftMode ? draftClaudeRole : undefined}
          draftCodexRole={draftMode ? draftCodexRole : undefined}
          onDraftClaudeRoleChange={draftMode ? setDraftClaudeRole : undefined}
          onDraftCodexRoleChange={draftMode ? setDraftCodexRole : undefined}
        />
      </div>
    </div>
  );
}
