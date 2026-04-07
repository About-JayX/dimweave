import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ArtifactTimeline } from "./ArtifactTimeline";

describe("ArtifactTimeline", () => {
  test("renders artifacts as selectable buttons and shows the selected detail", () => {
    const html = renderToStaticMarkup(
      <ArtifactTimeline
        items={[
          {
            artifactId: "art_diff",
            taskId: "task_1",
            sessionId: "sess_coder_b",
            kind: "diff",
            title: "Patch v2",
            contentRef: "/tmp/patch.diff",
            createdAt: 200,
            sessionTitle: "Coder implementation",
          },
        ]}
        selectedArtifactId="art_diff"
        detail={{
          headline: "Patch v2",
          body: "diff --git a/file b/file",
          meta: "Preview truncated · /tmp/patch.diff",
          previewAvailable: true,
        }}
        detailLoading={false}
        detailError={null}
        onSelect={() => {}}
      />,
    );

    expect(html).toContain("Artifact detail");
    expect(html).toContain("diff --git");
    expect(html).toContain('type="button"');
    expect(html).not.toContain("Artifact Timeline"); // old heavy section label removed
  });

  test("renders empty state when no artifacts", () => {
    const html = renderToStaticMarkup(
      <ArtifactTimeline
        items={[]}
        selectedArtifactId={null}
        detail={null}
        detailLoading={false}
        detailError={null}
        onSelect={() => {}}
      />,
    );

    expect(html).toContain("No task artifacts captured yet");
    expect(html).not.toContain("Artifact Timeline");
  });
});
