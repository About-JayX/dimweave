import { describe, expect, test } from "bun:test";
import { canConnectCodex } from "./codex-launch-config";

describe("canConnectCodex", () => {
  test("requires an explicit role selection before connect", () => {
    expect(
      canConnectCodex({
        cwd: "/repo",
        role: "",
        connecting: false,
        running: false,
      }),
    ).toBe(false);
  });

  test("allows connect when workspace and role are both selected", () => {
    expect(
      canConnectCodex({
        cwd: "/repo",
        role: "coder",
        connecting: false,
        running: false,
      }),
    ).toBe(true);
  });
});
