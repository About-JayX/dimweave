import type { HistoryItem, HistoryPickerModel } from "./view-model";
import type { SessionRole } from "@/stores/task-store/types";

const PROVIDER_LABELS: Record<string, string> = {
  claude: "Claude",
  codex: "Codex",
};

function HistoryGroup({
  title,
  items,
  onResume,
  onAttach,
}: {
  title: string;
  items: HistoryItem[];
  onResume: (sessionId: string) => void;
  onAttach: (item: HistoryItem, role: SessionRole) => void;
}) {
  if (items.length === 0) return null;

  return (
    <div className="space-y-2">
      <div className="text-[11px] font-medium text-muted-foreground/70">
        {title}
      </div>
      {items.map((item) => (
        <div
          key={`${item.provider}:${item.externalId}`}
          className="rounded-lg border border-border/35 bg-background/30 px-3 py-2"
        >
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0 space-y-1">
              <div className="flex items-center gap-2">
                <span className="truncate text-sm font-medium text-foreground">
                  {item.title}
                </span>
                <span className="rounded-full border border-border/50 px-1.5 py-0.5 text-[10px] text-muted-foreground">
                  {PROVIDER_LABELS[item.provider] ?? item.provider}
                </span>
              </div>
              {item.preview && (
                <div className="line-clamp-2 text-[11px] text-muted-foreground/75">
                  {item.preview}
                </div>
              )}
              {item.cwd && (
                <div className="truncate text-[10px] text-muted-foreground/50">
                  {item.cwd}
                </div>
              )}
              {item.normalizedTaskId && (
                <div className="text-[10px] text-muted-foreground/45">
                  Task: {item.normalizedTaskId}
                </div>
              )}
              <div className="truncate text-[10px] text-muted-foreground/50">
                {item.externalId}
              </div>
            </div>
            <div className="flex shrink-0 items-center gap-1">
              {item.actions.includes("resume_current") && item.normalizedSessionId && (
                <button
                  className="rounded-md border border-border/50 px-2 py-1 text-[10px] text-foreground transition-colors hover:border-primary/50 hover:text-primary"
                  onClick={() => onResume(item.normalizedSessionId!)}
                >
                  Open
                </button>
              )}
              {item.actions.includes("resume_existing") && item.normalizedSessionId && (
                <button
                  className="rounded-md border border-border/50 px-2 py-1 text-[10px] text-foreground transition-colors hover:border-primary/50 hover:text-primary"
                  onClick={() => onResume(item.normalizedSessionId!)}
                >
                  Resume
                </button>
              )}
              {item.actions.includes("attach_lead") && (
                <button
                  className="rounded-md border border-amber-400/25 px-2 py-1 text-[10px] text-amber-300 transition-colors hover:bg-amber-400/10"
                  onClick={() => onAttach(item, "lead")}
                >
                  As Lead
                </button>
              )}
              {item.actions.includes("attach_coder") && (
                <button
                  className="rounded-md border border-emerald-400/25 px-2 py-1 text-[10px] text-emerald-300 transition-colors hover:bg-emerald-400/10"
                  onClick={() => onAttach(item, "coder")}
                >
                  As Coder
                </button>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

export function HistoryPicker({
  model,
  onResume,
  onAttach,
}: {
  model: HistoryPickerModel;
  onResume: (sessionId: string) => void;
  onAttach: (item: HistoryItem, role: SessionRole) => void;
}) {
  const total =
    model.attached.length + model.elsewhere.length + model.available.length;

  return (
    <section className="rounded-xl border border-border/40 bg-card/50 px-4 py-3 backdrop-blur-sm">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
          History Picker
        </div>
        <div className="text-[10px] text-muted-foreground/50">{total} items</div>
      </div>
      {total === 0 ? (
        <div className="text-xs text-muted-foreground/60">
          No provider history available for this workspace.
        </div>
      ) : (
        <div className="space-y-3">
          <HistoryGroup
            title="Attached To This Task"
            items={model.attached}
            onResume={onResume}
            onAttach={onAttach}
          />
          <HistoryGroup
            title="Mapped In Other Tasks"
            items={model.elsewhere}
            onResume={onResume}
            onAttach={onAttach}
          />
          <HistoryGroup
            title="External Provider History"
            items={model.available}
            onResume={onResume}
            onAttach={onAttach}
          />
        </div>
      )}
    </section>
  );
}
