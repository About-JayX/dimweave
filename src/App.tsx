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
  loadShellLayoutState,
  resolveShellWorkspaceLabel,
  saveShellLayoutState,
  toggleShellNavItem,
} from "./components/shell-layout-state";
import { useBridgeStore } from "./stores/bridge-store";
import {
  selectPermissionPromptCount,
  selectTerminalLineCount,
  selectTotalMessageCount,
  selectUiErrorCount,
} from "./stores/bridge-store/selectors";
import { ErrorLogDialog } from "./components/ErrorLogDialog";
import { useFeishuProjectStore } from "./stores/feishu-project-store";
import { activeItemCount } from "./components/BugInboxPanel/view-model";
import { useTaskStore } from "./stores/task-store";
import { selectActiveTask } from "./stores/task-store/selectors";
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
  const [shellLayout, setShellLayout] = useState(loadShellLayoutState);
  useEffect(() => {
    saveShellLayoutState(shellLayout);
  }, [shellLayout]);
  const [recentWorkspaces, setRecentWorkspaces] = useState<string[]>([]);
  const [recentWorkspacesLoaded, setRecentWorkspacesLoaded] = useState(false);
  const [selectedWorkspace, setSelectedWorkspace] =
    useState<WorkspaceCandidate | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [workspaceActionError, setWorkspaceActionError] = useState<
    string | null
  >(null);
  const activeTask = useTaskStore(selectActiveTask);
  const storeSelectedWorkspace = useTaskStore((s) => s.selectedWorkspace);
  const storeSetSelectedWorkspace = useTaskStore((s) => s.setSelectedWorkspace);
  const bootstrapComplete = useTaskStore((s) => s.bootstrapComplete);
  const bootstrapError = useTaskStore((s) => s.bootstrapError);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);
  const workspaceLabel = resolveShellWorkspaceLabel(
    activeTask?.projectRoot ?? storeSelectedWorkspace,
  );

  const approvalCount = useBridgeStore(selectPermissionPromptCount);
  const bugItems = useFeishuProjectStore((s) => s.items);
  const bugCount = useMemo(() => activeItemCount(bugItems), [bugItems]);
  const logLineCount = useBridgeStore(selectTerminalLineCount);
  const uiErrorCount = useBridgeStore(selectUiErrorCount);
  const uiErrors = useBridgeStore((s) => s.uiErrors);
  const clearUiErrors = useBridgeStore((s) => s.clearUiErrors);
  const runtimeHealth = useBridgeStore((s) => s.runtimeHealth);
  const clearMessages = useBridgeStore((s) => s.clearMessages);
  // Top-bar indicator only needs a count, not the full list — count is
  // O(bucket-count) via sum of bucket sizes. Prior chatMessages filter
  // over the flat array was O(total-message-count) each render.
  const messageCount = useBridgeStore(selectTotalMessageCount);
  const [errorLogOpen, setErrorLogOpen] = useState(false);

  useEffect(() => {
    if (!bootstrapComplete || recentWorkspacesLoaded) {
      return;
    }

    try {
      setRecentWorkspaces(
        loadRecentWorkspaces(
          localStorage.getItem(RECENT_WORKSPACES_STORAGE_KEY),
        ),
      );
    } catch {
      setRecentWorkspaces([]);
    }

    setRecentWorkspacesLoaded(true);
  }, [bootstrapComplete, recentWorkspacesLoaded]);

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
      setSelectedWorkspace((current) =>
        selectWorkspaceCandidate(next, current),
      );
    },
    [],
  );

  const handleContinueIntoWorkspace = useCallback(() => {
    setWorkspaceActionError(null);
    const nextRecent = continueIntoSelectedWorkspace({
      selected: selectedWorkspace,
      recentWorkspaces,
      setSelectedWorkspace: storeSetSelectedWorkspace,
    });
    if (!nextRecent) return;
    setRecentWorkspaces(nextRecent);
    try {
      localStorage.setItem(
        RECENT_WORKSPACES_STORAGE_KEY,
        JSON.stringify(nextRecent),
      );
    } catch {}
  }, [recentWorkspaces, selectedWorkspace, storeSetSelectedWorkspace]);

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
          bugCount={bugCount}
          messageCount={messageCount}
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
            currentWorkspace={activeTask?.projectRoot ?? storeSelectedWorkspace}
            selectedWorkspace={selectedWorkspace}
            recentWorkspaces={recentWorkspaces}
            workspaceActionError={workspaceActionError}
            surfaceMode={shellLayout.mainSurface}
            logLineCount={logLineCount}
            errorCount={uiErrorCount}
            onErrorBadgeClick={() => setErrorLogOpen(true)}
            onClear={clearMessages}
            onSearchToggle={() => setSearchOpen((v) => !v)}
            onChooseWorkspace={handleChooseWorkspace}
            onSelectRecentWorkspace={handleSelectRecentWorkspace}
            onContinueIntoWorkspace={handleContinueIntoWorkspace}
          />
          <MessagePanel
            surfaceMode={shellLayout.mainSurface}
            searchOpen={searchOpen}
            onSearchClose={() => setSearchOpen(false)}
          />
          {shellLayout.mainSurface === "chat" && <ReplyInput />}
        </main>
      </div>
      <ErrorLogDialog
        open={errorLogOpen}
        errors={uiErrors}
        onClose={() => setErrorLogOpen(false)}
        onClear={clearUiErrors}
      />
      {!storeSelectedWorkspace && (
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
