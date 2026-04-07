import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";
import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveTaskArtifacts,
  selectActiveTaskSessions,
} from "@/stores/task-store/selectors";
import { ArtifactTimeline } from "./ArtifactTimeline";
import { SessionTree } from "./SessionTree";
import { TaskHeader, type ReviewBadge } from "./TaskHeader";
import {
  buildArtifactDetailModel,
  buildArtifactTimeline,
  buildSessionTreeRows,
  getTaskPanelEmptyStateMessage,
  type ArtifactDetailPayload,
} from "./view-model";

export function TaskPanel() {
  const task = useTaskStore(selectActiveTask);
  const taskSessions = useTaskStore(selectActiveTaskSessions);
  const taskArtifacts = useTaskStore(selectActiveTaskArtifacts);
  const resumeSession = useTaskStore((s) => s.resumeSession);
  const sessionRows = useMemo(
    () => buildSessionTreeRows(taskSessions, task),
    [task, taskSessions],
  );
  const artifactTimeline = useMemo(
    () => buildArtifactTimeline(taskArtifacts, taskSessions),
    [taskArtifacts, taskSessions],
  );
  const [selectedArtifactId, setSelectedArtifactId] = useState<string | null>(
    null,
  );
  const [artifactDetail, setArtifactDetail] =
    useState<ArtifactDetailPayload | null>(null);
  const [artifactDetailLoading, setArtifactDetailLoading] = useState(false);
  const [artifactDetailError, setArtifactDetailError] = useState<string | null>(
    null,
  );

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
        : (artifactTimeline[0]?.artifactId ?? null),
    );
  }, [artifactTimeline]);

  const selectedArtifact = useMemo(
    () =>
      selectedArtifactId
        ? (artifactTimeline.find(
            (item) => item.artifactId === selectedArtifactId,
          ) ?? null)
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

  const reviewBadge: ReviewBadge | null =
    task?.status === "reviewing" ? { label: "Review", tone: "warning" } : null;

  if (!task) {
    return (
      <div className="rounded-xl border border-dashed border-border/40 bg-background/20 px-4 py-3 text-xs text-muted-foreground/65">
        {getTaskPanelEmptyStateMessage()}
      </div>
    );
  }

  return (
    <div className="space-y-3">
      <TaskHeader task={task} reviewBadge={reviewBadge} />
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
  );
}
