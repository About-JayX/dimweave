import { useEffect, useState } from "react";
import type { FeishuProjectInboxItem } from "@/stores/feishu-project-store";
import {
  ActionMenu,
  type ActionMenuItem,
} from "@/components/AgentStatus/ActionMenu";

interface IssueListProps {
  items: FeishuProjectInboxItem[];
  loading?: boolean;
  loadingMore?: boolean;
  hasMore?: boolean;
  onLoadMore?: () => void;
  onIgnore: (workItemId: string, ignored: boolean) => void;
  onStartHandling: (workItemId: string) => void;
}

function Skeleton() {
  return (
    <div className="space-y-1">
      {[1, 2, 3].map((i) => (
        <div
          key={i}
          className="animate-pulse rounded-lg border border-border/40 bg-card/45 px-3 py-2"
        >
          <div className="h-3 w-16 rounded bg-muted-foreground/20" />
          <div className="mt-1.5 h-3.5 w-3/4 rounded bg-muted-foreground/20" />
          <div className="mt-1 h-2.5 w-1/3 rounded bg-muted-foreground/15" />
        </div>
      ))}
    </div>
  );
}

function LoadingSpinner() {
  return (
    <div className="flex items-center justify-center py-3">
      <svg
        className="size-4 animate-spin text-muted-foreground"
        viewBox="0 0 24 24"
        fill="none"
      >
        <circle
          cx="12"
          cy="12"
          r="10"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeDasharray="50"
          strokeLinecap="round"
        />
      </svg>
    </div>
  );
}

export function IssueList({
  items,
  loading,
  loadingMore,
  hasMore,
  onLoadMore,
  onIgnore,
  onStartHandling,
}: IssueListProps) {
  const [sentinelNode, setSentinelNode] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!hasMore || !onLoadMore || loadingMore) return;
    if (!sentinelNode) return;

    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) {
          onLoadMore();
        }
      },
      { threshold: 0 },
    );
    observer.observe(sentinelNode);
    return () => observer.disconnect();
  }, [hasMore, onLoadMore, loadingMore, sentinelNode]);

  if (loading) {
    return <Skeleton />;
  }

  if (items.length === 0) {
    return (
      <p className="py-4 text-center text-[11px] text-muted-foreground/60">
        No items in inbox.
      </p>
    );
  }

  return (
    <ul className="space-y-1">
      {items.map((item) => {
        const menu: ActionMenuItem[] = [];
        if (!item.ignored) {
          menu.push({
            label: item.linkedTaskId ? "Open task" : "Handle",
            onClick: () => onStartHandling(item.workItemId),
          });
        }
        menu.push({
          label: item.ignored ? "Restore" : "Ignore",
          danger: !item.ignored,
          onClick: () => onIgnore(item.workItemId, !item.ignored),
        });

        return (
          <li
            key={item.recordId}
            className={`rounded-lg border px-3 py-2 transition-colors ${
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
                  {item.linkedTaskId && (
                    <span className="text-primary/70">Linked</span>
                  )}
                </div>
              </div>
              <ActionMenu items={menu} />
            </div>
          </li>
        );
      })}
      {hasMore && (
        <li>
          {loadingMore ? (
            <LoadingSpinner />
          ) : (
            <div ref={setSentinelNode} className="h-1" />
          )}
        </li>
      )}
    </ul>
  );
}
