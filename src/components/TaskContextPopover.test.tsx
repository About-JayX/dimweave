import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskContextPopover } from "./TaskContextPopover";

describe("TaskContextPopover", () => {
  test("renders a minimal empty state when no task is active", () => {
    const html = renderToStaticMarkup(
      <TaskContextPopover
        open
        onClose={() => {}}
        task={null}
        sessionCount={0}
        artifactCount={0}
      />,
    );

    expect(html).toContain("data-task-context-drawer=\"true\"");
    expect(html).toContain("No active task");
    expect(html).not.toContain("bg-background/28");
    expect(html).not.toContain(
      "The conversation timeline stays live, but task context and review state will appear here once a task is active.",
    );
  });

  test("renders task details when an active task exists", () => {
    const html = renderToStaticMarkup(
      <TaskContextPopover
        open
        onClose={() => {}}
        task={{
          taskId: "task-1",
          title: "Refine shell header",
          workspaceRoot: "/Users/jason/Desktop/figma",
          status: "active",
          reviewStatus: null,
          createdAt: 1,
          updatedAt: 1,
        }}
        sessionCount={3}
        artifactCount={5}
      />,
    );

    expect(html).toContain("Refine shell header");
    expect(html).toContain("3 sessions");
    expect(html).toContain("5 artifacts");
  });
});
