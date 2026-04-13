import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { TaskSetupDialog } from "./TaskSetupDialog";

describe("TaskSetupDialog", () => {
  test("renders create-mode dialog with provider selectors", () => {
    const html = renderToStaticMarkup(
      createElement(TaskSetupDialog, {
        mode: "create",
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit: () => {},
      }),
    );
    expect(html).toContain("New Task");
    expect(html).toContain("Lead provider");
    expect(html).toContain("Coder provider");
    expect(html).toContain("Create");
  });

  test("renders edit-mode dialog pre-filled with current config", () => {
    const html = renderToStaticMarkup(
      createElement(TaskSetupDialog, {
        mode: "edit",
        workspace: "/repo",
        open: true,
        onOpenChange: () => {},
        onSubmit: () => {},
        initialTitle: "Fix routing bug",
        initialLeadProvider: "codex",
        initialCoderProvider: "claude",
      }),
    );
    expect(html).toContain("Edit Task");
    expect(html).toContain("Fix routing bug");
    expect(html).toContain("Save");
  });

  test("does not render content when closed", () => {
    const html = renderToStaticMarkup(
      createElement(TaskSetupDialog, {
        mode: "create",
        workspace: "/repo",
        open: false,
        onOpenChange: () => {},
        onSubmit: () => {},
      }),
    );
    expect(html).not.toContain("New Task");
    expect(html).not.toContain("Lead provider");
  });
});
