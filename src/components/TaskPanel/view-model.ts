import type {
  ArtifactInfo,
  ProviderHistoryInfo,
  ReviewStatus,
  SessionInfo,
  TaskInfo,
} from "@/stores/task-store/types";

export type BadgeTone = "warning" | "progress" | "neutral";

export interface ReviewBadge {
  label: string;
  tone: BadgeTone;
}

export interface SessionTreeRow {
  sessionId: string;
  depth: number;
  session: SessionInfo;
  reviewBadge: ReviewBadge | null;
}

export type HistoryAction =
  | "resume_current"
  | "resume_existing"
  | "attach_lead"
  | "attach_coder";

export interface HistoryItem {
  provider: ProviderHistoryInfo["provider"];
  externalId: string;
  title: string;
  preview: string | null;
  cwd: string | null;
  updatedAt: number;
  normalizedSessionId: string | null;
  normalizedTaskId: string | null;
  actions: HistoryAction[];
}

export interface HistoryPickerModel {
  attached: HistoryItem[];
  elsewhere: HistoryItem[];
  available: HistoryItem[];
}

export interface ArtifactTimelineItem extends ArtifactInfo {
  sessionTitle: string;
}

export function getReviewBadge(
  reviewStatus: ReviewStatus | null | undefined,
): ReviewBadge | null {
  switch (reviewStatus) {
    case "pending_lead_review":
      return { label: "Pending Review", tone: "warning" };
    case "in_review":
      return { label: "In Review", tone: "progress" };
    case "pending_lead_approval":
      return { label: "Pending Approval", tone: "warning" };
    default:
      return null;
  }
}

export function getTaskPanelEmptyStateMessage(): string {
  return "No active task. Create or select a task to inspect task progress, review status, and artifacts.";
}

export function buildSessionTreeRows(
  sessions: SessionInfo[],
  task?: TaskInfo | null,
): SessionTreeRow[] {
  const children = new Map<string | null, SessionInfo[]>();
  for (const session of sessions) {
    const key = session.parentSessionId ?? null;
    const next = children.get(key) ?? [];
    next.push(session);
    children.set(key, next);
  }

  for (const items of children.values()) {
    items.sort((a, b) => {
      if (a.role !== b.role) return a.role === "lead" ? -1 : 1;
      return b.updatedAt - a.updatedAt;
    });
  }

  const rows: SessionTreeRow[] = [];
  const visit = (parentId: string | null, depth: number) => {
    for (const session of children.get(parentId) ?? []) {
      const reviewBadge =
        task?.currentCoderSessionId === session.sessionId &&
        session.role === "coder"
          ? getReviewBadge(task.reviewStatus)
          : null;
      rows.push({ sessionId: session.sessionId, depth, session, reviewBadge });
      visit(session.sessionId, depth + 1);
    }
  };

  visit(null, 0);
  return rows;
}

export function buildHistoryPickerModel(
  task: TaskInfo,
  sessions: SessionInfo[],
  providerHistory: ProviderHistoryInfo[],
): HistoryPickerModel {
  const attachedExternalIds = new Set(
    sessions
      .map((session) =>
        session.externalSessionId
          ? `${session.provider}:${session.externalSessionId}`
          : null,
      )
      .filter((value): value is string => Boolean(value)),
  );

  const items = providerHistory
    .map<HistoryItem>((entry) => {
      const key = `${entry.provider}:${entry.externalId}`;
      const attachedToCurrentTask =
        attachedExternalIds.has(key) || entry.normalizedTaskId === task.taskId;
      const mappedElsewhere =
        !attachedToCurrentTask &&
        Boolean(entry.normalizedSessionId) &&
        Boolean(entry.normalizedTaskId);
      const actions: HistoryAction[] = attachedToCurrentTask
        ? ["resume_current"]
        : mappedElsewhere
          ? ["resume_existing"]
          : ["attach_lead", "attach_coder"];

      return {
        provider: entry.provider,
        externalId: entry.externalId,
        title: entry.title?.trim() || `${entry.provider} session`,
        preview: entry.preview?.trim() || null,
        cwd: entry.cwd?.trim() || null,
        updatedAt: entry.updatedAt,
        normalizedSessionId: entry.normalizedSessionId ?? null,
        normalizedTaskId: entry.normalizedTaskId ?? null,
        actions,
      };
    })
    .sort((a, b) => b.updatedAt - a.updatedAt);

  return {
    attached: items.filter((item) => item.actions[0] === "resume_current"),
    elsewhere: items.filter((item) => item.actions[0] === "resume_existing"),
    available: items.filter((item) => item.actions[0] === "attach_lead"),
  };
}

export function buildArtifactTimeline(
  artifacts: ArtifactInfo[],
  sessions: SessionInfo[],
): ArtifactTimelineItem[] {
  const sessionTitles = new Map(
    sessions.map((session) => [session.sessionId, session.title]),
  );
  return [...artifacts]
    .sort((a, b) => b.createdAt - a.createdAt)
    .map((artifact) => ({
      ...artifact,
      sessionTitle: sessionTitles.get(artifact.sessionId) ?? "Unknown session",
    }));
}
