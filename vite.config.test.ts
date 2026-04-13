import { describe, expect, test } from "bun:test";
import { WATCH_IGNORED_GLOBS } from "./vite.config";

describe("vite watch ignore", () => {
  test("WATCH_IGNORED_GLOBS includes .worktrees and worktrees patterns", () => {
    expect(WATCH_IGNORED_GLOBS).toContain("**/.worktrees/**");
    expect(WATCH_IGNORED_GLOBS).toContain("**/worktrees/**");
  });

  test("WATCH_IGNORED_GLOBS is a non-empty array", () => {
    expect(Array.isArray(WATCH_IGNORED_GLOBS)).toBe(true);
    expect(WATCH_IGNORED_GLOBS.length >= 2).toBe(true);
  });
});
