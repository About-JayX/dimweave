import { describe, expect, test } from "bun:test";
import {
  createShellLayoutState,
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
