import type { ArtifactDetailModel, ArtifactTimelineItem } from "./view-model";

const KIND_LABELS: Record<string, string> = {
  research: "Research",
  plan: "Plan",
  review: "Review",
  diff: "Diff",
  verification: "Verification",
  summary: "Summary",
};

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
      <div className="mb-2 text-[11px] font-medium text-muted-foreground/60">
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
              className="w-full rounded-lg border border-border/35 bg-background/30 px-3 py-2 text-left transition-colors hover:border-border/60 hover:bg-background/45 aria-pressed:border-primary/45 aria-pressed:bg-background/60"
            >
              <div className="flex items-start justify-between gap-3">
                <div className="min-w-0 space-y-1">
                  <div className="flex items-center gap-2">
                    <span className="truncate text-sm font-medium text-foreground">
                      {item.title}
                    </span>
                    <span className="rounded-full border border-border/50 px-1.5 py-0.5 text-[10px] text-muted-foreground">
                      {KIND_LABELS[item.kind] ?? item.kind}
                    </span>
                  </div>
                  <div className="text-[11px] text-muted-foreground/70">
                    {item.sessionTitle}
                  </div>
                  <div className="truncate font-mono text-[10px] text-muted-foreground/45">
                    {item.contentRef}
                  </div>
                </div>
                <div className="shrink-0 text-[10px] text-muted-foreground/45">
                  {new Date(item.createdAt).toLocaleString()}
                </div>
              </div>
            </button>
          ))}
          {selectedArtifactId ? (
            <div className="rounded-xl border border-border/40 bg-background/35 px-3 py-3">
              <div className="text-[11px] uppercase tracking-[0.16em] text-muted-foreground/55">
                Artifact detail
              </div>
              {detailLoading ? (
                <div className="mt-2 text-xs text-muted-foreground/70">
                  Loading artifact preview…
                </div>
              ) : detailError ? (
                <div className="mt-2 rounded-lg border border-destructive/35 bg-destructive/8 px-3 py-2 text-xs text-destructive">
                  {detailError}
                </div>
              ) : detail ? (
                <div className="mt-2 space-y-2">
                  <div className="text-sm font-medium text-foreground">
                    {detail.headline}
                  </div>
                  <div className="whitespace-pre-wrap break-words rounded-lg border border-border/45 bg-background/65 px-3 py-2 font-mono text-[11px] leading-relaxed text-muted-foreground">
                    {detail.body}
                  </div>
                  <div className="text-[11px] text-muted-foreground/65">
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
