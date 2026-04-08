import { describe, expect, mock, test } from "bun:test";
import { collectPastedAttachmentPaths } from "./paste-attachments";

describe("collectPastedAttachmentPaths", () => {
  test("returns the backend-provided paths unchanged", async () => {
    const result = await collectPastedAttachmentPaths(async () => [
      "/tmp/clip.png",
      "/tmp/spec.md",
    ]);

    expect(result).toEqual(["/tmp/clip.png", "/tmp/spec.md"]);
  });

  test("swallows backend failures so normal text paste can continue", async () => {
    const oldError = console.error;
    const errorSpy = mock(() => {});
    console.error = errorSpy;

    try {
      const result = await collectPastedAttachmentPaths(async () => {
        throw new Error("clipboard unavailable");
      });

      expect(result).toEqual([]);
      expect(errorSpy).toHaveBeenCalled();
    } finally {
      console.error = oldError;
    }
  });
});
