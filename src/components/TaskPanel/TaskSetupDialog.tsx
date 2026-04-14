import { useState, useEffect, useCallback } from "react";
import { Plus, Trash2 } from "lucide-react";
import { AgentStatusPanel } from "@/components/AgentStatus";
import type { AgentDraftConfig } from "@/components/AgentStatus/provider-session-view-model";
import type { Provider } from "@/stores/task-store/types";

export interface AgentDef {
  provider: Provider;
  role: string;
}

export interface TaskSetupSubmitPayload {
  agents: AgentDef[];
  claudeConfig: AgentDraftConfig | null;
  codexConfig: AgentDraftConfig | null;
  requestLaunch: boolean;
}

export type TaskSetupMode = "create" | "edit";

interface TaskSetupDialogProps {
  mode?: TaskSetupMode;
  workspace: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSubmit: (payload: TaskSetupSubmitPayload) => void;
  initialAgents?: AgentDef[];
}

const PROVIDERS: Provider[] = ["claude", "codex"];
const DEFAULT_AGENTS: AgentDef[] = [
  { provider: "claude", role: "lead" },
  { provider: "codex", role: "coder" },
];

function AgentDefRow({
  def,
  onChange,
  onRemove,
}: {
  def: AgentDef;
  onChange: (updated: AgentDef) => void;
  onRemove: () => void;
}) {
  return (
    <div className="flex items-center gap-2">
      <select
        value={def.provider}
        onChange={(e) => onChange({ ...def, provider: e.target.value as Provider })}
        className="rounded-lg border border-border/50 bg-background px-2 py-1 text-xs text-foreground outline-none focus:border-primary/40"
      >
        {PROVIDERS.map((p) => (
          <option key={p} value={p}>{p}</option>
        ))}
      </select>
      <input
        type="text"
        value={def.role}
        onChange={(e) => onChange({ ...def, role: e.target.value })}
        placeholder="role"
        className="min-w-0 flex-1 rounded-lg border border-border/50 bg-background px-2 py-1 text-xs text-foreground outline-none placeholder:text-muted-foreground/40 focus:border-primary/40"
      />
      <button
        type="button"
        onClick={onRemove}
        className="rounded p-1 text-muted-foreground hover:bg-rose-500/20 hover:text-rose-400"
      >
        <Trash2 className="size-3" />
      </button>
    </div>
  );
}

export function TaskSetupDialog({
  mode = "create",
  workspace,
  open,
  onOpenChange,
  onSubmit,
  initialAgents,
}: TaskSetupDialogProps) {
  const [agentDefs, setAgentDefs] = useState<AgentDef[]>(
    initialAgents ?? DEFAULT_AGENTS,
  );
  const [claudeConfig, setClaudeConfig] = useState<AgentDraftConfig | null>(null);
  const [codexConfig, setCodexConfig] = useState<AgentDraftConfig | null>(null);

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

  const updateDef = (index: number, updated: AgentDef) => {
    setAgentDefs((prev) => prev.map((d, i) => (i === index ? updated : d)));
  };
  const removeDef = (index: number) => {
    setAgentDefs((prev) => prev.filter((_, i) => i !== index));
  };
  const addDef = () => {
    setAgentDefs((prev) => [...prev, { provider: "claude", role: "" }]);
  };

  const validAgents = agentDefs.filter((d) => d.role.trim().length > 0);
  const hasClaude = validAgents.some((a) => a.provider === "claude");
  const hasCodex = validAgents.some((a) => a.provider === "codex");
  const draftLeadProvider: Provider = validAgents[0]?.provider ?? "claude";
  const draftCoderProvider: Provider = validAgents[1]?.provider ?? validAgents[0]?.provider ?? "codex";

  const submit = (launch: boolean) => {
    onSubmit({
      agents: validAgents,
      claudeConfig,
      codexConfig,
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
        <h3 className="text-sm font-semibold text-foreground">
          {mode === "edit" ? "Edit Task" : "New Task"}
        </h3>

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-medium text-muted-foreground">Agents</span>
            <button
              type="button"
              onClick={addDef}
              className="inline-flex items-center gap-0.5 rounded-md px-1.5 py-0.5 text-[10px] text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              <Plus className="size-3" />
              Add
            </button>
          </div>
          {agentDefs.map((def, i) => (
            <AgentDefRow
              key={i}
              def={def}
              onChange={(u) => updateDef(i, u)}
              onRemove={() => removeDef(i)}
            />
          ))}
        </div>

        <div className="flex items-center justify-end gap-2 pt-1">
          <button type="button" onClick={handleClose}
            className="rounded-lg px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground">
            Cancel
          </button>
          {mode === "edit" ? (
            <button type="button" onClick={() => submit(false)}
              className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90">
              Save
            </button>
          ) : (
            <>
              <button type="button" onClick={() => submit(true)} disabled={validAgents.length === 0}
                className="rounded-lg border border-primary/30 bg-primary/10 px-3 py-1.5 text-xs font-medium text-primary transition-colors hover:bg-primary/20 disabled:opacity-40">
                Create &amp; Connect
              </button>
              <button type="button" onClick={() => submit(false)}
                className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90">
                Create
              </button>
            </>
          )}
        </div>

        {mode === "create" && (
          <AgentStatusPanel workspace={workspace} draftMode
            draftLeadProvider={draftLeadProvider} draftCoderProvider={draftCoderProvider}
            onClaudeDraftChange={hasClaude ? setClaudeConfig : undefined}
            onCodexDraftChange={hasCodex ? setCodexConfig : undefined} />
        )}
      </div>
    </div>
  );
}
