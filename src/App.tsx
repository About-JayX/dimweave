import { useCallback, useEffect, useMemo, useState } from "react";
import { MessagePanel } from "./components/MessagePanel";
import { ReplyInput } from "./components/ReplyInput";
import { ShellContextBar } from "./components/ShellContextBar";
import { ShellTopBar } from "./components/ShellTopBar";
import { TaskContextPopover } from "./components/TaskContextPopover";
import { AppBootstrapGate } from "./components/AppBootstrapGate";
import { WorkspaceEntryOverlay } from "./components/WorkspaceEntryOverlay";
import {
  closeShellSidebar,
  createShellLayoutState,
  resolveShellWorkspaceLabel,
  toggleShellNavItem,
} from "./components/shell-layout-state";
import { useBridgeStore } from "./stores/bridge-store";
import {
  selectMessages,
  selectPermissionPromptCount,
} from "./stores/bridge-store/selectors";
import { useTaskStore } from "./stores/task-store";
import { selectActiveTask } from "./stores/task-store/selectors";
import { filterRenderableChatMessages } from "./components/MessagePanel/view-model";
import { useTheme } from "./components/use-theme";
import { useBorderRadius } from "./components/use-border-radius";
import { useCodexAccountStore } from "./stores/codex-account-store";
import {
  continueIntoSelectedWorkspace,
  loadRecentWorkspaces,
  selectWorkspaceCandidate,
  type WorkspaceCandidate,
} from "./components/workspace-entry-state";

const RECENT_WORKSPACES_STORAGE_KEY = "dimweave:recent-workspaces";

export default function App() {
  const theme = useTheme();
  const radius = useBorderRadius();
  const [shellLayout, setShellLayout] = useState(createShellLayoutState);
  const [recentWorkspaces, setRecentWorkspaces] = useState<string[]>([]);
  const [recentWorkspacesLoaded, setRecentWorkspacesLoaded] = useState(false);
  const [selectedWorkspace, setSelectedWorkspace] =
    useState<WorkspaceCandidate | null>(null);
  const [workspaceActionError, setWorkspaceActionError] = useState<string | null>(
    null,
  );
  const activeTask = useTaskStore(selectActiveTask);
  const bootstrapComplete = useTaskStore((s) => s.bootstrapComplete);
  const bootstrapError = useTaskStore((s) => s.bootstrapError);
  const startWorkspaceTask = useTaskStore((s) => s.startWorkspaceTask);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);
  const workspaceLabel = resolveShellWorkspaceLabel(activeTask?.workspaceRoot);

  const messages = useBridgeStore(selectMessages);
  const approvalCount = useBridgeStore(selectPermissionPromptCount);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);
  const runtimeHealth = useBridgeStore((s) => s.runtimeHealth);
  const clearMessages = useBridgeStore((s) => s.clearMessages);
  const chatMessages = useMemo(
    () => filterRenderableChatMessages(messages),
    [messages],
  );
  const errorCount = useMemo(
    () => allTerminalLines.filter((l) => l.kind === "error").length,
    [allTerminalLines],
  );

  useEffect(() => {
    if (!bootstrapComplete || recentWorkspacesLoaded) {
      return;
    }

    try {
      setRecentWorkspaces(
        loadRecentWorkspaces(localStorage.getItem(RECENT_WORKSPACES_STORAGE_KEY)),
      );
    } catch {
      setRecentWorkspaces([]);
    }

    setRecentWorkspacesLoaded(true);
  }, [bootstrapComplete, recentWorkspacesLoaded]);

  useEffect(() => {
    if (!activeTask) {
      return;
    }
    setSelectedWorkspace(null);
    setWorkspaceActionError(null);
  }, [activeTask]);

  const handleChooseWorkspace = useCallback(async () => {
    setWorkspaceActionError(null);
    const picked = await pickDirectory();
    if (!picked) {
      return;
    }
    setSelectedWorkspace((current) =>
      selectWorkspaceCandidate({ type: "picked", path: picked }, current),
    );
  }, [pickDirectory]);

  const handleSelectRecentWorkspace = useCallback(
    (next: WorkspaceCandidate) => {
      setWorkspaceActionError(null);
      setSelectedWorkspace((current) => selectWorkspaceCandidate(next, current));
    },
    [],
  );

  const handleContinueIntoWorkspace = useCallback(async () => {
    try {
      setWorkspaceActionError(null);
      const nextRecent = await continueIntoSelectedWorkspace({
        selected: selectedWorkspace,
        recentWorkspaces,
        startWorkspaceTask,
      });
      if (!nextRecent) {
        return;
      }
      setRecentWorkspaces(nextRecent);
      try {
        localStorage.setItem(
          RECENT_WORKSPACES_STORAGE_KEY,
          JSON.stringify(nextRecent),
        );
      } catch {}
    } catch (error) {
      setWorkspaceActionError(
        error instanceof Error ? error.message : String(error),
      );
    }
  }, [recentWorkspaces, selectedWorkspace, startWorkspaceTask]);

  if (bootstrapError) {
    return <AppBootstrapGate status="error" message={bootstrapError} />;
  }

  if (!bootstrapComplete) {
    return <AppBootstrapGate status="loading" />;
  }

  return (
    <div className="relative flex h-screen flex-col overflow-hidden bg-background font-sans text-foreground">
      <div className="flex flex-1 min-h-0">
        <ShellContextBar
          activeItem={shellLayout.activeItem}
          approvalCount={approvalCount}
          messageCount={chatMessages.length}
          runtimeHealth={runtimeHealth}
          themeMode={theme.mode}
          radiusMode={radius.mode}
          onToggle={(item) =>
            setShellLayout((current) => toggleShellNavItem(current, item))
          }
          onThemeChange={theme.setMode}
          onRadiusToggle={radius.toggle}
        />
        <TaskContextPopover
          activePane={shellLayout.sidebarPane}
          onClose={() =>
            setShellLayout((current) => closeShellSidebar(current))
          }
          task={activeTask}
        />
        <main className="flex min-w-0 flex-1 flex-col animate-in fade-in duration-500">
          <ShellTopBar
            workspaceLabel={workspaceLabel}
            currentWorkspace={activeTask?.workspaceRoot ?? null}
            selectedWorkspace={selectedWorkspace}
            recentWorkspaces={recentWorkspaces}
            workspaceActionError={workspaceActionError}
            surfaceMode={shellLayout.mainSurface}
            logLineCount={allTerminalLines.length}
            errorCount={errorCount}
            onClear={clearMessages}
            onChooseWorkspace={handleChooseWorkspace}
            onSelectRecentWorkspace={handleSelectRecentWorkspace}
            onContinueIntoWorkspace={handleContinueIntoWorkspace}
          />
          <MessagePanel surfaceMode={shellLayout.mainSurface} />
          {shellLayout.mainSurface === "chat" && <ReplyInput />}
        </main>
      </div>
      {!activeTask && (
        // Launch always re-enters through explicit workspace selection.
        <WorkspaceEntryOverlay
          selected={selectedWorkspace}
          recentWorkspaces={recentWorkspaces}
          actionError={workspaceActionError}
          onChooseFolder={handleChooseWorkspace}
          onSelectRecent={handleSelectRecentWorkspace}
          onContinue={handleContinueIntoWorkspace}
        />
      )}
    </div>
  );
}
