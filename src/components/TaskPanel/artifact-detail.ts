export interface ArtifactDetailTarget {
  title: string;
  contentRef: string;
}

export interface ArtifactDetailPayload {
  reference: string;
  fileName: string | null;
  exists: boolean;
  preview: string | null;
  truncated: boolean;
}

export interface ArtifactDetailModel {
  headline: string;
  body: string;
  meta: string;
  previewAvailable: boolean;
}

export function buildArtifactDetailModel(
  artifact: ArtifactDetailTarget | null,
  detail: ArtifactDetailPayload | null,
): ArtifactDetailModel | null {
  if (!artifact) {
    return null;
  }

  if (detail?.preview) {
    return {
      headline: detail.fileName ?? artifact.title,
      body: detail.preview,
      meta: detail.truncated
        ? `Preview truncated · ${detail.reference}`
        : detail.reference,
      previewAvailable: true,
    };
  }

  if (detail?.exists) {
    return {
      headline: detail.fileName ?? artifact.title,
      body: "Preview unavailable for this local artifact.",
      meta: detail.reference,
      previewAvailable: false,
    };
  }

  return {
    headline: artifact.title,
    body: "Preview unavailable for this artifact reference.",
    meta: artifact.contentRef,
    previewAvailable: false,
  };
}
