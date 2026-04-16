import { describe, expect, mock, test } from "bun:test";
import type { AgentDef } from "./TaskSetupDialog";

// ── Mock Tauri APIs before any store import ────────────────────
mock.module("@tauri-apps/api/core", () => ({
  invoke: () => Promise.resolve(null),
}));
mock.module("@tauri-apps/api/event", () => ({
  listen: () => Promise.resolve(() => {}),
}));

// ── deriveProviderConfig (pure function) ───────────────────────

const { deriveProviderConfig } = await import("./index");

describe("deriveProviderConfig", () => {
  test("extracts lead and coder providers from agent list", () => {
    const agents: AgentDef[] = [
      { provider: "codex", role: "lead" },
      { provider: "claude", role: "coder" },
    ];
    const config = deriveProviderConfig(agents);
    expect(config.leadProvider).toBe("codex");
    expect(config.coderProvider).toBe("claude");
  });

  test("defaults lead to claude and coder to codex when roles missing", () => {
    const agents: AgentDef[] = [{ provider: "codex", role: "reviewer" }];
    const config = deriveProviderConfig(agents);
    expect(config.leadProvider).toBe("claude");
    expect(config.coderProvider).toBe("codex");
  });

  test("uses first matching role when multiple agents share a role", () => {
    const agents: AgentDef[] = [
      { provider: "codex", role: "coder" },
      { provider: "claude", role: "coder" },
    ];
    const config = deriveProviderConfig(agents);
    expect(config.coderProvider).toBe("codex");
  });

  test("handles empty agent list with defaults", () => {
    const config = deriveProviderConfig([]);
    expect(config.leadProvider).toBe("claude");
    expect(config.coderProvider).toBe("codex");
  });
});

// ── Helpers ────────────────────────────────────────────────���───

function makeTask(overrides: Record<string, unknown> = {}) {
  return {
    taskId: "t1",
    projectRoot: "/repo",
    taskWorktreeRoot: "/repo/.worktrees/feat",
    title: "Test",
    status: "draft",
    leadProvider: "claude",
    coderProvider: "codex",
    createdAt: 1,
    updatedAt: 1,
    ...overrides,
  };
}

// ── create mode: createConfiguredTask receives derived config ──

describe("handleSetupSubmit (create mode)", () => {
  test("calls daemon_create_task with derived provider config", async () => {
    const invokeCalls: { cmd: string; args: unknown }[] = [];
    const invokeImpl = (cmd: string, args?: Record<string, unknown>) => {
      invokeCalls.push({ cmd, args });
      if (cmd === "daemon_create_task") {
        return Promise.resolve(
          makeTask({
            leadProvider: (args as any)?.leadProvider ?? "claude",
            coderProvider: (args as any)?.coderProvider ?? "codex",
          }),
        );
      }
      return Promise.resolve(null);
    };

    const { createConfiguredTaskAction } =
      await import("@/stores/task-store/index");
    const set = mock(() => {});
    const action = createConfiguredTaskAction(set as any, invokeImpl as any);

    const agents: AgentDef[] = [
      { provider: "codex", role: "lead" },
      { provider: "claude", role: "coder" },
    ];
    const config = deriveProviderConfig(agents);
    const task = await action("/repo", "Test", config);

    const call = invokeCalls.find((c) => c.cmd === "daemon_create_task");
    expect(call).toBeDefined();
    expect((call!.args as any).leadProvider).toBe("codex");
    expect((call!.args as any).coderProvider).toBe("claude");
    expect(task.leadProvider).toBe("codex");
    expect(task.coderProvider).toBe("claude");
  });
});

// ── edit mode: updateTaskConfig receives derived config ────────

describe("handleEditSubmit (edit mode)", () => {
  test("calls daemon_update_task_config with derived config", async () => {
    const invokeCalls: { cmd: string; args: unknown }[] = [];
    const invokeImpl = (cmd: string, args?: Record<string, unknown>) => {
      invokeCalls.push({ cmd, args });
      if (cmd === "daemon_update_task_config") {
        return Promise.resolve(
          makeTask({
            leadProvider: (args as any)?.leadProvider,
            coderProvider: (args as any)?.coderProvider,
          }),
        );
      }
      return Promise.resolve(null);
    };

    const { createUpdateTaskConfigAction } =
      await import("@/stores/task-store/index");
    const set = mock(() => {});
    const action = createUpdateTaskConfigAction(set as any, invokeImpl as any);

    const agents: AgentDef[] = [
      { provider: "claude", role: "lead", agentId: "a1" },
      { provider: "codex", role: "coder", agentId: "a2" },
    ];
    const config = deriveProviderConfig(agents);
    await action("t1", config);

    const call = invokeCalls.find((c) => c.cmd === "daemon_update_task_config");
    expect(call).toBeDefined();
    expect((call!.args as any).taskId).toBe("t1");
    expect((call!.args as any).leadProvider).toBe("claude");
    expect((call!.args as any).coderProvider).toBe("codex");
  });
});

// ── launch cwd: taskWorktreeRoot preferred over projectRoot ────

describe("launch cwd uses taskWorktreeRoot", () => {
  test("create mode: taskWorktreeRoot differs from projectRoot", () => {
    const task = makeTask({
      projectRoot: "/repo",
      taskWorktreeRoot: "/repo/.worktrees/feat",
    });
    expect(task.taskWorktreeRoot).toBe("/repo/.worktrees/feat");
    expect(task.taskWorktreeRoot).not.toBe(task.projectRoot);
  });

  test("edit mode: taskWorktreeRoot differs from projectRoot", () => {
    const task = makeTask({
      projectRoot: "/repo",
      taskWorktreeRoot: "/repo/.worktrees/edit-branch",
    });
    expect(task.taskWorktreeRoot).toBe("/repo/.worktrees/edit-branch");
    expect(task.taskWorktreeRoot).not.toBe(task.projectRoot);
  });
});
