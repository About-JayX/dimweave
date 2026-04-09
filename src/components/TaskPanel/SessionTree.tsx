import type { SessionTreeRow } from "./view-model";

const PROVIDER_LABELS: Record<string, string> = {
  claude: "Claude",
  codex: "Codex",
};

const STATUS_STYLES: Record<string, string> = {
  active: "text-emerald-300 border-emerald-500/50 bg-emerald-500/15",
  paused: "text-zinc-400 border-zinc-500/50 bg-zinc-500/10",
  completed: "text-slate-300 border-slate-500/50 bg-slate-500/15",
  error: "text-rose-300 border-rose-500/50 bg-rose-500/15",
};

export function SessionTree({
  rows,
  onResume,
}: {
  rows: SessionTreeRow[];
  onResume: (sessionId: string) => void;
}) {
  return (
    <section className="px-4 py-3">
      <div className="mb-2.5 text-xs font-medium text-muted-foreground/80">
        Sessions
      </div>
      {rows.length === 0 ? (
        <div className="text-xs text-muted-foreground/60">No sessions yet.</div>
      ) : (
        <div className="space-y-2">
          {rows.map((row) => (
            <div
              key={row.sessionId}
              className="rounded-lg border border-border/50 bg-card/40 px-3 py-2.5"
              style={{ marginLeft: `${row.depth * 18}px` }}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 space-y-1.5">
                  <div className="flex items-center gap-2">
                    <span
                      className="min-w-0 truncate text-sm font-medium text-foreground"
                      title={row.session.title}
                    >
                      {row.session.title}
                    </span>
                    <span className="shrink-0 rounded-full border border-border/60 bg-muted/30 px-1.5 py-0.5 text-[11px] text-muted-foreground">
                      {row.session.role}
                    </span>
                    <span className="shrink-0 rounded-full border border-border/60 bg-muted/30 px-1.5 py-0.5 text-[11px] text-muted-foreground">
                      {PROVIDER_LABELS[row.session.provider] ??
                        row.session.provider}
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <span
                      className={`rounded-full border px-1.5 py-0.5 text-[11px] font-medium ${STATUS_STYLES[row.session.status] ?? STATUS_STYLES.paused}`}
                    >
                      {row.session.status}
                    </span>
                    {row.session.externalSessionId && (
                      <span className="truncate text-[11px] text-muted-foreground/60">
                        {row.session.externalSessionId}
                      </span>
                    )}
                  </div>
                </div>
                <button
                  className="shrink-0 rounded-md border border-border/60 bg-muted/20 px-2.5 py-1 text-[11px] font-medium text-foreground transition-colors hover:border-primary/60 hover:bg-primary/10 hover:text-primary active:bg-primary/20 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary/50"
                  onClick={() => onResume(row.sessionId)}
                >
                  Resume
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
