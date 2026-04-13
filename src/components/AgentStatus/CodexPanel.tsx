import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { CyberSelect } from "@/components/ui/cyber-select";
import { useBridgeStore } from "@/stores/bridge-store";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { useTaskStore } from "@/stores/task-store";
import type { ProviderSessionInfo } from "@/types";
import { AuthActions } from "./AuthActions";
import { CodexHeader } from "./CodexHeader";
import { CodexUsageSection, type CodexUsageData } from "./CodexUsageSection";
import { CodexConfigRows } from "./CodexConfigRows";
import {
  buildProviderHistoryOptions,
  findProviderHistoryEntry,
  formatProviderConnectionLabel,
  NEW_PROVIDER_SESSION_VALUE,
  resolveProviderHistoryAction,
  resolveProviderHistoryWorkspace,
} from "./provider-session-view-model";
import {
  buildCodexLaunchConfig,
  canConnectCodex,
  CODEX_CONNECT_READY_TIMEOUT_MS,
  getCodexConnectTimeoutMessage,
  getDefaultReasoningEffort,
  hasCodexConnectTimedOut,
} from "./codex-launch-config";
import { ChevronDown, ChevronUp, SlidersHorizontal } from "lucide-react";
import {
  makeProviderHistoryErrorSelector,
  makeProviderHistoryLoadingSelector,
  makeProviderHistorySelector,
  selectActiveTask,
} from "@/stores/task-store/selectors";

interface CodexPanelProps {
  codexTuiRunning: boolean;
  stopCodexTui: () => void;
  profile: { name?: string; planType?: string } | null;
  usage: CodexUsageData | null;
  refreshing: boolean;
  refreshUsage: () => void;
  providerSession?: ProviderSessionInfo;
  workspace?: string;
  draftMode?: boolean;
}

