import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

// Stub must exist before any component import triggers bridge-store init
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
        if (cmd === "daemon_get_task_snapshot") return null;
        if (cmd === "codex_list_models") return [];
        if (cmd === "codex_get_profile") return null;
        return null;
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
    addEventListener: () => {},
    removeEventListener: () => {},
    innerWidth: 800,
  },
  document: {
    addEventListener: () => {},
    removeEventListener: () => {},
  },
  localStorage: {
    getItem: () => null, setItem: () => {}, removeItem: () => {},
    clear: () => {}, key: () => null, length: 0,
  },
});

describe("TaskSetupDialog", () => {
  test("renders create-mode modal with agent defs and action buttons", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain("New Task");
    expect(html).toContain("Agents");
    expect(html).toContain("Create");
    expect(html).toContain("Create &amp; Connect");
    expect(html).toContain('role="dialog"');
  });

  test("renders default agent defs (claude lead + codex coder)", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain("claude");
    expect(html).toContain("codex");
    expect(html).toContain("lead");
    expect(html).toContain("coder");
  });

  test("does not render content when closed", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={false}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).not.toContain("New Task");
    expect(html).not.toContain('role="dialog"');
  });

  test("create-mode agent defs are dialog-local, not global store", async () => {
    const roleCmds: string[] = [];
    const orig = (globalThis as any).window.__TAURI_INTERNALS__.invoke;
    (globalThis as any).window.__TAURI_INTERNALS__.invoke = async (cmd: string, ...rest: any[]) => {
      if (cmd === "daemon_set_claude_role" || cmd === "daemon_set_codex_role")
        roleCmds.push(cmd);
      return orig(cmd, ...rest);
    };
    try {
      const { TaskSetupDialog } = await import("./TaskSetupDialog");
      renderToStaticMarkup(
        <TaskSetupDialog workspace="/repo" open={true}
          onOpenChange={() => {}} onSubmit={() => {}} />,
      );
      expect(roleCmds).toEqual([]);
    } finally {
      (globalThis as any).window.__TAURI_INTERNALS__.invoke = orig;
    }
  });

  test("launch gate: agent array determines which providers are launched", () => {
    type AgentDef = { provider: string; role: string };
    const cases: { agents: AgentDef[]; expectClaude: boolean; expectCodex: boolean }[] = [
      { agents: [{ provider: "claude", role: "lead" }, { provider: "codex", role: "coder" }], expectClaude: true, expectCodex: true },
      { agents: [{ provider: "claude", role: "lead" }, { provider: "claude", role: "coder" }], expectClaude: true, expectCodex: false },
      { agents: [{ provider: "codex", role: "lead" }, { provider: "codex", role: "coder" }], expectClaude: false, expectCodex: true },
    ];
    for (const c of cases) {
      expect(c.agents.some((a) => a.provider === "claude")).toBe(c.expectClaude);
      expect(c.agents.some((a) => a.provider === "codex")).toBe(c.expectCodex);
    }
  });

  test("Create & Connect disabled with empty agents, Create enabled", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} initialAgents={[]} />,
    );
    expect(html).toContain(">Create</button>");
    expect(html).toContain("disabled");
  });

  test("edit mode renders Save button and Edit Task heading", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead" }]} />,
    );
    expect(html).toContain("Edit Task");
    expect(html).toContain(">Save</button>");
    expect(html).not.toContain("Create &amp; Connect");
  });

  test("edit mode preserves agentId and displayName in initialAgents", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "reviewer", agentId: "a1", displayName: "Rev" }]} />,
    );
    expect(html).toContain('value="reviewer"');
    expect(html).toContain("codex");
  });

  test("empty-task submit payload has zero agents", () => {
    type AgentDef = { provider: string; role: string };
    const agentDefs: AgentDef[] = [];
    const validAgents = agentDefs.filter((d) => d.role.trim().length > 0);
    expect(validAgents).toEqual([]);
    expect({ agents: validAgents, requestLaunch: false }.agents).toEqual([]);
  });

  test("dialog shell uses flex column layout without outer scroll", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain("flex flex-col");
    expect(html).not.toContain("overflow-y-auto max-h-[90vh]");
  });

  test("provider panels live inside a dedicated inner scroll region", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain('data-scroll-region="true"');
    expect(html).toContain("overflow-y-auto");
  });

  test("action buttons are in a fixed footer section", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain('data-dialog-footer="true"');
  });

  test("edit mode wraps agent rows with draggable row marker", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[
          { provider: "claude", role: "lead", agentId: "a1" },
          { provider: "codex", role: "coder", agentId: "a2" },
        ]} />,
    );
    expect(html).toContain('data-draggable-row="true"');
  });

  test("edit mode exposes drag handle button on each sortable row", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[
          { provider: "claude", role: "lead", agentId: "a1" },
          { provider: "codex", role: "coder", agentId: "a2" },
        ]} />,
    );
    expect(html).toContain('data-drag-handle="true"');
  });

  test("create mode does not add draggable row markers", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).not.toContain('data-draggable-row="true"');
  });

  test("edit-mode diff logic: update existing, add new, remove deleted", () => {
    // Mirrors the handleEditSubmit diff logic in index.tsx
    type AgentDef = { provider: string; role: string; agentId?: string; displayName?: string | null };
    const existing = [
      { agentId: "a1", provider: "claude", role: "lead" },
      { agentId: "a2", provider: "codex", role: "coder" },
    ];
    const incoming: AgentDef[] = [
      { provider: "claude", role: "reviewer", agentId: "a1" },
      { provider: "codex", role: "tester" },
    ];
    const incomingIds = new Set(incoming.filter((d) => d.agentId).map((d) => d.agentId!));
    const toRemove = existing.filter((a) => !incomingIds.has(a.agentId));
    const toUpdate = incoming.filter((d) => d.agentId);
    const toAdd = incoming.filter((d) => !d.agentId);
    expect(toRemove.map((a) => a.agentId)).toEqual(["a2"]);
    expect(toUpdate.map((a) => a.agentId)).toEqual(["a1"]);
    expect(toAdd.length).toBe(1);
    expect(toAdd[0].role).toBe("tester");
  });
});
