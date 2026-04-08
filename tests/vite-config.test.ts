import { afterEach, describe, expect, test } from "bun:test";

const DIMWEAVE_VITE_PORT = "DIMWEAVE_VITE_PORT";

afterEach(() => {
  delete process.env[DIMWEAVE_VITE_PORT];
});

describe("vite config", () => {
  test("dev server port stays aligned with tauri devUrl defaults", async () => {
    process.env[DIMWEAVE_VITE_PORT] = "3001";

    const mod = await import(`../vite.config.ts?case=${Date.now()}`);
    const config = mod.default;

    expect(config.server?.port).toBe(1420);
  });
});
