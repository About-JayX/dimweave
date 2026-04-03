import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import { WorkspaceSwitcher } from "./WorkspaceSwitcher";

describe("WorkspaceSwitcher", () => {
  test("shows choose workspace label when no active task exists", () => {
    const html = renderToStaticMarkup(
      <WorkspaceSwitcher
        workspaceLabel="No workspace selected"
        currentWorkspace={null}
        selected={null}
        recentWorkspaces={["/repo-a"]}
        actionError={null}
        onChooseFolder={() => {}}
        onSelectRecent={() => {}}
        onContinue={() => {}}
        defaultOpen
      />,
    );

    expect(html).toContain("Choose workspace");
    expect(html).toContain("Recent workspaces");
  });

  test("renders the active workspace label and selected candidate state", () => {
    const html = renderToStaticMarkup(
      <WorkspaceSwitcher
        workspaceLabel="~/repo-a"
        currentWorkspace="/repo-a"
        selected={{ type: "recent", path: "/repo-b" }}
        recentWorkspaces={["/repo-a", "/repo-b"]}
        actionError={null}
        onChooseFolder={() => {}}
        onSelectRecent={() => {}}
        onContinue={() => {}}
        defaultOpen
      />,
    );

    expect(html).toContain("~/repo-a");
    expect(html).toContain("/repo-b");
    expect(html).toContain('data-workspace-selected="true"');
  });
});
