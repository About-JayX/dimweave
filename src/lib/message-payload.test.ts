import { describe, expect, test } from "bun:test";
import { filterRenderableChatMessages } from "@/components/MessagePanel/view-model";
import type { Attachment, BridgeMessage } from "@/types";
import { hasMessagePayload } from "./message-payload";

const fileAttachment: Attachment = {
  filePath: "/tmp/spec.md",
  fileName: "spec.md",
};

function buildMessage(
  overrides: Partial<BridgeMessage> = {},
): BridgeMessage {
  return {
    id: "msg-1",
    source: { kind: "user" },
    target: { kind: "role", role: "coder" },
    message: "",
    timestamp: 1,
    ...overrides,
  };
}

describe("message payload helpers", () => {
  test("attachments-only input still counts as sendable payload", () => {
    expect(hasMessagePayload("   \n\t", [fileAttachment])).toBe(true);
    expect(hasMessagePayload("   \n\t", [])).toBe(false);
  });

  test("chat filtering keeps attachments-only messages", () => {
    const rendered = filterRenderableChatMessages([
      buildMessage({ attachments: [fileAttachment] }),
      buildMessage({ id: "msg-2", message: "   \n\t" }),
    ]);

    expect(rendered).toHaveLength(1);
    expect(rendered[0]?.attachments?.[0]?.fileName).toBe("spec.md");
  });
});
