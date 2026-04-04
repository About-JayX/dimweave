import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { PermissionQueueView } from "./PermissionQueue";

describe("PermissionQueue", () => {
  test("renders the latest inline approval error above the pending queue", () => {
    const html = renderToStaticMarkup(
      <PermissionQueueView
        prompts={[
          {
            agent: "claude",
            requestId: "req_1",
            toolName: "Bash",
            description: "Run ls",
            inputPreview: "ls",
            createdAt: 100,
          },
        ]}
        error={{
          requestId: "req_1",
          message: "Permission approval did not reach the daemon",
        }}
        onResolve={async () => {}}
      />,
    );

    expect(html).toContain("Last action failed");
    expect(html).toContain("Permission approval did not reach the daemon");
  });
});
