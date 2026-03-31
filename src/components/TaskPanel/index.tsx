import { useTaskStore } from "@/stores/task-store";

const STATUS_LABELS: Record<string, string> = {
  draft: "Draft",
  planning: "Planning",
  implementing: "Implementing",
  reviewing: "Reviewing",
  done: "Done",
  error: "Error",
};

const REVIEW_LABELS: Record<string, string> = {
  pending_lead_review: "Pending Review",
  in_review: "In Review",
  pending_lead_approval: "Pending Approval",
};

export function TaskPanel() {
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const sessions = useTaskStore((s) => s.sessions);

  const task = activeTaskId ? tasks[activeTaskId] : null;
  const taskSessions = activeTaskId ? (sessions[activeTaskId] ?? []) : [];

  if (!task) {
    return (
      <div className="px-3 py-2 text-xs text-muted-foreground/60 border-b border-border/30">
        No active task
      </div>
    );
  }

  return (
    <div className="px-3 py-2 border-b border-border/30 text-xs space-y-1">
      <div className="flex items-center gap-2">
        <span className="font-medium text-foreground truncate">
          {task.title}
        </span>
        <span className="text-muted-foreground/70 shrink-0">
          {STATUS_LABELS[task.status] ?? task.status}
        </span>
      </div>
      {task.reviewStatus && (
        <div className="text-amber-400/80">
          {REVIEW_LABELS[task.reviewStatus] ?? task.reviewStatus}
        </div>
      )}
      {taskSessions.length > 0 && (
        <div className="text-muted-foreground/50">
          {taskSessions.length} session{taskSessions.length > 1 ? "s" : ""}
        </div>
      )}
    </div>
  );
}
