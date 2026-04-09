import { describe, expect, test } from "bun:test";
import { canConnectClaude } from "./connect-state";

describe("canConnectClaude", () => {
  test("requires an explicit role selection before connect", () => {
    expect(
      canConnectClaude({
        cwd: "/repo",
        role: "",
        connecting: false,
        connected: false,
        disconnecting: false,
      }),
    ).toBe(false);
  });

  test("allows connect when workspace and role are both selected", () => {
    expect(
      canConnectClaude({
        cwd: "/repo",
        role: "lead",
        connecting: false,
        connected: false,
        disconnecting: false,
      }),
    ).toBe(true);
  });
});
