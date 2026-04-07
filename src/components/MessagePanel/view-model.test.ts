import { describe, expect, test } from "bun:test";
import { getSearchQueryForDisclosure } from "./view-model";

describe("getSearchQueryForDisclosure", () => {
  test("closing the disclosure clears any hidden search query", () => {
    expect(getSearchQueryForDisclosure(false, "error")).toBe("");
  });

  test("open disclosure preserves the active query", () => {
    expect(getSearchQueryForDisclosure(true, "error")).toBe("error");
  });
});
