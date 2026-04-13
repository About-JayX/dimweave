import { describe, expect, test } from "bun:test";
import {
  continueIntoSelectedWorkspace,
  loadRecentWorkspaces,
  pushRecentWorkspace,
  selectWorkspaceCandidate,
  type WorkspaceCandidate,
} from "./workspace-entry-state";

function picked(path: string): WorkspaceCandidate {
  return { type: "picked", path };
}

function recent(path: string): WorkspaceCandidate {
  return { type: "recent", path };
}

describe("selectWorkspaceCandidate", () => {
  test("replaces a previous recent selection when a folder is picked", () => {
    expect(selectWorkspaceCandidate(picked("/repo-b"), recent("/repo-a"))).toEqual(
      picked("/repo-b"),
    );
  });

  test("replaces a previous picked folder when a recent workspace is selected", () => {
    expect(selectWorkspaceCandidate(recent("/repo-a"), picked("/repo-b"))).toEqual(
      recent("/repo-a"),
    );
  });
});

describe("pushRecentWorkspace", () => {
  test("deduplicates and caps recent workspaces", () => {
    expect(
      pushRecentWorkspace(["/repo-a", "/repo-b", "/repo-a"], "/repo-c", 3),
    ).toEqual(["/repo-c", "/repo-a", "/repo-b"]);
  });
});

describe("loadRecentWorkspaces", () => {
  test("normalizes corrupted storage payloads safely", () => {
    expect(loadRecentWorkspaces("not-json")).toEqual([]);
    expect(loadRecentWorkspaces("{\"bad\":true}")).toEqual([]);
  });
});

describe("continueIntoSelectedWorkspace", () => {
  test("sets the selected workspace without creating a task", () => {
    const workspacesSet: string[] = [];

    const nextRecent = continueIntoSelectedWorkspace({
      selected: recent("/repo-a"),
      recentWorkspaces: ["/repo-b"],
      setSelectedWorkspace: (ws) => { workspacesSet.push(ws); },
    });

    expect(workspacesSet).toEqual(["/repo-a"]);
    expect(nextRecent).toEqual(["/repo-a", "/repo-b"]);
  });

  test("returns null when no workspace is selected", () => {
    const result = continueIntoSelectedWorkspace({
      selected: null,
      recentWorkspaces: [],
      setSelectedWorkspace: () => {},
    });
    expect(result).toBeNull();
  });
});
