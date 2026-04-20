import { shortenPath } from "@/lib/utils";

export type ShellSidebarPane = "task" | "agents" | "approvals" | "bugs";
export type ShellNavItem = ShellSidebarPane | "logs";
export type ShellMainSurface = "chat" | "logs";

export interface ShellLayoutState {
  activeItem: ShellNavItem | null;
  sidebarPane: ShellSidebarPane | null;
  mainSurface: ShellMainSurface;
}

export const SHELL_LAYOUT_STORAGE_KEY = "dimweave:shell-layout";

const SHELL_NAV_ITEMS: readonly ShellNavItem[] = [
  "task",
  "agents",
  "approvals",
  "bugs",
  "logs",
];
const SHELL_MAIN_SURFACES: readonly ShellMainSurface[] = ["chat", "logs"];

export function createShellLayoutState(): ShellLayoutState {
  return {
    activeItem: null,
    sidebarPane: null,
    mainSurface: "chat",
  };
}

/// Load persisted shell layout from localStorage. Unknown fields fall back
/// to defaults rather than throwing — keeps the app booting even after a
/// schema change removed one of the valid enum values.
export function loadShellLayoutState(): ShellLayoutState {
  if (typeof window === "undefined") return createShellLayoutState();
  try {
    const raw = window.localStorage.getItem(SHELL_LAYOUT_STORAGE_KEY);
    if (!raw) return createShellLayoutState();
    const parsed = JSON.parse(raw) as Partial<ShellLayoutState>;
    const activeItem =
      parsed.activeItem && SHELL_NAV_ITEMS.includes(parsed.activeItem)
        ? parsed.activeItem
        : null;
    const sidebarPane: ShellSidebarPane | null =
      parsed.sidebarPane &&
      (["task", "agents", "approvals", "bugs"] as const).includes(
        parsed.sidebarPane,
      )
        ? parsed.sidebarPane
        : null;
    const mainSurface =
      parsed.mainSurface && SHELL_MAIN_SURFACES.includes(parsed.mainSurface)
        ? parsed.mainSurface
        : "chat";
    return { activeItem, sidebarPane, mainSurface };
  } catch {
    return createShellLayoutState();
  }
}

export function saveShellLayoutState(state: ShellLayoutState): void {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.setItem(
      SHELL_LAYOUT_STORAGE_KEY,
      JSON.stringify(state),
    );
  } catch {
    // Ignore quota / private-mode errors — persistence is best-effort.
  }
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

export function getMountedShellPanes(
  mountedPanes: ShellSidebarPane[],
  activePane: ShellSidebarPane | null,
): ShellSidebarPane[] {
  if (!activePane || mountedPanes.includes(activePane)) {
    return mountedPanes;
  }

  return [...mountedPanes, activePane];
}

export function resolveShellWorkspaceLabel(
  activeTaskWorkspace: string | null | undefined,
): string {
  const preferredWorkspace = activeTaskWorkspace?.trim();
  if (preferredWorkspace) {
    return shortenPath(preferredWorkspace);
  }

  return "No workspace selected";
}
