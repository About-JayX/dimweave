import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

let callbackId = 0;
Object.assign(globalThis, {
  window: {
    __TAURI_INTERNALS__: {
      transformCallback: () => ++callbackId,
      unregisterCallback: () => {},
      invoke: async (cmd: string) => {
        if (cmd === "plugin:event|listen") return callbackId;
        if (cmd === "daemon_get_status_snapshot")
          return { agents: [], claudeRole: "lead", codexRole: "coder" };
        if (cmd === "daemon_get_task_snapshot") return null;
        return null;
      },
    },
    __TAURI_EVENT_PLUGIN_INTERNALS__: { unregisterListener: () => {} },
    addEventListener: () => {},
    removeEventListener: () => {},
    innerWidth: 800,
  },
  document: { addEventListener: () => {}, removeEventListener: () => {} },
  localStorage: {
    getItem: () => null, setItem: () => {}, removeItem: () => {},
    clear: () => {}, key: () => null, length: 0,
  },
});

describe("ErrorBoundary", () => {
  test("renders children in normal state", async () => {
    const { ErrorBoundary } = await import("./ErrorBoundary");
    const html = renderToStaticMarkup(
      <ErrorBoundary><div>Child OK</div></ErrorBoundary>,
    );
    expect(html).toContain("Child OK");
    expect(html).not.toContain("Retry");
  });

  test("fallback contains Retry button and error message", async () => {
    const { ErrorBoundary } = await import("./ErrorBoundary");
    // Directly test the render path when hasError is true.
    // ErrorBoundary is a class; construct and set state manually.
    const instance = new ErrorBoundary({ children: <div>Child</div> });
    instance.state = { hasError: true };
    const element = instance.render();
    const html = renderToStaticMarkup(element as React.ReactElement);
    expect(html).toContain("Retry");
    expect(html).toContain("Something went wrong");
    // Must NOT contain auto-remount (no requestAnimationFrame)
    expect(html).not.toContain("Child");
  });

  test("pushes to uiErrors, not terminalLines", async () => {
    const { ErrorBoundary } = await import("./ErrorBoundary");
    const { useBridgeStore } = await import("@/stores/bridge-store");
    const before = useBridgeStore.getState().terminalLines.length;
    const instance = new ErrorBoundary({ children: <div /> });
    instance.componentDidCatch(new Error("test crash"), {
      componentStack: "\n    at Broken",
    } as any);
    const after = useBridgeStore.getState();
    expect(after.uiErrors.length).toBeGreaterThan(0);
    expect(after.uiErrors[after.uiErrors.length - 1].message).toBe("test crash");
    // terminalLines should NOT grow from this
    expect(after.terminalLines.length).toBe(before);
  });
});
