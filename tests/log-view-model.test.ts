import { describe, expect, test } from "bun:test";
import { formatTerminalTimestamp } from "../src/components/MessagePanel/view-model";

describe("formatTerminalTimestamp", () => {
  test("delegates to a shared formatter instead of rebuilding locale logic in the row", () => {
    const formatter = {
      format(value: number) {
        return `ts:${value}`;
      },
    } as Intl.DateTimeFormat;

    expect(formatTerminalTimestamp(42, formatter)).toBe("ts:42");
  });
});
