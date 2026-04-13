import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { selectActiveReplyTarget } from "@/stores/task-store/selectors";
import { useTaskStore } from "@/stores/task-store";
import { getTaskSessionWarning } from "./task-session-guard";

function makeTask(id: string, workspaceRoot: string) {
  return {
    taskId: id,
    workspaceRoot,
    title: `Task ${id}`,
    status: "draft" as const,
    leadSessionId: `${id}-lead`,
    currentCoderSessionId: null,
    createdAt: 100,
    updatedAt: 200,
  };
}

function makeClaudeLeadSession(taskId: string, externalSessionId: string, cwd: string) {
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

function installTauriStub(snapshot: { task: ReturnType<typeof makeTask>; sessions: unknown[]; artifacts: unknown[] } | null = null) {
  let callbackId = 0;
  Object.assign(globalThis, {
    window: {
      __TAURI_INTERNALS__: {
        transformCallback: () => ++callbackId,
        unregisterCallback: () => {},
        invoke: async (cmd: string) => {
          if (cmd === "plugin:event|listen") return callbackId;
          if (cmd === "daemon_get_status_snapshot") {
            return { agents: [], claudeRole: "lead", codexRole: "coder" };
          }
          if (cmd === "daemon_get_task_snapshot") return snapshot;
          if (cmd === "daemon_clear_active_task") return null;
          return null;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => {},
      },
      addEventListener: () => {},
      removeEventListener: () => {},
      innerHeight: 900,
    },
    localStorage: {
      getItem: () => null,
      setItem: () => {},
      removeItem: () => {},
      clear: () => {},
      key: () => null,
      length: 0,
    },
  });
}

describe("ReplyInput", () => {
  test("restores the task-scoped reply target instead of falling back to auto", () => {
    useTaskStore.setState({
      activeTaskId: "task-2",
      tasks: {
        "task-2": makeTask("task-2", "/repo-b"),
      },
      replyTargets: {
        "task-2": "coder",
      },
      sessions: {
        "task-2": [],
      },
      artifacts: {},
      providerHistory: {},
      providerHistoryLoading: {},
      providerHistoryError: {},
      bootstrapComplete: true,
      bootstrapError: null,
    });
    expect(selectActiveReplyTarget(useTaskStore.getState())).toBe("coder");
  });

  test("renders a centered pill grip with a narrow trigger zone instead of a full-width strip", async () => {
    installTauriStub();
    const { ReplyInput } = await import("./index");

    const html = renderToStaticMarkup(<ReplyInput />);

    expect(html).toContain("data-reply-input-resize-handle=\"true\"");
    expect(html).toContain("absolute left-1/2 top-0");
    expect(html).toContain("data-reply-input-resize-grip=\"true\"");
    expect(html).toContain("w-14");
    expect(html).not.toContain("hover:bg-primary/25");
    expect(html).not.toContain("cursor-row-resize");
  });

  test("returns a reconnect warning when the connected agent belongs to another task session", () => {
    const task = makeTask("task-2", "/repo-b");
    const session = makeClaudeLeadSession("task-2", "claude_current", "/repo-b");
    expect(
      getTaskSessionWarning({
        target: "auto",
        activeTask: task,
        sessions: [session],
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
        claudeRole: "lead",
        codexRole: "coder",
      }),
    ).toBe("Reconnect to this task");
  });

  test("disables send when no active task exists", async () => {
    installTauriStub();
    useTaskStore.setState({
      activeTaskId: null,
      tasks: {},
      replyTargets: {},
      sessions: {},
      artifacts: {},
      providerHistory: {},
      providerHistoryLoading: {},
      providerHistoryError: {},
      bootstrapComplete: true,
      bootstrapError: null,
    });
    const { ReplyInput } = await import("./index");
    const html = renderToStaticMarkup(<ReplyInput />);
    expect(html).toContain("Create a task first");
  });

  test("returns no warning when the connected agent matches the active task session", () => {
    const task = makeTask("task-2", "/repo-b");
    const session = makeClaudeLeadSession("task-2", "claude_current", "/repo-b");
    expect(
      getTaskSessionWarning({
        target: "auto",
        activeTask: task,
        sessions: [session],
        agents: {
          claude: {
            name: "claude",
            displayName: "Claude Code",
            status: "connected",
            providerSession: {
              provider: "claude",
              externalSessionId: "claude_current",
              cwd: "/repo-b",
              connectionMode: "resumed",
            },
          },
          codex: {
            name: "codex",
            displayName: "Codex",
            status: "disconnected",
          },
        },
        claudeRole: "lead",
        codexRole: "coder",
      }),
    ).toBeNull();
  });
});
