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

  test("shows the search toggle in chat mode when onSearchToggle is provided", () => {
    const html = renderToStaticMarkup(
      <ShellTopBar
        workspaceLabel="~/project"
        currentWorkspace="/Users/jason/project"
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
        onSearchToggle={() => {}}
      />,
    );

    expect(html).toContain('aria-label="Search messages"');
  });

  test("error badge not rendered when count is zero in logs mode", () => {
    const html = renderToStaticMarkup(
      <ShellTopBar
        workspaceLabel="/repo" currentWorkspace="/repo"
        selectedWorkspace={null} recentWorkspaces={[]}
        workspaceActionError={null} surfaceMode="logs"
        logLineCount={5} errorCount={0} onClear={() => {}}
        onChooseWorkspace={() => {}} onSelectRecentWorkspace={() => {}}
        onContinueIntoWorkspace={() => {}}
      />,
    );
    // bg-destructive/8 is specific to the error badge styling
    expect(html).not.toContain("bg-destructive/8");
  });

  test("error badge is a clickable button with onErrorBadgeClick", () => {
    const html = renderToStaticMarkup(
      <ShellTopBar
        workspaceLabel="/repo" currentWorkspace="/repo"
        selectedWorkspace={null} recentWorkspaces={[]}
        workspaceActionError={null} surfaceMode="logs"
        logLineCount={5} errorCount={3} onClear={() => {}}
        onChooseWorkspace={() => {}} onSelectRecentWorkspace={() => {}}
        onContinueIntoWorkspace={() => {}} onErrorBadgeClick={() => {}}
      />,
    );
    expect(html).toContain("<button");
    expect(html).toContain("3");
  });
});
