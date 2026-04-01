import { describe, expect, test } from "bun:test";
import {
  NEW_PROVIDER_SESSION_VALUE,
  buildProviderHistoryOptions,
  formatProviderConnectionLabel,
  resolveProviderHistoryWorkspace,
} from "../src/components/AgentStatus/provider-session-view-model";
import type { ProviderHistoryInfo } from "../src/stores/task-store/types";
import type { ProviderSessionInfo } from "../src/types";

function makeHistory(
  provider: "claude" | "codex",
  externalId: string,
  updatedAt: number,
): ProviderHistoryInfo {
  return {
    provider,
    externalId,
    title: `${provider} ${externalId}`,
    preview: `Preview ${externalId}`,
    cwd: "/tmp/ws",
    archived: false,
    createdAt: updatedAt - 10,
    updatedAt,
    status: "paused",
    normalizedSessionId: null,
    normalizedTaskId: null,
  };
}

describe("buildProviderHistoryOptions", () => {
  test("prepends a new-session option and filters entries by provider", () => {
    const options = buildProviderHistoryOptions("claude", [
      makeHistory("codex", "thread_1", 100),
      makeHistory("claude", "resume_2", 250),
      makeHistory("claude", "resume_1", 150),
    ]);

    expect(options.map((option) => option.value)).toEqual([
      NEW_PROVIDER_SESSION_VALUE,
      "resume_2",
      "resume_1",
    ]);
    expect(options[0]?.label).toBe("New session");
    expect(options[1]?.description).toContain("Preview resume_2");
  });
});

describe("formatProviderConnectionLabel", () => {
  test("surfaces whether the current provider connection is new or resumed", () => {
    const resumed: ProviderSessionInfo = {
      provider: "claude",
      externalSessionId: "claude_resume_42",
      cwd: "/tmp/ws",
      connectionMode: "resumed",
    };
    const fresh: ProviderSessionInfo = {
      provider: "codex",
      externalSessionId: "thread_123",
      cwd: "/tmp/ws",
      connectionMode: "new",
    };

    expect(formatProviderConnectionLabel(resumed)).toBe(
      "Resumed claude_resume_42",
    );
    expect(formatProviderConnectionLabel(fresh)).toBe("New thread thread_123");
  });
});

describe("resolveProviderHistoryWorkspace", () => {
  test("falls back to the connected provider session cwd when no local cwd is selected", () => {
    const session: ProviderSessionInfo = {
      provider: "claude",
      externalSessionId: "claude_resume_42",
      cwd: "/tmp/provider-ws",
      connectionMode: "resumed",
    };

    expect(resolveProviderHistoryWorkspace("", session)).toBe("/tmp/provider-ws");
    expect(resolveProviderHistoryWorkspace("/tmp/manual-ws", session)).toBe(
      "/tmp/manual-ws",
    );
    expect(resolveProviderHistoryWorkspace("", undefined)).toBe("");
  });
});
