/**
 * Minimal happy-dom environment for interaction tests.
 * Call setupDOM() before component imports, teardownDOM() after.
 */
import { Window } from "happy-dom";
import { createElement, type ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";

let win: InstanceType<typeof Window> | null = null;
let root: Root | null = null;
let container: HTMLElement | null = null;

/** Install happy-dom globals + Tauri stubs. Must run before any component import. */
export function setupDOM() {
  win = new Window({ url: "http://localhost" });
  const doc = win.document;
  container = doc.createElement("div") as unknown as HTMLElement;
  (doc.body as any).appendChild(container);

  // Add Tauri stubs to the happy-dom window
  let cbId = 0;
  Object.assign(win, {
    __TAURI_INTERNALS__: {
      transformCallback: () => ++cbId,
      unregisterCallback: () => {},
      invoke: async (cmd: string) => {
        if (cmd === "plugin:event|listen") return cbId;
        if (cmd === "daemon_get_status_snapshot")
          return { agents: [], claudeRole: "lead", codexRole: "coder" };
        if (cmd === "daemon_get_task_snapshot") return null;
        if (cmd === "codex_list_models") return [];
        if (cmd === "codex_get_profile") return null;
        return null;
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
    innerWidth: 800,
  });

  // Set globals for React and Tauri
  Object.assign(globalThis, {
    window: win,
    document: doc,
    navigator: win.navigator,
    HTMLElement: win.HTMLElement,
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

/** Render a React element and flush. */
export async function render(el: ReactElement) {
  if (!container || !win) throw new Error("call setupDOM() first");
  root = createRoot(container);
  root.render(el);
  await new Promise((r) => setTimeout(r, 50));
  return container;
}

/** Query helpers. */
export function query(selector: string) {
  return container?.querySelector(selector) ?? null;
}
export function queryAll(selector: string) {
  return Array.from(container?.querySelectorAll(selector) ?? []);
}

/** Simulate a click. */
export function click(el: Element) {
  (el as any).click();
}

/** Cleanup. */
export function teardownDOM() {
  root?.unmount();
  root = null;
  container = null;
  win?.close();
  win = null;
}
