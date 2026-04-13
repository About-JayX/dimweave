import { invoke } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import type { ArtifactDetailPayload, ArtifactTimelineItem } from "./view-model";
import { buildArtifactDetailModel } from "./view-model";

export function useArtifactDetail(artifactTimeline: ArtifactTimelineItem[]) {
  const [selectedArtifactId, setSelectedArtifactId] = useState<string | null>(
    null,
  );
  const [artifactDetail, setArtifactDetail] =
    useState<ArtifactDetailPayload | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (artifactTimeline.length === 0) {
      setSelectedArtifactId(null);
      setArtifactDetail(null);
      setError(null);
      setLoading(false);
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
  const contentRef = selectedArtifact?.contentRef ?? null;

  useEffect(() => {
    let cancelled = false;
    if (!contentRef) {
      setArtifactDetail(null);
      setError(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    void invoke<ArtifactDetailPayload>("daemon_get_artifact_detail", {
      contentRef,
    })
      .then((detail) => {
        if (cancelled) return;
        setArtifactDetail(detail);
        setLoading(false);
      })
      .catch((err) => {
        if (cancelled) return;
        setArtifactDetail(null);
        setError(err instanceof Error ? err.message : String(err));
        setLoading(false);
      });

    return () => {
      cancelled = true;
    };
  }, [contentRef]);

  const model = useMemo(
    () => buildArtifactDetailModel(selectedArtifact, artifactDetail),
    [artifactDetail, selectedArtifact],
  );

  return {
    selectedArtifactId,
    setSelectedArtifactId,
    detail: model,
    detailLoading: loading,
    detailError: error,
  };
}
