import { Settings2, Trash2, ShieldAlert } from "lucide-react";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import type { TaskAgentInfo, TaskInfo } from "@/stores/task-store/types";

const NO_AGENTS: TaskAgentInfo[] = [];

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
  draft: "border-zinc-500/40 bg-zinc-500/8 text-zinc-500",
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
      <span
        className="inline-flex items-center gap-1 text-[11px] text-rose-400"
        title={lastSave.error ?? "Save failed"}
      >
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

function StatusChip({ status }: { status: string }) {
  return (
    <span
      data-task-status="true"
      className={`rounded-full border px-2 py-0.5 text-[10px] font-medium ${STATUS_STYLES[status] ?? "border-zinc-500/40 bg-zinc-500/8 text-zinc-500"}`}
    >
      {STATUS_LABELS[status] ?? status}
    </span>
  );
}

export function TaskHeader({
  task,
  reviewBadge,
  onEditTask,
  onDeleteTask,
  collapsed,
  onClick,
}: {
  task: TaskInfo;
  reviewBadge?: ReviewBadge | null;
  onEditTask?: () => void;
  onDeleteTask?: () => void;
  collapsed?: boolean;
  onClick?: () => void;
}) {
  const agents = useTaskStore((s) => s.taskAgents[task.taskId] ?? NO_AGENTS);
  const agentStatuses = useTaskStore(
    (s) => s.agentRuntimeStatuses[task.taskId],
  );
  const pendingPromptCount = useBridgeStore((s) => {
    let n = 0;
    for (const p of s.permissionPrompts) {
      if (p.taskId === task.taskId) n += 1;
    }
    return n;
  });
  const showDetail = !collapsed;
  return (
    <div
      className={`rounded-xl border border-border/50 bg-card/50 px-4 py-3${onClick ? " cursor-pointer transition-colors hover:bg-card/70" : ""}`}
      data-collapsed={collapsed ? "true" : undefined}
      onClick={onClick}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
      onKeyDown={
        onClick
          ? (e) => {
              if (e.key === "Enter" || e.key === " ") onClick();
            }
          : undefined
      }
    >
      {/* Main row: task info left, edit icon (non-collapsed) or status (collapsed) right */}
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1 space-y-1">
          <div className="flex items-center gap-1.5">
            <span className="truncate text-sm font-semibold text-foreground">
              {task.title}
            </span>
            {pendingPromptCount > 0 && (
              <span
                className="inline-flex items-center gap-0.5 rounded-full border border-amber-500/40 bg-amber-500/15 px-1.5 py-px text-[10px] font-semibold text-amber-500"
                title={`${pendingPromptCount} pending approval${pendingPromptCount > 1 ? "s" : ""} — open this task to resolve`}
              >
                <ShieldAlert className="size-3" />
                {pendingPromptCount}
              </span>
            )}
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
                const provStyle =
                  agent.provider === "claude"
                    ? "border-claude/30 bg-claude/8 text-claude/80"
                    : "border-codex/30 bg-codex/8 text-codex/80";
                const isOnline =
                  agentStatuses?.find((s) => s.agentId === agent.agentId)
                    ?.online ?? false;
                return (
                  <span
                    key={agent.agentId}
                    className={`inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] ${provStyle}`}
                  >
                    <span
                      data-agent-online={isOnline ? "true" : "false"}
                      className={`inline-block h-1.5 w-1.5 rounded-full ${isOnline ? "bg-emerald-400" : "bg-zinc-500"}`}
                    />
                    {agent.displayName ?? agent.role}: {agent.provider}
                  </span>
                );
              })}
            </div>
          )}
        </div>
        {/* Upper-right: icon-only edit button (non-collapsed) or compact status (collapsed) */}
        {showDetail ? (
          onEditTask || onDeleteTask ? (
            <div className="flex items-center gap-0.5 shrink-0">
              {onDeleteTask && (
                <button
                  type="button"
                  data-delete-icon="true"
                  onClick={(e) => {
                    e.stopPropagation();
                    onDeleteTask();
                  }}
                  className="rounded-lg p-1.5 text-muted-foreground/50 transition-colors hover:bg-rose-500/20 hover:text-rose-400 active:opacity-70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  title="Delete task"
                  aria-label="Delete task"
                >
                  <Trash2 className="size-3.5" />
                </button>
              )}
              {onEditTask && (
                <button
                  type="button"
                  data-edit-icon="true"
                  onClick={(e) => {
                    e.stopPropagation();
                    onEditTask();
                  }}
                  className="shrink-0 rounded-lg p-1.5 text-muted-foreground/50 transition-colors hover:bg-muted hover:text-foreground active:opacity-70 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  title="Edit task"
                  aria-label="Edit task"
                >
                  <Settings2 className="size-3.5" />
                </button>
              )}
            </div>
          ) : null
        ) : (
          <StatusChip status={task.status} />
        )}
      </div>
      {/* Footer row: review badge + compact status chip anchored lower-right (non-collapsed) */}
      {showDetail && (
        <div className="mt-2 flex items-center justify-end gap-2">
          {reviewBadge && (
            <span
              className={`inline-flex rounded-full border px-2 py-0.5 text-[10px] font-medium ${REVIEW_TONE_STYLES[reviewBadge.tone]}`}
            >
              {reviewBadge.label}
            </span>
          )}
          <StatusChip status={task.status} />
        </div>
      )}
    </div>
  );
}
