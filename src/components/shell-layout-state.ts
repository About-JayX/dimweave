export type ShellSidebarPane = "task" | "agents" | "approvals";
export type ShellNavItem = ShellSidebarPane | "logs";
export type ShellMainSurface = "chat" | "logs";

export interface ShellLayoutState {
  activeItem: ShellNavItem | null;
  sidebarPane: ShellSidebarPane | null;
  mainSurface: ShellMainSurface;
}

export function createShellLayoutState(): ShellLayoutState {
  return {
    activeItem: null,
    sidebarPane: null,
    mainSurface: "chat",
  };
}

export function toggleShellNavItem(
  state: ShellLayoutState,
  item: ShellNavItem,
): ShellLayoutState {
  if (item === "logs") {
    if (state.activeItem === "logs") {
      return createShellLayoutState();
    }

    return {
      activeItem: "logs",
      sidebarPane: null,
      mainSurface: "logs",
    };
  }

  if (state.activeItem === item) {
    return createShellLayoutState();
  }

  return {
    activeItem: item,
    sidebarPane: item,
    mainSurface: "chat",
  };
}

export function closeShellSidebar(state: ShellLayoutState): ShellLayoutState {
  if (!state.sidebarPane) {
    return state;
  }

  return {
    activeItem: null,
    sidebarPane: null,
    mainSurface: state.mainSurface,
  };
}
