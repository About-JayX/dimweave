import type { FeishuProjectInboxItem } from "@/stores/feishu-project-store";

interface IssueListProps {
  items: FeishuProjectInboxItem[];
  onIgnore: (workItemId: string, ignored: boolean) => void;
  onStartHandling: (workItemId: string) => void;
}

export function IssueList({
  items,
  onIgnore,
  onStartHandling,
}: IssueListProps) {
  if (items.length === 0) {
    return (
      <p className="py-4 text-center text-[11px] text-muted-foreground/60">
        No items in inbox.
      </p>
    );
  }

  return (
    <ul className="space-y-1">
      {items.map((item) => (
        <li
          key={item.recordId}
          className={`group rounded-lg border px-3 py-2 transition-colors ${
            item.ignored
              ? "border-border/25 bg-muted/20 opacity-60"
              : "border-border/40 bg-card/45 hover:border-border/60"
          }`}
        >
          <div className="flex items-start gap-2">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-1.5">
                <span className="rounded bg-muted/60 px-1 py-px text-[9px] font-medium uppercase text-muted-foreground">
                  {item.workItemTypeKey}
                </span>
                {item.statusLabel && (
                  <span className="text-[9px] text-muted-foreground">
                    {item.statusLabel}
                  </span>
                )}
              </div>
              <a
                href={item.sourceUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="mt-0.5 block text-[12px] font-medium text-card-foreground hover:text-primary hover:underline"
              >
                {item.title || `#${item.workItemId}`}
              </a>
              <div className="mt-0.5 flex items-center gap-2 text-[9px] text-muted-foreground">
                {item.assigneeLabel && <span>{item.assigneeLabel}</span>}
                <span className="capitalize">{item.lastIngress}</span>
              </div>
            </div>
            <div className="flex shrink-0 gap-1 opacity-0 transition-opacity group-hover:opacity-100">
              <button
                className="rounded border border-border/50 px-1.5 py-0.5 text-[9px] text-foreground hover:border-primary/50 active:bg-primary/10 focus-visible:ring-1 focus-visible:ring-primary/40"
                onClick={() => onIgnore(item.workItemId, !item.ignored)}
              >
                {item.ignored ? "Restore" : "Ignore"}
              </button>
              {!item.ignored && !item.linkedTaskId && (
                <button
                  className="rounded border border-primary/50 px-1.5 py-0.5 text-[9px] text-primary hover:bg-primary/10 active:bg-primary/20 focus-visible:ring-1 focus-visible:ring-primary/40"
                  onClick={() => onStartHandling(item.workItemId)}
                >
                  Handle
                </button>
              )}
            </div>
          </div>
        </li>
      ))}
    </ul>
  );
}
