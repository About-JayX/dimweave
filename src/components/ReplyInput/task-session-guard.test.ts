import { describe, expect, test } from "bun:test";
import { getTaskSessionWarning } from "./task-session-guard";

function makeTask(id: string, workspaceRoot: string) {
  return {
    taskId: id,
    workspaceRoot,
    title: `Task ${id}`,
    status: "draft" as const,
    reviewStatus: null,
    leadSessionId: `${id}-lead`,
    currentCoderSessionId: null,
    createdAt: 100,
    updatedAt: 200,
  };
}

function makeClaudeLeadSession(
  taskId: string,
  externalSessionId: string | null,
  cwd: string,
) {
  return {
    sessionId: `${taskId}-lead`,
    taskId,
    parentSessionId: null,
    provider: "claude" as const,
    role: "lead" as const,
    externalSessionId,
    transcriptPath: null,
    status: "active" as const,
    cwd,
    title: "Lead",
    createdAt: 100,
    updatedAt: 200,
  };
}

describe("getTaskSessionWarning", () => {
  test("returns a reconnect warning when the connected agent belongs to another task session", () => {
    const task = makeTask("task-2", "/repo-b");
    const session = makeClaudeLeadSession("task-2", "claude_current", "/repo-b");
    const warning = getTaskSessionWarning({
      target: "auto",
      activeTask: task,
      sessions: [session],
      claudeRole: "lead",
      codexRole: "coder",
      agents: {
        claude: {
          name: "claude",
          displayName: "Claude Code",
          status: "connected",
          providerSession: {
            provider: "claude",
            externalSessionId: "claude_stale",
            cwd: "/repo-a",
            connectionMode: "resumed",
          },
        },
        codex: {
          name: "codex",
          displayName: "Codex",
          status: "disconnected",
        },
      },
    });

    expect(warning).toBe("Reconnect to this task");
  });

  test("returns a launch warning when the active task does not yet own a provider session", () => {
    const task = makeTask("task-2", "/repo-b");
    const session = makeClaudeLeadSession("task-2", null, "/repo-b");
    const warning = getTaskSessionWarning({
      target: "auto",
      activeTask: task,
      sessions: [session],
      claudeRole: "lead",
      codexRole: "coder",
      agents: {
        claude: {
          name: "claude",
          displayName: "Claude Code",
          status: "connected",
          providerSession: {
            provider: "claude",
            externalSessionId: "claude_stale",
            cwd: "/repo-a",
            connectionMode: "resumed",
          },
        },
        codex: {
          name: "codex",
          displayName: "Codex",
          status: "disconnected",
        },
      },
    });

    expect(warning).toBe("Launch agent for this task");
  });
});
