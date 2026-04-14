import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

// Stub Tauri internals
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

import { TaskAgentEditor, type AgentEditorPayload } from "./TaskAgentEditor";
import type { TaskAgentInfo } from "@/stores/task-store/types";

const noop = () => {};

describe("TaskAgentEditor", () => {
  test("add mode: heading says Add Agent, button says Add", () => {
    const html = renderToStaticMarkup(
      <TaskAgentEditor agent={null} onSubmit={noop} onCancel={noop} />,
    );
    expect(html).toContain("Add Agent");
    expect(html).toContain(">Add</button>");
    expect(html).not.toContain("Save");
  });

  test("edit mode: heading says Edit Agent, button says Save", () => {
    const agent: TaskAgentInfo = {
      agentId: "a1",
      taskId: "t1",
      provider: "codex",
      role: "coder",
      displayName: "My Codex",
      order: 0,
      createdAt: 1,
    };
    const html = renderToStaticMarkup(
      <TaskAgentEditor agent={agent} onSubmit={noop} onCancel={noop} />,
    );
    expect(html).toContain("Edit Agent");
    expect(html).toContain(">Save</button>");
    expect(html).not.toContain(">Add</button>");
  });

  test("edit mode pre-fills provider, role, and displayName", () => {
    const agent: TaskAgentInfo = {
      agentId: "a1",
      taskId: "t1",
      provider: "codex",
      role: "reviewer",
      displayName: "Code Reviewer",
      order: 0,
      createdAt: 1,
    };
    const html = renderToStaticMarkup(
      <TaskAgentEditor agent={agent} onSubmit={noop} onCancel={noop} />,
    );
    // Provider select has codex selected
    expect(html).toContain("codex");
    // Role input pre-filled
    expect(html).toContain('value="reviewer"');
    // Display name pre-filled
    expect(html).toContain('value="Code Reviewer"');
  });

  test("provider select offers claude and codex options", () => {
    const html = renderToStaticMarkup(
      <TaskAgentEditor agent={null} onSubmit={noop} onCancel={noop} />,
    );
    expect(html).toContain("<option");
    expect(html).toContain("claude");
    expect(html).toContain("codex");
  });

  test("dialog has proper accessibility attributes", () => {
    const html = renderToStaticMarkup(
      <TaskAgentEditor agent={null} onSubmit={noop} onCancel={noop} />,
    );
    expect(html).toContain('role="dialog"');
    expect(html).toContain('aria-modal="true"');
  });

  test("displays optional label for display name field", () => {
    const html = renderToStaticMarkup(
      <TaskAgentEditor agent={null} onSubmit={noop} onCancel={noop} />,
    );
    expect(html).toContain("Display name");
    expect(html).toContain("optional");
  });

  test("cancel button is present", () => {
    const html = renderToStaticMarkup(
      <TaskAgentEditor agent={null} onSubmit={noop} onCancel={noop} />,
    );
    expect(html).toContain("Cancel");
  });

  test("role input has placeholder text", () => {
    const html = renderToStaticMarkup(
      <TaskAgentEditor agent={null} onSubmit={noop} onCancel={noop} />,
    );
    expect(html).toContain("e.g. lead, coder, reviewer");
  });
});

describe("AgentEditorPayload construction", () => {
  test("add payload shape: provider + role + displayName", () => {
    const payload: AgentEditorPayload = {
      provider: "claude",
      role: "reviewer",
      displayName: "My Reviewer",
    };
    expect(payload.provider).toBe("claude");
    expect(payload.role).toBe("reviewer");
    expect(payload.displayName).toBe("My Reviewer");
  });

  test("empty displayName becomes null", () => {
    // Mirrors the handleSubmit logic in TaskAgentEditor
    const raw = "";
    const displayName = raw.trim() || null;
    expect(displayName).toBeNull();
  });

  test("whitespace-only displayName becomes null", () => {
    const raw = "   ";
    const displayName = raw.trim() || null;
    expect(displayName).toBeNull();
  });

  test("edit payload preserves provider change", () => {
    const existing: TaskAgentInfo = {
      agentId: "a1", taskId: "t1", provider: "claude",
      role: "coder", displayName: null, order: 0, createdAt: 1,
    };
    const payload: AgentEditorPayload = {
      provider: "codex",
      role: existing.role,
      displayName: null,
    };
    expect(payload.provider).toBe("codex");
    expect(payload.role).toBe("coder");
  });
});
