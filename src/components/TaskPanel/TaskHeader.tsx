import type { TaskInfo } from "@/stores/task-store/types";

export type ReviewBadge = { label: string; tone: "warning" | "progress" };

const REVIEW_TONE_STYLES: Record<ReviewBadge["tone"], string> = {
  warning: "border-amber-400/30 bg-amber-400/10 text-amber-300",
  progress: "border-sky-400/30 bg-sky-400/10 text-sky-300",
};

const STATUS_LABELS: Record<string, string> = {
  draft: "Draft",
  planning: "Planning",
  implementing: "In progress",
  reviewing: "Review",
  done: "Done",
  error: "Error",
};

const STATUS_STYLES: Record<string, string> = {
  draft: "border-border/50 text-muted-foreground",
  planning: "border-sky-400/30 text-sky-400",
  implementing: "border-indigo-400/30 text-indigo-400",
  reviewing: "border-amber-400/35 bg-amber-400/8 text-amber-300",
  done: "border-emerald-400/30 text-emerald-400",
  error: "border-rose-400/30 text-rose-400",
};

export function TaskHeader({
  task,
  reviewBadge,
}: {
  task: TaskInfo;
  reviewBadge?: ReviewBadge | null;
}) {
  return (
    <div className="rounded-xl border border-border/35 bg-background/35 px-4 py-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1 space-y-0.5">
          <div className="truncate text-sm font-semibold text-foreground">
            {task.title}
          </div>
          <div className="truncate text-[11px] text-muted-foreground/70">
            {task.workspaceRoot}
          </div>
          <div className="font-mono text-[10px] text-muted-foreground/45">
            {task.taskId}
          </div>
        </div>
        <div className="flex shrink-0 flex-col items-end gap-1.5">
          <div
            className={`rounded-full border px-2 py-0.5 text-[10px] ${STATUS_STYLES[task.status] ?? "border-border/50 text-muted-foreground"}`}
          >
            {STATUS_LABELS[task.status] ?? task.status}
          </div>
          {reviewBadge && (
            <span
              className={`inline-flex rounded-full border px-2 py-0.5 text-[10px] font-medium ${REVIEW_TONE_STYLES[reviewBadge.tone]}`}
            >
              {reviewBadge.label}
            </span>
          )}
        </div>
      </div>
    </div>
  );
}
