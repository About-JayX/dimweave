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

  test("agent list shows rows matching provided initialAgents", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead" }, { provider: "codex", role: "coder" }]} />,
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

  test("agent list section has a scrollable region", async () => {
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

  test("create mode does not add draggable row markers when empty", async () => {
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

  // TDD: provider-aware config fields — these fail against current (Task 1) code

  test("right pane config form has a separate model field", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('placeholder="model"');
  });

  test("model field renders empty for agents without a preselected model", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    // model input present but has no preselected value
    expect(html).toContain('placeholder="model"');
    expect(html).not.toContain('value="claude-opus"');
    expect(html).not.toContain('value="claude-sonnet"');
  });

  test("right pane config form shows New session and Resume session options", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain("New session");
    expect(html).toContain("Resume session");
  });

  test("resume session mode shows session ID input when historyAction is resumeExternal", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1",
          historyAction: { kind: "resumeExternal", externalId: "sess-abc" } } as any]} />,
    );
    expect(html).toContain('placeholder="session ID"');
    expect(html).toContain('value="sess-abc"');
  });

  test("right pane config form has an effort field", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('placeholder="effort"');
  });

  // Provider-capability gating tests

  test("claude effort placeholder is 'effort', codex is 'reasoning effort'", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const claudeHtml = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(claudeHtml).toContain('placeholder="effort"');
    expect(claudeHtml).not.toContain('placeholder="reasoning effort"');
    const codexHtml = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1" }]} />,
    );
    expect(codexHtml).toContain('placeholder="reasoning effort"');
    expect(codexHtml).not.toContain('placeholder="effort"');
  });

  test("codex effort is disabled when model is empty", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1" }]} />,
    );
    expect(html).toContain('placeholder="reasoning effort"');
    expect(html).toContain("disabled");
  });

  test("codex effort is enabled when model is set", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1", model: "o3-pro" }]} />,
    );
    expect(html).toContain('placeholder="reasoning effort"');
    // When model is set, the effort input should not have the disabled="" attribute
    const effortMatch = html.match(/<input[^>]*placeholder="reasoning effort"[^>]*/);
    expect(effortMatch).toBeTruthy();
    expect(effortMatch![0]).not.toContain('disabled=""');
  });

  test("claude resume placeholder is 'session ID', codex is 'thread ID'", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const claudeHtml = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1",
          historyAction: { kind: "resumeExternal", externalId: "s1" } } as any]} />,
    );
    expect(claudeHtml).toContain('placeholder="session ID"');
    const codexHtml = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1",
          historyAction: { kind: "resumeExternal", externalId: "t1" } } as any]} />,
    );
    expect(codexHtml).toContain('placeholder="thread ID"');
  });

  // TDD: two-pane shell — these fail against current code

  test("dialog renders two-pane layout with left pane and right pane", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain('data-left-pane="true"');
    expect(html).toContain('data-right-pane="true"');
  });

  test("create mode starts with empty agent list by default", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).not.toContain('value="lead"');
    expect(html).not.toContain('value="coder"');
  });

  test("right pane shows placeholder when no agent is selected", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain('data-right-pane-placeholder="true"');
  });
});
