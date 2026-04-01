import { describe, expect, test } from "bun:test";
import {
  buildArtifactTimeline,
  buildHistoryPickerModel,
  buildSessionTreeRows,
  getReviewBadge,
} from "../src/components/TaskPanel/view-model";
import type {
  ArtifactInfo,
  ProviderHistoryInfo,
  SessionInfo,
  TaskInfo,
} from "../src/stores/task-store/types";

function makeTask(overrides: Partial<TaskInfo> = {}): TaskInfo {
  return {
    taskId: "task_1",
    workspaceRoot: "/ws",
    title: "Ship session history",
    status: "reviewing",
    reviewStatus: "pending_lead_approval",
    leadSessionId: "sess_lead",
    currentCoderSessionId: "sess_coder_b",
    createdAt: 100,
    updatedAt: 300,
    ...overrides,
  };
}

function makeSession(
  sessionId: string,
  role: "lead" | "coder",
  overrides: Partial<SessionInfo> = {},
): SessionInfo {
  return {
    sessionId,
    taskId: "task_1",
    parentSessionId: role === "coder" ? "sess_lead" : null,
    provider: role === "lead" ? "claude" : "codex",
    role,
    externalSessionId: `${sessionId}_external`,
    transcriptPath: null,
    status: role === "lead" ? "active" : "paused",
    cwd: "/ws",
    title: role === "lead" ? "Lead session" : `Coder ${sessionId}`,
    createdAt: role === "lead" ? 100 : 150,
    updatedAt: role === "lead" ? 250 : 200,
    ...overrides,
  };
}

function makeProviderHistory(
  externalId: string,
  provider: "claude" | "codex",
  overrides: Partial<ProviderHistoryInfo> = {},
): ProviderHistoryInfo {
  return {
    provider,
    externalId,
    title: `${provider} ${externalId}`,
    preview: `Preview ${externalId}`,
    cwd: "/ws",
    archived: false,
    createdAt: 100,
    updatedAt: 200,
    status: "paused",
    normalizedSessionId: null,
    normalizedTaskId: null,
    ...overrides,
  };
}

describe("buildSessionTreeRows", () => {
  test("renders lead as root and coder children indented beneath it", () => {
    const rows = buildSessionTreeRows([
      makeSession("sess_coder_a", "coder", { updatedAt: 150 }),
      makeSession("sess_lead", "lead", { updatedAt: 300 }),
      makeSession("sess_coder_b", "coder", { updatedAt: 250 }),
    ]);

    expect(rows.map((row) => row.sessionId)).toEqual([
      "sess_lead",
      "sess_coder_b",
      "sess_coder_a",
    ]);
    expect(rows.map((row) => row.depth)).toEqual([0, 1, 1]);
  });

  test("marks the active coder review gate badge on current coder session", () => {
    const rows = buildSessionTreeRows(
      [makeSession("sess_lead", "lead"), makeSession("sess_coder_b", "coder")],
      makeTask(),
    );

    expect(rows[1]?.reviewBadge?.label).toBe("Pending Approval");
    expect(rows[1]?.reviewBadge?.tone).toBe("warning");
  });
});

describe("buildHistoryPickerModel", () => {
  test("splits attached, elsewhere, and external history entries", () => {
    const task = makeTask();
    const sessions = [
      makeSession("sess_lead", "lead", { externalSessionId: "claude-attached" }),
      makeSession("sess_coder_b", "coder", {
        externalSessionId: "codex-attached",
      }),
    ];
    const providerHistory = [
      makeProviderHistory("claude-attached", "claude", {
        updatedAt: 500,
        normalizedSessionId: "sess_lead",
        normalizedTaskId: "task_1",
      }),
      makeProviderHistory("codex-other-task", "codex", {
        updatedAt: 450,
        normalizedSessionId: "sess_elsewhere",
        normalizedTaskId: "task_2",
      }),
      makeProviderHistory("claude-free", "claude", { updatedAt: 400 }),
      makeProviderHistory("codex-free", "codex", { updatedAt: 350 }),
    ];

    const model = buildHistoryPickerModel(task, sessions, providerHistory);

    expect(model.attached.map((item) => item.externalId)).toEqual([
      "claude-attached",
    ]);
    expect(model.elsewhere.map((item) => item.externalId)).toEqual([
      "codex-other-task",
    ]);
    expect(model.available.map((item) => item.externalId)).toEqual([
      "claude-free",
      "codex-free",
    ]);
    expect(model.available[0]?.actions).toEqual(["attach_lead", "attach_coder"]);
    expect(model.elsewhere[0]?.actions).toEqual(["resume_existing"]);
  });
});

describe("buildArtifactTimeline", () => {
  test("sorts newest artifacts first and resolves session titles", () => {
    const sessions = [
      makeSession("sess_lead", "lead", { title: "Lead planning" }),
      makeSession("sess_coder_b", "coder", { title: "Coder implementation" }),
    ];
    const artifacts: ArtifactInfo[] = [
      {
        artifactId: "art_plan",
        taskId: "task_1",
        sessionId: "sess_lead",
        kind: "plan",
        title: "Execution plan",
        contentRef: "artifact://plan",
        createdAt: 100,
      },
      {
        artifactId: "art_diff",
        taskId: "task_1",
        sessionId: "sess_coder_b",
        kind: "diff",
        title: "Patch v2",
        contentRef: "artifact://diff",
        createdAt: 200,
      },
    ];

    const timeline = buildArtifactTimeline(artifacts, sessions);
    expect(timeline.map((item) => item.artifactId)).toEqual([
      "art_diff",
      "art_plan",
    ]);
    expect(timeline[0]?.sessionTitle).toBe("Coder implementation");
  });
});

describe("getReviewBadge", () => {
  test("maps lead approval state to warning badge", () => {
    expect(getReviewBadge("pending_lead_approval")).toEqual({
      label: "Pending Approval",
      tone: "warning",
    });
  });

  test("returns null for missing review state", () => {
    expect(getReviewBadge(null)).toBeNull();
  });
});
