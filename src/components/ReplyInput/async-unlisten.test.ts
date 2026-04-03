import { describe, expect, test } from "bun:test";
import { createAsyncUnlistenCleanup } from "./async-unlisten";

async function flushMicrotasks() {
  await Promise.resolve();
  await Promise.resolve();
}

describe("createAsyncUnlistenCleanup", () => {
  test("cleans up listeners that finish registering after unmount", async () => {
    let resolveRegistration!: (unlisten: () => void) => void;
    const registration = new Promise<() => void>((resolve) => {
      resolveRegistration = resolve;
    });
    let unlistenCalls = 0;

    const cleanup = createAsyncUnlistenCleanup(() => registration);
    cleanup();

    resolveRegistration(() => {
      unlistenCalls += 1;
    });
    await flushMicrotasks();

    expect(unlistenCalls).toBe(1);
  });

  test("cleans up active listeners during unmount", async () => {
    let unlistenCalls = 0;
    const cleanup = createAsyncUnlistenCleanup(
      async () => () => {
        unlistenCalls += 1;
      },
    );

    await flushMicrotasks();
    cleanup();
    cleanup();

    expect(unlistenCalls).toBe(1);
  });
});
