import type { ArtifactDetailModel, ArtifactTimelineItem } from "./view-model";

const KIND_LABELS: Record<string, string> = {
  research: "Research",
  plan: "Plan",
  review: "Review",
  diff: "Diff",
  verification: "Verification",
  summary: "Summary",
};

const KIND_STYLES: Record<string, string> = {
  research: "border-cyan-500/50 bg-cyan-500/10 text-cyan-300",
  plan: "border-sky-500/50 bg-sky-500/10 text-sky-300",
  review: "border-amber-500/50 bg-amber-500/10 text-amber-300",
  diff: "border-indigo-500/50 bg-indigo-500/10 text-indigo-300",
  verification: "border-emerald-500/50 bg-emerald-500/10 text-emerald-300",
  summary: "border-violet-500/50 bg-violet-500/10 text-violet-300",
};

const DEFAULT_KIND_STYLE = "border-zinc-500/50 bg-zinc-500/10 text-zinc-400";

export function ArtifactTimeline({
  items,
  selectedArtifactId,
  detail,
  detailLoading,
  detailError,
  onSelect,
}: {
  items: ArtifactTimelineItem[];
  selectedArtifactId: string | null;
  detail: ArtifactDetailModel | null;
  detailLoading: boolean;
  detailError: string | null;
  onSelect: (artifactId: string) => void;
}) {
  return (
    <section className="px-4 py-3">
      <div className="mb-2.5 text-xs font-medium text-muted-foreground/80">
        Artifacts
      </div>
      {items.length === 0 ? (
        <div className="text-xs text-muted-foreground/60">
          No task artifacts captured yet.
        </div>
      ) : (
        <div className="space-y-3">
          {items.map((item) => (
            <button
              key={item.artifactId}
              type="button"
              onClick={() => onSelect(item.artifactId)}
              aria-pressed={selectedArtifactId === item.artifactId}
              className="w-full rounded-lg border border-border/50 bg-card/40 px-3 py-2.5 text-left transition-colors hover:border-border/70 hover:bg-card/60 aria-pressed:border-primary/50 aria-pressed:bg-primary/8"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate text-sm font-medium text-foreground">
                      {item.title}
                    </span>
                    <span
                      className={`rounded-full border px-1.5 py-0.5 text-[11px] font-medium ${KIND_STYLES[item.kind] ?? DEFAULT_KIND_STYLE}`}
                    >
                      {KIND_LABELS[item.kind] ?? item.kind}
                    </span>
                  </div>
                  <div className="text-xs text-muted-foreground/70">
                    {item.sessionTitle}
                  </div>
                  <div className="truncate font-mono text-[11px] text-muted-foreground/50">
                    {item.contentRef}
                  </div>
                </div>
                <div className="shrink-0 text-[11px] text-muted-foreground/60">
                  {new Date(item.createdAt).toLocaleString()}
                </div>
              </div>
            </button>
          ))}
          {selectedArtifactId ? (
            <div className="rounded-xl border border-border/50 bg-card/50 px-3 py-3">
              <div className="text-xs font-medium uppercase tracking-wider text-muted-foreground/70">
                Artifact detail
              </div>
              {detailLoading ? (
                <div className="mt-2 text-xs text-muted-foreground/70">
                  Loading artifact preview…
                </div>
              ) : detailError ? (
                <div className="mt-2 rounded-lg border border-rose-500/40 bg-rose-500/10 px-3 py-2 text-xs text-rose-300">
                  {detailError}
                </div>
              ) : detail ? (
                <div className="mt-2 space-y-2">
                  <div className="text-sm font-medium text-foreground">
                    {detail.headline}
                  </div>
                  <div className="whitespace-pre-wrap break-words rounded-lg border border-border/50 bg-background/70 px-3 py-2 font-mono text-xs leading-relaxed text-muted-foreground">
                    {detail.body}
                  </div>
                  <div className="text-xs text-muted-foreground/70">
                    {detail.meta}
                  </div>
                </div>
              ) : null}
            </div>
          ) : null}
        </div>
      )}
    </section>
  );
}
