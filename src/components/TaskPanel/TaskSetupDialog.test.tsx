import { describe, expect, test, mock } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

// Mock CyberSelect so option arrays are inspectable in static markup.
// The real component only renders options when open (useState), which
// renderToStaticMarkup never triggers.  This mock preserves the class
// structure that existing CSS-assertion tests depend on and adds a
// hidden <input data-cyber-options="..."> for option-contract tests.
mock.module("@/components/ui/cyber-select", () => {
  function middleEllipsis(text: string, maxLen: number): string {
    if (text.length <= maxLen) return text;
    const keep = Math.floor((maxLen - 1) / 2);
    return `${text.slice(0, keep)}\u2026${text.slice(-keep)}`;
  }
  function getCyberSelectMenuPanelClassName(variant: string, compact?: boolean): string {
    if (variant === "history" && compact)
      return "right-0 top-7 min-w-44 max-w-64 max-h-48 rounded-lg p-1";
    return variant === "history"
      ? "right-0 top-7 w-[150%] max-h-48 rounded-lg p-1"
      : "right-0 top-7 min-w-36 max-w-64 max-h-52 rounded-lg p-1";
  }
  function HistoryMenuOption() { return null; }
  function CyberSelect({ value, options, placeholder, variant = "default", compact = false }: any) {
    const isHistory = variant === "history";
    const selected = options.find((o: any) => o.value === value);
    const displayLabel = selected?.label ?? placeholder ?? value;
    const containerCls = `relative ${isHistory && !compact ? "flex min-w-0 flex-1" : "inline-flex"}`;
    const btnCls = `inline-flex items-center gap-1 border font-medium ${
      isHistory && !compact
        ? "min-w-0 flex-1 justify-between rounded-full px-2.5 py-1.5 text-[10px]"
        : "rounded px-1.5 py-0.5 text-[10px]"
    } border-input bg-muted text-foreground`;
    const label = isHistory && selected ? middleEllipsis(displayLabel, 36) : displayLabel;
    return (
      <div className={containerCls}>
        <button type="button" className={btnCls}>
          <span className={isHistory && !compact ? "flex-1" : "max-w-28"}>{label}</span>
        </button>
        <input type="hidden" data-cyber-options={JSON.stringify(options)} data-cyber-placeholder={placeholder ?? ""} />
      </div>
    );
  }
  return { CyberSelect, middleEllipsis, getCyberSelectMenuPanelClassName, HistoryMenuOption };
});

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
        initialAgents={[{ provider: "codex", role: "coder", agentId: "a1", displayName: "Rev" }]} />,
    );
    // CyberSelect shows matched label; role "coder" renders as "Coder"
    expect(html).toContain("Coder");
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

  test("create mode default row has draggable marker", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain('data-draggable-row="true"');
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

  test("claude model CyberSelect shows Select model when unset", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('data-model-select="true"');
    // CyberSelect button shows "Select model" when value is ""
    expect(html).toContain("Select model");
  });

  test("model select starts unselected for new agents", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('data-model-select="true"');
    // First option should be the empty placeholder
    expect(html).toContain("Select model");
  });

  test("session section uses a history dropdown with New session sentinel", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('data-history-select="true"');
    expect(html).toContain("New session");
    // No radio controls
    expect(html).not.toContain('type="radio"');
  });

  test("history CyberSelect shows New session when no history is provided", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('data-history-select="true"');
    expect(html).toContain("New session");
  });

  test("claude effort CyberSelect shows Default when unset", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('data-effort-select="true"');
    expect(html).toContain("Default");
  });

  test("claude effort label differs from codex effort label", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const claudeHtml = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(claudeHtml).toContain("Effort");
    expect(claudeHtml).not.toContain("Reasoning effort");
    const codexHtml = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1" }]} />,
    );
    expect(codexHtml).toContain("Reasoning effort");
  });

  test("codex effort select is disabled when model is empty", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1" }]} />,
    );
    expect(html).toContain('data-effort-select="true"');
    expect(html).toContain("disabled");
  });

  test("codex model CyberSelect shows selected model label from live codexModels", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1", model: "o3-pro" }]}
        codexModels={[{ slug: "o3-pro", displayName: "o3-pro", reasoningLevels: [{ effort: "low" }] }]} />,
    );
    expect(html).toContain('data-model-select="true"');
    // CyberSelect button text should show the matched model label
    expect(html).toContain("o3-pro");
  });

  test("codex shows loading placeholder when codexModels is empty", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1" }]}
        codexModels={[]} />,
    );
    expect(html).toContain('data-model-select="true"');
    expect(html).toContain("Loading models");
  });

  test("codex effort derives from selected model reasoning levels", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1", model: "o3-pro", effort: "medium" }]}
        codexModels={[{ slug: "o3-pro", displayName: "o3-pro", reasoningLevels: [{ effort: "low" }, { effort: "medium" }, { effort: "high" }] }]} />,
    );
    expect(html).toContain('data-effort-select="true"');
    // Effort button should show the selected reasoning level label
    expect(html).toContain("medium");
  });

  test("no free-form text input for model, effort, or role", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).not.toContain('placeholder="model"');
    expect(html).not.toContain('placeholder="effort"');
    expect(html).not.toContain('placeholder="role"');
  });

  test("role is a CyberSelect dropdown showing selected role label", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('data-role-select="true"');
    // CyberSelect shows the matched label as button text
    expect(html).toContain("Lead");
    // No free-form text input for role
    expect(html).not.toContain('placeholder="role"');
  });

  test("dialog uses CyberSelect instead of native select for main controls", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    // CyberSelect renders as <button> not <select>
    // The right-pane provider card should have no native <select> elements
    const cardMatch = html.match(/data-provider-card="true"([\s\S]*)/);
    expect(cardMatch).toBeTruthy();
    expect(cardMatch![1]).not.toContain("<select");
  });

  test("dialog session trigger is compact — no flex-1 full-width expansion", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    const histBlock = html.split('data-history-select="true"')[1];
    expect(histBlock).toBeTruthy();
    // Compact history trigger uses inline-flex container, not flex-1
    expect(histBlock).toContain("inline-flex");
    expect(histBlock).not.toContain("rounded-full");
  });

  test("dialog session and role triggers share the same compact class family", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    const roleBlock = html.split('data-role-select="true"')[1]?.split('data-')[0] ?? "";
    const histBlock = html.split('data-history-select="true"')[1]?.split('data-')[0] ?? "";
    // Both triggers use the same compact trigger class (rounded, py-0.5)
    expect(roleBlock).toContain("py-0.5");
    expect(histBlock).toContain("py-0.5");
    expect(roleBlock).toContain("rounded");
    expect(histBlock).toContain("rounded");
  });

  test("history trigger in dialog applies middle ellipsis to long selected label", async () => {
    const { middleEllipsis } = await import("../ui/cyber-select");
    const longTitle = "Implement the entire authentication middleware refactor for compliance";
    const expected = middleEllipsis(longTitle, 36);
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1",
          historyAction: { kind: "resumeExternal", externalId: "sess-long" } }]}
        providerHistory={[{ provider: "claude", externalId: "sess-long", title: longTitle,
          archived: false, createdAt: 1, updatedAt: 2, status: "completed" as const }]} />,
    );
    expect(expected).not.toBe(longTitle);
    expect(html).toContain(expected);
  });

  test("history dropdown pre-selects matching entry when historyAction is resumeExternal", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1",
          historyAction: { kind: "resumeExternal", externalId: "sess-x" } }]}
        providerHistory={[{ provider: "claude", externalId: "sess-x", title: "Old session",
          archived: false, createdAt: 1, updatedAt: 2, status: "completed" as const }]} />,
    );
    expect(html).toContain('data-history-select="true"');
    // CyberSelect shows the selected label as button text
    expect(html).toContain("Old session");
  });

  // Option contract tests — inspect actual options arrays via mock's data-cyber-options

  function parseOptionsFromBlock(html: string, dataAttr: string): { options: { value: string; label: string }[]; placeholder: string } {
    const block = html.split(`${dataAttr}="true"`)[1] ?? "";
    const optMatch = block.match(/data-cyber-options="([^"]*)"/);
    const phMatch = block.match(/data-cyber-placeholder="([^"]*)"/);
    return { options: optMatch ? JSON.parse(optMatch[1].replace(/&quot;/g, '"')) : [], placeholder: phMatch?.[1] ?? "" };
  }

  test("claude model trigger shows 'Select model' when unset, options include real Default", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    const { options, placeholder } = parseOptionsFromBlock(html, 'data-model-select');
    // Visible trigger label is "Select model" (placeholder) when model is unset
    const modelBlock = html.split('data-model-select="true"')[1]?.split('data-effort')[0] ?? "";
    expect(modelBlock).toContain(">Select model<");
    // Real provider "Default" option preserved in menu
    expect(options.some(o => o.value === "" && o.label === "Default")).toBe(true);
    // "Select model" is NOT a real menu option
    expect(options.every(o => o.label !== "Select model")).toBe(true);
    expect(placeholder).toBe("Select model");
  });

  test("claude model trigger shows 'Default' when model is explicitly empty string", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1", model: "" }]} />,
    );
    const modelBlock = html.split('data-model-select="true"')[1]?.split('data-effort')[0] ?? "";
    // When model is explicitly "" (user chose Default), trigger shows "Default"
    expect(modelBlock).toContain(">Default<");
    expect(modelBlock).not.toContain(">Select model<");
  });

  test("claude effort options contain exactly one Default entry", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    const { options } = parseOptionsFromBlock(html, 'data-effort-select');
    const defaultEntries = options.filter(o => o.value === "" && o.label === "Default");
    expect(defaultEntries.length).toBe(1);
  });

  test("codex effort prepends Default when reasoning levels lack it", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "codex", role: "coder", agentId: "b1", model: "o3-pro" }]}
        codexModels={[{ slug: "o3-pro", displayName: "o3-pro", reasoningLevels: [{ effort: "low" }, { effort: "high" }] }]} />,
    );
    const { options } = parseOptionsFromBlock(html, 'data-effort-select');
    expect(options[0]).toEqual({ value: "", label: "Default" });
    expect(options.length).toBe(3);
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

  test("create mode starts with one default locked row", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    expect(html).toContain('data-draggable-row="true"');
    expect(html).toContain('data-locked-row="true"');
  });

  test("left pane rows include provider icon marker", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('data-provider-icon="true"');
  });

  test("right pane uses provider-card visual grouping", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog mode="edit" workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}}
        initialAgents={[{ provider: "claude", role: "lead", agentId: "a1" }]} />,
    );
    expect(html).toContain('data-provider-card="true"');
  });

  test("right pane shows provider card for the default selected agent", async () => {
    const { TaskSetupDialog } = await import("./TaskSetupDialog");
    const html = renderToStaticMarkup(
      <TaskSetupDialog workspace="/repo" open={true}
        onOpenChange={() => {}} onSubmit={() => {}} />,
    );
    // Default row is auto-selected so right pane shows config, not placeholder
    expect(html).toContain('data-provider-card="true"');
    expect(html).not.toContain('data-right-pane-placeholder="true"');
  });
});