export function CodexPanel({
  codexTuiRunning,
  stopCodexTui,
  profile,
  usage,
  refreshing,
  refreshUsage,
  providerSession,
  workspace,
  draftMode = false,
}: CodexPanelProps) {
  const models = useCodexAccountStore((s) => s.models);
  const fetchModels = useCodexAccountStore((s) => s.fetchModels);
  const applyConfig = useBridgeStore((s) => s.applyConfig);
  const codexRole = useBridgeStore((s) => s.codexRole);
  const activeTask = useTaskStore(selectActiveTask);
  const selectedWorkspace = useTaskStore((s) => s.selectedWorkspace);
  const fetchProviderHistory = useTaskStore((s) => s.fetchProviderHistory);
  const resumeSession = useTaskStore((s) => s.resumeSession);

  const [selectedModel, setSelectedModel] = useState("");
  const [selectedReasoning, setSelectedReasoning] = useState("");
  const [selectedHistoryId, setSelectedHistoryId] = useState(
    NEW_PROVIDER_SESSION_VALUE,
  );
  const effectiveCwd = useMemo(
    () =>
      resolveProviderHistoryWorkspace(
        workspace ?? activeTask?.workspaceRoot ?? selectedWorkspace,
      ),
    [workspace, activeTask?.workspaceRoot, selectedWorkspace],
  );

  const [connecting, setConnecting] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [connectStartedAt, setConnectStartedAt] = useState<number | null>(null);
  const [showAdvanced, setShowAdvanced] = useState(false);
  const locked = codexTuiRunning;
  const prevRunningRef = useRef(codexTuiRunning);
  const [justConnected, setJustConnected] = useState(false);
  const selectWorkspaceHistory = useMemo(
    () => makeProviderHistorySelector(effectiveCwd),
    [effectiveCwd],
  );
  const selectWorkspaceHistoryLoading = useMemo(
    () => makeProviderHistoryLoadingSelector(effectiveCwd),
    [effectiveCwd],
  );
  const selectWorkspaceHistoryError = useMemo(
    () => makeProviderHistoryErrorSelector(effectiveCwd),
    [effectiveCwd],
  );
  const workspaceHistory = useTaskStore(selectWorkspaceHistory);
  const historyLoading = useTaskStore(selectWorkspaceHistoryLoading);
  const historyError = useTaskStore(selectWorkspaceHistoryError);

  useEffect(() => {
    if (codexTuiRunning && !prevRunningRef.current) {
      setConnecting(false);
      setActionError(null);
      setConnectStartedAt(null);
      setJustConnected(true);
      setShowAdvanced(false);
      const t = setTimeout(() => setJustConnected(false), 600);
      return () => clearTimeout(t);
    }
    if (!codexTuiRunning && !connecting) {
      setConnectStartedAt(null);
    }
    prevRunningRef.current = codexTuiRunning;
  }, [codexTuiRunning, connecting]);

  useEffect(() => {
    if (!connecting || codexTuiRunning || connectStartedAt === null) return;

    const elapsed = Date.now() - connectStartedAt;
    if (
      hasCodexConnectTimedOut({
        connecting,
        running: codexTuiRunning,
        connectStartedAt,
      })
    ) {
      setConnecting(false);
      setConnectStartedAt(null);
      setActionError(getCodexConnectTimeoutMessage());
      return;
    }

    const timer = setTimeout(() => {
      setConnecting(false);
      setConnectStartedAt(null);
      setActionError(getCodexConnectTimeoutMessage());
    }, CODEX_CONNECT_READY_TIMEOUT_MS - elapsed);

    return () => clearTimeout(timer);
  }, [codexTuiRunning, connectStartedAt, connecting]);

  useEffect(() => {
    fetchModels();
  }, [fetchModels]);

  useEffect(() => {
    if (models.length > 0 && !selectedModel) {
      const first = models[0];
      setSelectedModel(first.slug);
      setSelectedReasoning(getDefaultReasoningEffort(first));
    }
  }, [models, selectedModel]);

  const currentModel = useMemo(
    () => models.find((m) => m.slug === selectedModel),
    [models, selectedModel],
  );
  const reasoningOptions = useMemo(
    () => currentModel?.reasoningLevels ?? [],
    [currentModel],
  );
  const modelSelectOptions = useMemo(
    () => models.map((m) => ({ value: m.slug, label: m.displayName })),
    [models],
  );
  const reasoningSelectOptions = useMemo(
    () => reasoningOptions.map((r) => ({ value: r.effort, label: r.effort })),
    [reasoningOptions],
  );
  const historyOptions = useMemo(
    () => buildProviderHistoryOptions("codex", workspaceHistory),
    [workspaceHistory],
  );
  const selectedHistory = useMemo(
    () =>
      findProviderHistoryEntry("codex", workspaceHistory, selectedHistoryId),
    [selectedHistoryId, workspaceHistory],
  );
  const connectionLabel = useMemo(
    () => formatProviderConnectionLabel(providerSession),
    [providerSession],
  );

  const handleModelChange = useCallback(
    (slug: string) => {
      setSelectedModel(slug);
      const m = models.find((x) => x.slug === slug);
      if (m) {
        setSelectedReasoning(getDefaultReasoningEffort(m));
      }
    },
    [models],
  );

  useEffect(() => {
    if (!effectiveCwd) return;
    void fetchProviderHistory(effectiveCwd).catch(() => {});
  }, [effectiveCwd, fetchProviderHistory]);

  useEffect(() => {
    if (selectedHistoryId !== NEW_PROVIDER_SESSION_VALUE && !selectedHistory) {
      setSelectedHistoryId(NEW_PROVIDER_SESSION_VALUE);
    }
  }, [selectedHistory, selectedHistoryId]);

  const handleConnect = useCallback(async () => {
    setConnecting(true);
    setActionError(null);
    setConnectStartedAt(Date.now());
    try {
      const action = resolveProviderHistoryAction(selectedHistory);
      if (action.kind === "resumeNormalized") {
        await resumeSession(action.sessionId);
      } else {
        await applyConfig(
          buildCodexLaunchConfig({
            model: selectedModel,
            reasoningEffort: selectedReasoning,
            cwd: effectiveCwd,
            resumeThreadId:
              action.kind === "resumeExternal" ? action.externalId : undefined,
            taskId: activeTask?.taskId,
          }),
        );
      }
    } catch (error) {
      setConnecting(false);
      setConnectStartedAt(null);
      setActionError(error instanceof Error ? error.message : String(error));
    }
  }, [
    applyConfig,
    resumeSession,
    selectedModel,
    selectedReasoning,
    effectiveCwd,
    selectedHistory,
  ]);

  const summaryChips = useMemo(
    () => [
      effectiveCwd
        ? effectiveCwd.split("/").pop() || effectiveCwd
        : "Workspace required",
      selectedModel || "Select model",
      selectedReasoning || "Default reasoning",
      selectedHistory
        ? `Resume ${selectedHistory.externalId.slice(0, 12)}`
        : "New session",
    ],
    [effectiveCwd, selectedHistory, selectedModel, selectedReasoning],
  );

  return (
    <div
      className={cn(
        "rounded-2xl border bg-card px-4 py-3 card-depth transition-colors",
        codexTuiRunning
          ? "border-codex/40 glow-codex-subtle border-glow-codex"
          : "border-input hover:border-input/80",
        justConnected && "card-connect-anim",
      )}
    >
      <CodexHeader
        running={codexTuiRunning}
        connectionLabel={connectionLabel}
      />

      {locked && usage && (
        <CodexUsageSection
          usage={usage}
          refreshing={refreshing}
          refreshUsage={refreshUsage}
        />
      )}

      <div className="mt-3 flex flex-wrap gap-1.5">
        {summaryChips.map((chip) => (
          <span
            key={chip}
            className="rounded-full border border-border/45 bg-background/35 px-2 py-0.5 text-[10px] text-muted-foreground"
          >
            {chip}
          </span>
        ))}
      </div>

      <div className="mt-3 flex gap-2">
        {locked ? (
          <Button
            size="sm"
            variant="destructive"
            className="flex-1 rounded-full"
            onClick={stopCodexTui}
          >
            Disconnect
          </Button>
        ) : (
          <Button
            className={cn(
              "flex-1 rounded-full",
              connecting
                ? "bg-codex/20 text-codex border-codex/30 cursor-wait"
                : "bg-codex/15 text-codex border-codex/25 hover:bg-codex/25",
            )}
            size="sm"
            disabled={
              draftMode ||
              (!connecting &&
                !canConnectCodex({
                  cwd: effectiveCwd,
                  role: codexRole,
                  connecting: false,
                  running: !!codexTuiRunning,
                }))
            }
            onClick={handleConnect}
          >
            {connecting ? (
              <span className="flex items-center gap-2">
                <span className="size-3 border-2 border-codex/30 border-t-codex rounded-full radius-keep animate-spin" />
                Connecting…
              </span>
            ) : (
              "Connect"
            )}
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          className="shrink-0 text-muted-foreground"
          onClick={() => setShowAdvanced((open) => !open)}
        >
          <SlidersHorizontal className="size-3.5" />
          {showAdvanced ? (
            <ChevronUp className="size-3.5" />
          ) : (
            <ChevronDown className="size-3.5" />
          )}
        </Button>
      </div>

      {!locked && <AuthActions />}

      {showAdvanced && (
        <div className="mt-3 rounded-xl border border-border/35 bg-background/30 px-3 py-3">
          <CodexConfigRows
            locked={locked}
            profile={profile}
            models={models}
            selectedModel={selectedModel}
            modelSelectOptions={modelSelectOptions}
            handleModelChange={handleModelChange}
            reasoningOptions={reasoningOptions}
            selectedReasoning={selectedReasoning}
            setSelectedReasoning={setSelectedReasoning}
            reasoningSelectOptions={reasoningSelectOptions}
          />

          <div className="mt-2 flex items-center gap-2">
            <span className="shrink-0 text-[10px] text-muted-foreground">
              History
            </span>
            <CyberSelect
              variant="history"
              value={selectedHistoryId}
              options={historyOptions}
              onChange={setSelectedHistoryId}
              disabled={locked || !effectiveCwd || connecting}
              placeholder="New session"
            />
          </div>

          {!locked && effectiveCwd && historyLoading && (
            <div className="mt-1.5 text-[11px] text-muted-foreground">
              Loading Codex history...
            </div>
          )}
          {!locked && effectiveCwd && historyError && (
            <div className="mt-1.5 text-[11px] text-destructive">
              {historyError}
            </div>
          )}
        </div>
      )}

      {actionError && (
        <div className="mt-2 text-[11px] text-destructive">{actionError}</div>
      )}

      {!!codexTuiRunning && (
        <div className="mt-1.5 text-[11px] text-muted-foreground">
          Codex app-server is starting...
        </div>
      )}
    </div>
  );
}
