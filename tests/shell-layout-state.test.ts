import { describe, expect, test } from "bun:test";
import {
  createShellLayoutState,
  getMountedShellPanes,
  resolveShellWorkspaceLabel,
  toggleShellNavItem,
} from "../src/components/shell-layout-state";

describe("createShellLayoutState", () => {
  test("defaults to the chat surface with no active sidebar item", () => {
    expect(createShellLayoutState()).toEqual({
      activeItem: null,
      sidebarPane: null,
      mainSurface: "chat",
    });
  });
});

describe("toggleShellNavItem", () => {
  test("opens logs as a main-surface replacement", () => {
    const next = toggleShellNavItem(createShellLayoutState(), "logs");

    expect(next).toEqual({
      activeItem: "logs",
      sidebarPane: null,
      mainSurface: "logs",
    });
  });

  test("closes logs and returns to chat when clicked again", () => {
    const openLogs = toggleShellNavItem(createShellLayoutState(), "logs");

    expect(toggleShellNavItem(openLogs, "logs")).toEqual({
      activeItem: null,
      sidebarPane: null,
      mainSurface: "chat",
    });
  });

  test("opens task as a left sidebar pane while keeping chat visible", () => {
    const next = toggleShellNavItem(createShellLayoutState(), "task");

    expect(next).toEqual({
      activeItem: "task",
      sidebarPane: "task",
      mainSurface: "chat",
    });
  });

  test("switches from a sidebar pane into logs and closes the drawer", () => {
    const withTask = toggleShellNavItem(createShellLayoutState(), "task");

    expect(toggleShellNavItem(withTask, "logs")).toEqual({
      activeItem: "logs",
      sidebarPane: null,
      mainSurface: "logs",
    });
  });
});

describe("getMountedShellPanes", () => {
  test("mounts the active pane the first time it is opened", () => {
    expect(getMountedShellPanes([], "agents")).toEqual(["agents"]);
  });

  test("retains mounted panes while the drawer is collapsed", () => {
    expect(getMountedShellPanes(["agents"], null)).toEqual(["agents"]);
  });

  test("keeps prior panes mounted when switching between sections", () => {
    expect(getMountedShellPanes(["task"], "approvals")).toEqual([
      "task",
      "approvals",
    ]);
  });
});

describe("resolveShellWorkspaceLabel", () => {
  test("prefers the active task workspace when present", () => {
    expect(resolveShellWorkspaceLabel("/Users/jason/Desktop/figma")).toBe(
      "~/Desktop/figma",
    );
  });

  test("does not fall back to provider session workspaces", () => {
    expect(resolveShellWorkspaceLabel(null)).toBe("No workspace selected");
  });

  test("returns a clear empty label when nothing is active", () => {
    expect(resolveShellWorkspaceLabel(null)).toBe("No workspace selected");
  });
});
