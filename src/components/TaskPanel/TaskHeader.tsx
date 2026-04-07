import type { TaskInfo } from "@/stores/task-store/types";

const STATUS_LABELS: Record<string, string> = {
  draft: "Draft",
  planning: "Planning",
  implementing: "Implementing",
  reviewing: "Reviewing",
  done: "Done",
  error: "Error",
};

export function TaskHeader({
  task,
}: {
  task: TaskInfo;
}) {
  return (
    <div className="space-y-2 rounded-xl border border-border/35 bg-background/35 px-4 py-3">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
            Active Task
          </div>
          <div className="truncate text-sm font-semibold text-foreground">
            {task.title}
          </div>
          <div className="truncate text-[11px] text-muted-foreground/70">
            {task.workspaceRoot}
          </div>
        </div>
        <div className="shrink-0 rounded-full border border-border/50 px-2 py-0.5 text-[10px] text-muted-foreground">
          {STATUS_LABELS[task.status] ?? task.status}
        </div>
      </div>
    </div>
  );
}
