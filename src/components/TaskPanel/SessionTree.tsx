import type { SessionTreeRow } from "./view-model";

const PROVIDER_LABELS: Record<string, string> = {
  claude: "Claude",
  codex: "Codex",
};

const STATUS_STYLES: Record<string, string> = {
  active: "text-emerald-300 border-emerald-400/25 bg-emerald-400/10",
  paused: "text-muted-foreground border-border/50 bg-muted/20",
  completed: "text-slate-300 border-slate-400/25 bg-slate-400/10",
  error: "text-rose-300 border-rose-400/25 bg-rose-400/10",
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
      <div className="mb-3 text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
        Session Tree
      </div>
      {rows.length === 0 ? (
        <div className="text-xs text-muted-foreground/60">No sessions yet.</div>
      ) : (
        <div className="space-y-2">
          {rows.map((row) => (
            <div
              key={row.sessionId}
              className="rounded-lg border border-border/35 bg-background/30 px-3 py-2"
              style={{ marginLeft: `${row.depth * 18}px` }}
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 space-y-1">
                  <div className="flex items-center gap-2">
                    <span
                      className="min-w-0 truncate text-sm font-medium text-foreground"
                      title={row.session.title}
                    >
                      {row.session.title}
                    </span>
                    <span className="shrink-0 rounded-full border border-border/50 px-1.5 py-0.5 text-[10px] text-muted-foreground">
                      {row.session.role}
                    </span>
                    <span className="shrink-0 rounded-full border border-border/50 px-1.5 py-0.5 text-[10px] text-muted-foreground">
                      {PROVIDER_LABELS[row.session.provider] ??
                        row.session.provider}
                    </span>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <span
                      className={`rounded-full border px-1.5 py-0.5 text-[10px] ${STATUS_STYLES[row.session.status] ?? STATUS_STYLES.paused}`}
                    >
                      {row.session.status}
                    </span>
                    {row.session.externalSessionId && (
                      <span className="truncate text-[10px] text-muted-foreground/55">
                        {row.session.externalSessionId}
                      </span>
                    )}
                  </div>
                </div>
                <button
                  className="shrink-0 rounded-md border border-border/50 px-2 py-1 text-[10px] text-foreground transition-colors hover:border-primary/50 hover:text-primary"
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
