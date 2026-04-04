import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";
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
  buildArtifactDetailModel,
  buildArtifactTimeline,
  buildSessionTreeRows,
  getTaskPanelEmptyStateMessage,
  getReviewBadge,
  type ArtifactDetailPayload,
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
  const [selectedArtifactId, setSelectedArtifactId] = useState<string | null>(null);
  const [artifactDetail, setArtifactDetail] = useState<ArtifactDetailPayload | null>(null);
  const [artifactDetailLoading, setArtifactDetailLoading] = useState(false);
  const [artifactDetailError, setArtifactDetailError] = useState<string | null>(null);

  const handleResume = useCallback(
    (sessionId: string) => {
      void resumeSession(sessionId);
    },
    [resumeSession],
  );

  useEffect(() => {
    if (artifactTimeline.length === 0) {
      setSelectedArtifactId(null);
      setArtifactDetail(null);
      setArtifactDetailError(null);
      setArtifactDetailLoading(false);
      return;
    }
    setSelectedArtifactId((current) =>
      current && artifactTimeline.some((item) => item.artifactId === current)
        ? current
        : artifactTimeline[0]?.artifactId ?? null,
    );
  }, [artifactTimeline]);

  const selectedArtifact = useMemo(
    () =>
      selectedArtifactId
        ? artifactTimeline.find((item) => item.artifactId === selectedArtifactId) ?? null
        : null,
    [artifactTimeline, selectedArtifactId],
  );
  const selectedArtifactContentRef = selectedArtifact?.contentRef ?? null;

  useEffect(() => {
    let cancelled = false;
    if (!selectedArtifactContentRef) {
      setArtifactDetail(null);
      setArtifactDetailError(null);
      setArtifactDetailLoading(false);
      return;
    }

    setArtifactDetailLoading(true);
    setArtifactDetailError(null);
    void invoke<ArtifactDetailPayload>("daemon_get_artifact_detail", {
      contentRef: selectedArtifactContentRef,
    })
      .then((detail) => {
        if (cancelled) return;
        setArtifactDetail(detail);
        setArtifactDetailLoading(false);
      })
      .catch((error) => {
        if (cancelled) return;
        setArtifactDetail(null);
        setArtifactDetailError(
          error instanceof Error ? error.message : String(error),
        );
        setArtifactDetailLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [selectedArtifactContentRef]);

  const artifactDetailModel = useMemo(
    () => buildArtifactDetailModel(selectedArtifact, artifactDetail),
    [artifactDetail, selectedArtifact],
  );

  if (!task) {
    return (
      <section className="rounded-2xl border border-border/40 bg-card/45 px-4 py-4">
        <div className="mb-3 text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
          Task context
        </div>
        <div className="rounded-xl border border-dashed border-border/40 bg-background/20 px-4 py-3 text-xs text-muted-foreground/65">
          {getTaskPanelEmptyStateMessage()}
        </div>
      </section>
    );
  }

  return (
    <section className="space-y-3">
      <div className="rounded-2xl border border-border/40 bg-card/55 px-4 py-4">
        <div className="mb-3 flex items-center justify-between gap-3">
          <div>
            <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
              Task context
            </div>
            <div className="mt-0.5 text-sm font-semibold text-foreground">
              Session context
            </div>
          </div>
          <div className="text-[11px] text-muted-foreground/65">
            {taskSessions.length} sessions · {taskArtifacts.length} artifacts
          </div>
        </div>
        <TaskHeader task={task} reviewBadge={reviewBadge} />
        <div className="mt-3 grid grid-cols-2 gap-2 text-[11px]">
          <div className="rounded-xl border border-border/35 bg-background/30 px-3 py-2">
            <div className="text-[10px] uppercase tracking-[0.16em] text-muted-foreground/55">
              Active sessions
            </div>
            <div className="mt-1 text-lg font-semibold text-foreground">
              {taskSessions.length}
            </div>
          </div>
          <div className="rounded-xl border border-border/35 bg-background/30 px-3 py-2">
            <div className="text-[10px] uppercase tracking-[0.16em] text-muted-foreground/55">
              Artifacts
            </div>
            <div className="mt-1 text-lg font-semibold text-foreground">
              {taskArtifacts.length}
            </div>
          </div>
        </div>
      </div>

      <div className="min-w-0 space-y-3">
        <div className="rounded-2xl border border-border/40 bg-card/45 p-0">
          <SessionTree rows={sessionRows} onResume={handleResume} />
        </div>
        <div className="rounded-2xl border border-border/40 bg-card/45 p-0">
          <ArtifactTimeline
            items={artifactTimeline}
            selectedArtifactId={selectedArtifactId}
            detail={artifactDetailModel}
            detailLoading={artifactDetailLoading}
            detailError={artifactDetailError}
            onSelect={setSelectedArtifactId}
          />
        </div>
      </div>
    </section>
  );
}
