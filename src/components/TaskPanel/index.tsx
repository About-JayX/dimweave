import { useCallback, useMemo } from "react";
import {
  useTaskStore,
} from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskArtifacts,
  selectActiveTaskSessions,
} from "@/stores/task-store/selectors";
import { ArtifactTimeline } from "./ArtifactTimeline";
import { SessionTree } from "./SessionTree";
import { TaskHeader } from "./TaskHeader";
import {
  buildArtifactTimeline,
  buildSessionTreeRows,
  getTaskPanelEmptyStateMessage,
  getReviewBadge,
} from "./view-model";

export function TaskPanel() {
  const task = useTaskStore(selectActiveTask);
  const taskSessions = useTaskStore(selectActiveTaskSessions);
  const taskArtifacts = useTaskStore(selectActiveTaskArtifacts);
  const resumeSession = useTaskStore((s) => s.resumeSession);

  const reviewBadge = useMemo(
    () => getReviewBadge(task?.reviewStatus),
    [task?.reviewStatus],
  );
  const sessionRows = useMemo(
    () => buildSessionTreeRows(taskSessions, task),
    [task, taskSessions],
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

  if (!task) {
    return (
      <div className="border-b border-border/30 bg-card/30 px-4 py-3">
        <div className="rounded-xl border border-dashed border-border/40 bg-background/20 px-4 py-3 text-xs text-muted-foreground/65">
          {getTaskPanelEmptyStateMessage()}
        </div>
      </div>
    );
  }

  return (
    <div className="border-b border-border/30 bg-linear-to-b from-background/95 to-background/70">
      <div className="max-h-[40vh] space-y-3 overflow-y-auto px-4 py-3">
        <TaskHeader task={task} reviewBadge={reviewBadge} />
        <div className="space-y-3 min-w-0">
          <SessionTree rows={sessionRows} onResume={handleResume} />
          <ArtifactTimeline items={artifactTimeline} />
        </div>
      </div>
    </div>
  );
}
