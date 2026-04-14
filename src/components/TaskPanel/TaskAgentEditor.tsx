import { useCallback, useEffect, useState } from "react";
import type { Provider, TaskAgentInfo } from "@/stores/task-store/types";

export interface AgentEditorPayload {
  provider: Provider;
  role: string;
  displayName: string | null;
}

const PROVIDERS: Provider[] = ["claude", "codex"];

export function TaskAgentEditor({
  agent,
  onSubmit,
  onCancel,
}: {
  agent: TaskAgentInfo | null;
  onSubmit: (payload: AgentEditorPayload) => void;
  onCancel: () => void;
}) {
  const [provider, setProvider] = useState<Provider>(agent?.provider ?? "claude");
  const [role, setRole] = useState(agent?.role ?? "");
  const [displayName, setDisplayName] = useState(agent?.displayName ?? "");

  const handleClose = useCallback(() => onCancel(), [onCancel]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [handleClose]);

  const canSubmit = role.trim().length > 0;

  const handleSubmit = () => {
    if (!canSubmit) return;
    onSubmit({
      provider,
      role: role.trim(),
      displayName: displayName.trim() || null,
    });
  };

  const heading = agent ? "Edit Agent" : "Add Agent";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={handleClose}
      />
      <div
        role="dialog"
        aria-modal="true"
        className="relative z-10 w-full max-w-sm rounded-xl border border-border/50 bg-card p-4 shadow-xl space-y-3"
      >
        <h3 className="text-sm font-semibold text-foreground">{heading}</h3>

        <label className="block space-y-1">
          <span className="text-xs text-muted-foreground">Provider</span>
          <select
            value={provider}
            onChange={(e) => setProvider(e.target.value as Provider)}
            className="block w-full rounded-lg border border-border/50 bg-background px-2 py-1.5 text-xs text-foreground outline-none focus:border-primary/40"
          >
            {PROVIDERS.map((p) => (
              <option key={p} value={p}>{p}</option>
            ))}
          </select>
        </label>

        <label className="block space-y-1">
          <span className="text-xs text-muted-foreground">Role</span>
          <input
            type="text"
            value={role}
            onChange={(e) => setRole(e.target.value)}
            placeholder="e.g. lead, coder, reviewer"
            className="block w-full rounded-lg border border-border/50 bg-background px-2 py-1.5 text-xs text-foreground outline-none placeholder:text-muted-foreground/40 focus:border-primary/40"
          />
        </label>

        <label className="block space-y-1">
          <span className="text-xs text-muted-foreground">
            Display name <span className="text-muted-foreground/40">(optional)</span>
          </span>
          <input
            type="text"
            value={displayName}
            onChange={(e) => setDisplayName(e.target.value)}
            placeholder="e.g. Claude Lead"
            className="block w-full rounded-lg border border-border/50 bg-background px-2 py-1.5 text-xs text-foreground outline-none placeholder:text-muted-foreground/40 focus:border-primary/40"
          />
        </label>

        <div className="flex items-center justify-end gap-2 pt-1">
          <button
            type="button"
            onClick={handleClose}
            className="rounded-lg px-3 py-1.5 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={!canSubmit}
            onClick={handleSubmit}
            className="rounded-lg bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/90 disabled:opacity-40"
          >
            {agent ? "Save" : "Add"}
          </button>
        </div>
      </div>
    </div>
  );
}
