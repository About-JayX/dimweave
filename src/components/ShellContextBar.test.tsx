import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

function installTauriStub() {
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
          return null;
        },
      },
      __TAURI_EVENT_PLUGIN_INTERNALS__: {
        unregisterListener: () => {},
      },
    },
  });
}

describe("ShellContextBar", () => {
  test("renders a VS Code-style side rail without reviving the top runtime header", async () => {
    installTauriStub();
    const { ShellContextBar } = await import("./ShellContextBar");
    const html = renderToStaticMarkup(
      <ShellContextBar
        activeItem={null}
        approvalCount={0}
        bugCount={0}
        messageCount={0}
        runtimeHealth={null}
        themeMode="auto"
        radiusMode="rounded"
        onToggle={() => {}}
        onThemeChange={() => {}}
        onRadiusToggle={() => {}}
      />,
    );

    expect(html).toContain("Task context");
    expect(html).toContain("Agents");
    expect(html).toContain("Approvals");
    expect(html).toContain("Logs");
    expect(html).not.toContain("AgentNexus");
    expect(html).not.toContain("Daemon online");
  });

  test("renders a compact runtime warning affordance when the daemon is degraded", async () => {
    installTauriStub();
    const { ShellContextBar } = await import("./ShellContextBar");
    const html = renderToStaticMarkup(
      <ShellContextBar
        activeItem={null}
        approvalCount={0}
        bugCount={0}
        messageCount={0}
        runtimeHealth={{
          level: "error",
          source: "claude_sdk",
          message: "Claude reconnect failed after 5 attempts",
        }}
        themeMode="auto"
        radiusMode="rounded"
        onToggle={() => {}}
        onThemeChange={() => {}}
        onRadiusToggle={() => {}}
      />,
    );

    expect(html).toContain("Runtime degraded");
    expect(html).toContain("Claude reconnect failed after 5 attempts");
  });

  test("removes the runtime warning affordance once health clears", async () => {
    installTauriStub();
    const { ShellContextBar } = await import("./ShellContextBar");
    const degraded = renderToStaticMarkup(
      <ShellContextBar
        activeItem={null}
        approvalCount={0}
        bugCount={0}
        messageCount={0}
        runtimeHealth={{
          level: "warning",
          source: "claude_sdk",
          message: "Claude reconnecting (1/5)",
        }}
        themeMode="auto"
        radiusMode="rounded"
        onToggle={() => {}}
        onThemeChange={() => {}}
        onRadiusToggle={() => {}}
      />,
    );
    const recovered = renderToStaticMarkup(
      <ShellContextBar
        activeItem={null}
        approvalCount={0}
        bugCount={0}
        messageCount={0}
        runtimeHealth={null}
        themeMode="auto"
        radiusMode="rounded"
        onToggle={() => {}}
        onThemeChange={() => {}}
        onRadiusToggle={() => {}}
      />,
    );

    expect(degraded).toContain("Runtime degraded");
    expect(recovered).not.toContain("Runtime degraded");
  });

  test("shows a pending approval badge on the approvals rail item", async () => {
    installTauriStub();
    const { ShellContextBar } = await import("./ShellContextBar");
    const html = renderToStaticMarkup(
      <ShellContextBar
        activeItem={null}
        approvalCount={3}
        bugCount={0}
        messageCount={0}
        runtimeHealth={null}
        themeMode="auto"
        radiusMode="rounded"
        onToggle={() => {}}
        onThemeChange={() => {}}
        onRadiusToggle={() => {}}
      />,
    );

    expect(html).toContain("3");
    expect(html).toContain("Open approvals");
  });

  test("renders Bug Inbox rail item and badge", async () => {
    installTauriStub();
    const { ShellContextBar } = await import("./ShellContextBar");
    const html = renderToStaticMarkup(
      <ShellContextBar
        activeItem={null}
        approvalCount={0}
        bugCount={5}
        messageCount={0}
        runtimeHealth={null}
        themeMode="auto"
        radiusMode="rounded"
        onToggle={() => {}}
        onThemeChange={() => {}}
        onRadiusToggle={() => {}}
      />,
    );

    expect(html).toContain("Bug Inbox");
    expect(html).toContain("5");
  });
});
