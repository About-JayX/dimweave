import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";

describe("MessageBubble", () => {
  test("renders image attachments as zoomable buttons", async () => {
    const { MessageBubbleView } = await import("./MessageBubble");
    const html = renderToStaticMarkup(
      <MessageBubbleView
        msg={{
          id: "msg_1",
          from: "claude",
          to: "user",
          content: "See attached",
          timestamp: 1,
          attachments: [
            {
              filePath: "/tmp/review.png",
              fileName: "review.png",
              isImage: true,
            },
          ],
        }}
        onOpenImage={() => {}}
      />,
    );

    expect(html).toContain('type="button"');
    expect(html).toContain("Open image review.png");
  });

  test("renders a lightbox view with image metadata", async () => {
    const { MessageImageLightbox } = await import("./MessageBubble");
    const html = renderToStaticMarkup(
      <MessageImageLightbox
        attachment={{
          filePath: "/tmp/review.png",
          fileName: "review.png",
          isImage: true,
        }}
        onClose={() => {}}
      />,
    );

    expect(html).toContain("Image preview");
    expect(html).toContain("review.png");
    expect(html).toContain("Close preview");
    expect(html).toContain('role="dialog"');
    expect(html).toContain('aria-modal="true"');
  });
});
