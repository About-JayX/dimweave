import { describe, expect, test } from "bun:test";
import {
  buildCodexLaunchConfig,
  canConnectCodex,
  getDefaultReasoningEffort,
} from "../src/components/AgentStatus/codex-launch-config";

describe("canConnectCodex", () => {
  test("requires a selected project directory before connect is enabled", () => {
    expect(canConnectCodex({ cwd: "", connecting: false, running: false })).toBe(
      false,
    );
    expect(
      canConnectCodex({
        cwd: "/Users/jason/floder/agent-bridge",
        connecting: false,
        running: false,
      }),
    ).toBe(true);
  });
});

describe("buildCodexLaunchConfig", () => {
  test("preserves model, reasoning effort, and cwd in launch payload", () => {
    expect(
      buildCodexLaunchConfig({
        model: "gpt-5.4",
        reasoningEffort: "xhigh",
        cwd: "/tmp/project",
      }),
    ).toEqual({
      model: "gpt-5.4",
      reasoningEffort: "xhigh",
      cwd: "/tmp/project",
    });
  });

  test("passes through resumeThreadId when reconnecting an existing thread", () => {
    expect(
      buildCodexLaunchConfig({
        model: "gpt-5.4",
        reasoningEffort: "xhigh",
        cwd: "/tmp/project",
        resumeThreadId: "thread_123",
      }),
    ).toEqual({
      model: "gpt-5.4",
      reasoningEffort: "xhigh",
      cwd: "/tmp/project",
      resumeThreadId: "thread_123",
    });
  });
});

describe("getDefaultReasoningEffort", () => {
  test("prefers model default and falls back to first supported effort", () => {
    expect(
      getDefaultReasoningEffort({
        defaultReasoningLevel: "high",
        reasoningLevels: [{ effort: "low" }, { effort: "high" }],
      }),
    ).toBe("high");

    expect(
      getDefaultReasoningEffort({
        defaultReasoningLevel: null,
        reasoningLevels: [{ effort: "medium" }, { effort: "high" }],
      }),
    ).toBe("medium");
  });
});
