import { useCallback, useEffect, useMemo } from "react";
import { useTaskStore } from "@/stores/task-store";
import type { SessionRole } from "@/stores/task-store/types";
import { ArtifactTimeline } from "./ArtifactTimeline";
import { HistoryPicker } from "./HistoryPicker";
import { SessionTree } from "./SessionTree";
import { TaskHeader } from "./TaskHeader";
import {
  buildArtifactTimeline,
  buildHistoryPickerModel,
  buildSessionTreeRows,
  getReviewBadge,
  type HistoryItem,
} from "./view-model";

export function TaskPanel() {
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const tasks = useTaskStore((s) => s.tasks);
  const sessions = useTaskStore((s) => s.sessions);
  const artifacts = useTaskStore((s) => s.artifacts);
  const providerHistory = useTaskStore((s) => s.providerHistory);
  const resumeSession = useTaskStore((s) => s.resumeSession);
  const fetchProviderHistory = useTaskStore((s) => s.fetchProviderHistory);
  const attachProviderHistory = useTaskStore((s) => s.attachProviderHistory);

  const task = activeTaskId ? tasks[activeTaskId] : null;
  const taskSessions = activeTaskId ? sessions[activeTaskId] ?? [] : [];
  const taskArtifacts = activeTaskId ? artifacts[activeTaskId] ?? [] : [];
  const workspaceHistory =
    task?.workspaceRoot ? providerHistory[task.workspaceRoot] ?? [] : [];

  useEffect(() => {
    if (!task?.workspaceRoot) return;
    void fetchProviderHistory(task.workspaceRoot);
  }, [fetchProviderHistory, task?.workspaceRoot]);

  const reviewBadge = useMemo(
    () => getReviewBadge(task?.reviewStatus),
    [task?.reviewStatus],
  );
  const sessionRows = useMemo(
    () => buildSessionTreeRows(taskSessions, task),
    [task, taskSessions],
  );
  const historyModel = useMemo(
    () =>
      task
        ? buildHistoryPickerModel(task, taskSessions, workspaceHistory)
        : { attached: [], elsewhere: [], available: [] },
    [task, taskSessions, workspaceHistory],
  );
  const artifactTimeline = useMemo(
    () => buildArtifactTimeline(taskArtifacts, taskSessions),
    [taskArtifacts, taskSessions],
  );

  const handleResume = useCallback(
    (sessionId: string) => {
      void resumeSession(sessionId);
    },
    [resumeSession],
  );

  const handleAttach = useCallback(
    (item: HistoryItem, role: SessionRole) => {
      if (!task) return;
      void attachProviderHistory(
        item.provider,
        item.externalId,
        item.cwd ?? task.workspaceRoot,
        role,
      );
    },
    [attachProviderHistory, task],
  );

  if (!task) {
    return (
      <div className="border-b border-border/30 bg-card/30 px-4 py-3">
        <div className="rounded-xl border border-dashed border-border/40 bg-background/20 px-4 py-3 text-xs text-muted-foreground/65">
          No active task. Create or select a task to inspect session history,
          review status, and artifacts.
        </div>
      </div>
    );
  }

  return (
    <div className="border-b border-border/30 bg-linear-to-b from-background/95 to-background/70">
      <div className="max-h-[40vh] space-y-3 overflow-y-auto px-4 py-3">
        <TaskHeader task={task} reviewBadge={reviewBadge} />
        <div className="grid gap-3 xl:grid-cols-[minmax(0,1.05fr)_minmax(0,0.95fr)]">
          <div className="space-y-3 min-w-0">
            <SessionTree rows={sessionRows} onResume={handleResume} />
            <ArtifactTimeline items={artifactTimeline} />
          </div>
          <div className="min-w-0">
            <HistoryPicker
              model={historyModel}
              onResume={handleResume}
              onAttach={handleAttach}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
