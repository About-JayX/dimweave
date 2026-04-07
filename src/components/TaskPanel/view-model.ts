import type {
  ArtifactInfo,
  ProviderHistoryInfo,
  SessionInfo,
  TaskInfo,
} from "@/stores/task-store/types";
export {
  buildArtifactDetailModel,
  type ArtifactDetailModel,
  type ArtifactDetailPayload,
} from "./artifact-detail";

export interface SessionTreeRow {
  sessionId: string;
  depth: number;
  session: SessionInfo;
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

export function getTaskPanelEmptyStateMessage(): string {
  return "No active task. Create or select a task to track progress and artifacts.";
}

export function buildSessionTreeRows(
  sessions: SessionInfo[],
  task?: TaskInfo | null,
): SessionTreeRow[] {
  const currentRootIds = task
    ? [task.leadSessionId ?? null, task.currentCoderSessionId ?? null].filter(
        (value, index, items): value is string =>
          Boolean(value) && items.indexOf(value) === index,
      )
    : [];
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
  const sessionsById = new Map(
    sessions.map((session) => [session.sessionId, session]),
  );
  const visited = new Set<string>();

  const visit = (
    sessionId: string,
    depth: number,
    childFilter?: (c: SessionInfo) => boolean,
  ) => {
    if (visited.has(sessionId)) return;
    const session = sessionsById.get(sessionId);
    if (!session) return;
    visited.add(sessionId);
    rows.push({ sessionId: session.sessionId, depth, session });
    for (const child of children.get(session.sessionId) ?? []) {
      if (childFilter && !childFilter(child)) continue;
      visit(child.sessionId, depth + 1, childFilter);
    }
  };

  if (task) {
    // In task context: only traverse children that are still active or
    // explicitly pinned as a task pointer (so paused coder after disconnect
    // does not remain visible when currentCoderSessionId is cleared).
    const pinnedIds = new Set(currentRootIds);
    const activeOrPinned = (c: SessionInfo) =>
      c.status === "active" || pinnedIds.has(c.sessionId);
    for (const rootId of currentRootIds) {
      visit(rootId, 0, activeOrPinned);
    }
    return rows;
  }

  for (const session of children.get(null) ?? []) {
    visit(session.sessionId, 0);
  }
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
