import { describe, expect, test } from "bun:test";
import { buildClaudeLaunchRequest } from "./launch-request";

describe("buildClaudeLaunchRequest", () => {
  test("uses the selected Claude role instead of hard-coding lead", () => {
    expect(
      buildClaudeLaunchRequest({
        claudeRole: "coder",
        cwd: "/repo",
        model: "claude-sonnet-4-20250514",
        effort: "high",
        resumeSessionId: "session-123",
      }),
    ).toEqual({
      roleId: "coder",
      cwd: "/repo",
      model: "claude-sonnet-4-20250514",
      effort: "high",
      resumeSessionId: "session-123",
      taskId: null,
    });
  });

  test("normalizes blank optional fields to null", () => {
    expect(
      buildClaudeLaunchRequest({
        claudeRole: "coder",
        cwd: "/repo",
        model: "   ",
        effort: "  ",
        resumeSessionId: undefined,
      }),
    ).toEqual({
      roleId: "coder",
      cwd: "/repo",
      model: null,
      effort: null,
      resumeSessionId: null,
      taskId: null,
    });
  });

  test("rejects a blank Claude role", () => {
    expect(() =>
      buildClaudeLaunchRequest({
        claudeRole: "   ",
        cwd: "/repo",
      }),
    ).toThrow("Select Claude role before connecting");
  });
});
