import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { ShellTopBar } from "./ShellTopBar";

describe("ShellTopBar", () => {
  test("renders product title and workspace switcher", () => {
    const html = renderToStaticMarkup(
      <ShellTopBar
        workspaceLabel="~/Desktop/figma"
        currentWorkspace="/Users/jason/Desktop/figma"
        selectedWorkspace={null}
        recentWorkspaces={["/Users/jason/Desktop/figma"]}
        workspaceActionError={null}
        onChooseWorkspace={() => {}}
        onSelectRecentWorkspace={() => {}}
        onContinueIntoWorkspace={() => {}}
        surfaceMode="chat"
        logLineCount={0}
        errorCount={0}
        onClear={() => {}}
      />,
    );

    expect(html).toContain("Dimweave");
    expect(html).toContain("~/Desktop/figma");
  });

  test("shows choose workspace when no task is active", () => {
    const html = renderToStaticMarkup(
      <ShellTopBar
        workspaceLabel="No workspace selected"
        currentWorkspace={null}
        selectedWorkspace={null}
        recentWorkspaces={[]}
        workspaceActionError={null}
        onChooseWorkspace={() => {}}
        onSelectRecentWorkspace={() => {}}
        onContinueIntoWorkspace={() => {}}
        surfaceMode="chat"
        logLineCount={0}
        errorCount={0}
        onClear={() => {}}
      />,
    );

    expect(html).toContain("Choose workspace");
  });
});
