import { describe, expect, test } from "bun:test";
import { renderToStaticMarkup } from "react-dom/server";
import type { WorkspaceCandidate } from "./workspace-entry-state";
import { WorkspaceEntryOverlay } from "./WorkspaceEntryOverlay";

function recent(path: string): WorkspaceCandidate {
  return { type: "recent", path };
}

function picked(path: string): WorkspaceCandidate {
  return { type: "picked", path };
}

describe("WorkspaceEntryOverlay", () => {
  test("renders title, chooser, and disabled continue state", () => {
    const html = renderToStaticMarkup(
      <WorkspaceEntryOverlay
        selected={null}
        recentWorkspaces={[]}
        actionError={null}
        onChooseFolder={() => {}}
        onSelectRecent={() => {}}
        onContinue={() => {}}
      />,
    );

    expect(html).toContain("Choose a workspace");
    expect(html).toContain("Select a project directory");
    expect(html).toContain("Choose folder...");
    expect(html).toContain("Continue");
    expect(html).toContain("disabled");
  });

  test("renders the product name and selected candidate state", () => {
    const html = renderToStaticMarkup(
      <WorkspaceEntryOverlay
        selected={recent("/repo-a")}
        recentWorkspaces={["/repo-a"]}
        actionError={null}
        onChooseFolder={() => {}}
        onSelectRecent={() => {}}
        onContinue={() => {}}
      />,
    );

    expect(html).toContain(">Dimweave<");
    expect(html).toContain("/repo-a");
    expect(html).toContain("data-workspace-selected=\"true\"");
    expect(html).toContain("border-primary/35 bg-primary/8");
    expect(html).not.toContain("disabled=\"\"");
  });

  test("enables continue when a folder was picked manually", () => {
    const html = renderToStaticMarkup(
      <WorkspaceEntryOverlay
        selected={picked("/repo-b")}
        recentWorkspaces={[]}
        actionError={null}
        onChooseFolder={() => {}}
        onSelectRecent={() => {}}
        onContinue={() => {}}
      />,
    );

    expect(html).toContain("Selected workspace");
    expect(html).toContain("/repo-b");
    expect(html).not.toContain("disabled=\"\"");
  });
});
