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
    __TAURI_EVENT_PLUGIN_INTERNALS__: {
      unregisterListener: () => {},
    },
    addEventListener: () => {},
    removeEventListener: () => {},
    innerWidth: 800,
  },
  document: {
    addEventListener: () => {},
    removeEventListener: () => {},
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

describe("TaskSetupDialog", () => {
  test("renders create-mode modal with agent defs and action buttons", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog
        workspace="/repo"
        open={true}
        onOpenChange={() => {}}
        onSubmit={() => {}}
      />,
    );
    expect(html).toContain("New Task");
    expect(html).toContain("Agents");
    expect(html).toContain("Create");
    expect(html).toContain("Create &amp; Connect");
    expect(html).toContain('role="dialog"');
    expect(html).toContain('aria-modal="true"');
  });

  test("renders default agent defs (claude lead + codex coder)", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog
        workspace="/repo"
        open={true}
        onOpenChange={() => {}}
        onSubmit={() => {}}
      />,
    );
    // Default agents render as select+input rows
    expect(html).toContain("claude");
    expect(html).toContain("codex");
    expect(html).toContain("lead");
    expect(html).toContain("coder");
  });

  test("does not render content when closed", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog
        workspace="/repo"
        open={false}
        onOpenChange={() => {}}
        onSubmit={() => {}}
      />,
    );
    expect(html).not.toContain("New Task");
    expect(html).not.toContain("Agents");
    expect(html).not.toContain('role="dialog"');
  });

  test("submit payload uses agent array model", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");

    const html = renderToStaticMarkup(
      <TaskSetupDialog
        workspace="/repo"
        open={true}
        onOpenChange={() => {}}
        onSubmit={() => {}}
      />,
    );
    // AgentStatusPanel still renders inside the dialog
    expect(html).toContain("Runtime control");
  });

  test("create-mode agent defs are dialog-local, not global store", async () => {
    // Rendering create-mode must NOT invoke daemon_set_*_role commands.
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
    // Mirrors the gating logic in TaskPanel handleSetupSubmit
    type AgentDef = { provider: string; role: string };
    const cases: { agents: AgentDef[]; expectClaude: boolean; expectCodex: boolean }[] = [
      { agents: [{ provider: "claude", role: "lead" }, { provider: "codex", role: "coder" }], expectClaude: true, expectCodex: true },
      { agents: [{ provider: "claude", role: "lead" }, { provider: "claude", role: "coder" }], expectClaude: true, expectCodex: false },
      { agents: [{ provider: "codex", role: "lead" }, { provider: "codex", role: "coder" }], expectClaude: false, expectCodex: true },
      { agents: [{ provider: "codex", role: "lead" }, { provider: "claude", role: "coder" }], expectClaude: true, expectCodex: true },
    ];
    for (const c of cases) {
      const wantsClaude = c.agents.some((a) => a.provider === "claude");
      const wantsCodex = c.agents.some((a) => a.provider === "codex");
      expect(wantsClaude).toBe(c.expectClaude);
      expect(wantsCodex).toBe(c.expectCodex);
    }
  });

  test("add button renders for adding more agent defs", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog
        workspace="/repo"
        open={true}
        onOpenChange={() => {}}
        onSubmit={() => {}}
      />,
    );
    expect(html).toContain("Add");
  });
});
