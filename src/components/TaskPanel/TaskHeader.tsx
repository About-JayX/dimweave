import { Settings2 } from "lucide-react";
import { useTaskStore } from "@/stores/task-store";
import { selectActiveTaskAgents } from "@/stores/task-store/selectors";
import type { TaskInfo } from "@/stores/task-store/types";

export type ReviewBadge = { label: string; tone: "warning" | "progress" };

const REVIEW_TONE_STYLES: Record<ReviewBadge["tone"], string> = {
  warning: "border-amber-500/50 bg-amber-500/15 text-amber-300",
  progress: "border-sky-500/50 bg-sky-500/15 text-sky-300",
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
  draft: "border-zinc-500/50 bg-zinc-500/10 text-zinc-400",
  planning: "border-sky-500/50 bg-sky-500/15 text-sky-300",
  implementing: "border-indigo-500/50 bg-indigo-500/15 text-indigo-300",
  reviewing: "border-amber-500/50 bg-amber-500/15 text-amber-300",
  done: "border-emerald-500/50 bg-emerald-500/15 text-emerald-300",
  error: "border-rose-500/50 bg-rose-500/15 text-rose-300",
};

function SaveIndicator() {
  const lastSave = useTaskStore((s) => s.lastSave);
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  if (!lastSave || lastSave.taskId !== activeTaskId) return null;

  if (!lastSave.success) {
    return (
      <span className="inline-flex items-center gap-1 text-[11px] text-rose-400" title={lastSave.error ?? "Save failed"}>
        <span className="inline-block h-1.5 w-1.5 rounded-full bg-rose-400" />
        Save failed
      </span>
    );
  }

  return (
    <span className="inline-flex items-center gap-1 text-[11px] text-emerald-400/70">
      <span className="inline-block h-1.5 w-1.5 rounded-full bg-emerald-400/70" />
      Saved
    </span>
  );
}

export function TaskHeader({
  task,
  reviewBadge,
  onEditTask,
  collapsed,
  onClick,
}: {
  task: TaskInfo;
  reviewBadge?: ReviewBadge | null;
  onEditTask?: () => void;
  collapsed?: boolean;
  onClick?: () => void;
}) {
  const agents = useTaskStore(selectActiveTaskAgents);
  const showDetail = !collapsed;
  return (
    <div
      className={`rounded-xl border border-border/50 bg-card/50 px-4 py-3${onClick ? " cursor-pointer transition-colors hover:bg-card/70" : ""}`}
      data-collapsed={collapsed ? "true" : undefined}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={onClick ? (e) => { if (e.key === "Enter" || e.key === " ") onClick(); } : undefined}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1 space-y-1">
          <div className="truncate text-sm font-semibold text-foreground">
            {task.title}
          </div>
          {showDetail && (
            <div className="truncate text-xs text-muted-foreground/80">
              {task.workspaceRoot}
            </div>
          )}
          {showDetail && (
            <div className="flex items-center gap-2">
              <span className="font-mono text-[11px] text-muted-foreground/60">
                {task.taskId}
              </span>
              <SaveIndicator />
            </div>
          )}
          {showDetail && (
            <div className="flex flex-wrap items-center gap-1.5">
              {agents.map((agent) => {
                const provStyle = agent.provider === "claude"
                  ? "border-claude/30 bg-claude/8 text-claude/80"
                  : "border-codex/30 bg-codex/8 text-codex/80";
                return (
                  <span key={agent.agentId} className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] ${provStyle}`}>
                    <span className="inline-block h-1.5 w-1.5 rounded-full bg-zinc-500" />
                    {agent.displayName ?? agent.role}: {agent.provider}
                  </span>
                );
              })}
            </div>
          )}
        </div>
        <div className="flex shrink-0 flex-col items-end gap-1.5">
          <div className="flex items-center gap-1.5">
            <div
              className={`rounded-full border px-2.5 py-0.5 text-[11px] font-medium ${STATUS_STYLES[task.status] ?? "border-zinc-500/50 bg-zinc-500/10 text-zinc-400"}`}
            >
              {STATUS_LABELS[task.status] ?? task.status}
            </div>
          </div>
          {showDetail && reviewBadge && (
            <span
              className={`inline-flex rounded-full border px-2.5 py-0.5 text-[11px] font-medium ${REVIEW_TONE_STYLES[reviewBadge.tone]}`}
            >
              {reviewBadge.label}
            </span>
          )}
          {showDetail && onEditTask && (
            <button
              type="button"
              onClick={(e) => { e.stopPropagation(); onEditTask(); }}
              className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              title="Edit task"
            >
              <Settings2 className="size-3" />
              Edit task
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
