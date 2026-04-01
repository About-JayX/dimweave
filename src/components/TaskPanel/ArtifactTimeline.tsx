import type { ArtifactTimelineItem } from "./view-model";

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
}: {
  items: ArtifactTimelineItem[];
}) {
  return (
    <section className="rounded-xl border border-border/40 bg-card/50 px-4 py-3 backdrop-blur-sm">
      <div className="mb-3 flex items-center justify-between gap-3">
        <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
          Artifact Timeline
        </div>
        <div className="text-[10px] text-muted-foreground/50">
          {items.length} items
        </div>
      </div>
      {items.length === 0 ? (
        <div className="text-xs text-muted-foreground/60">
          No task artifacts captured yet.
        </div>
      ) : (
        <div className="space-y-2">
          {items.map((item) => (
            <div
              key={item.artifactId}
              className="rounded-lg border border-border/35 bg-background/30 px-3 py-2"
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
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
