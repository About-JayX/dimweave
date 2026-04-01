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
  resolveProviderHistoryWorkspace,
} from "./provider-session-view-model";
import {
  buildCodexLaunchConfig,
  canConnectCodex,
  getDefaultReasoningEffort,
} from "./codex-launch-config";

interface CodexPanelProps {
  codexTuiRunning: boolean;
  stopCodexTui: () => void;
  profile: { name?: string; planType?: string } | null;
  usage: CodexUsageData | null;
  refreshing: boolean;
  refreshUsage: () => void;
  providerSession?: ProviderSessionInfo;
}

export function CodexPanel({
  codexTuiRunning,
  stopCodexTui,
  profile,
  usage,
  refreshing,
  refreshUsage,
  providerSession,
}: CodexPanelProps) {
  const models = useCodexAccountStore((s) => s.models);
  const fetchModels = useCodexAccountStore((s) => s.fetchModels);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);
  const applyConfig = useBridgeStore((s) => s.applyConfig);
  const fetchProviderHistory = useTaskStore((s) => s.fetchProviderHistory);
  const providerHistory = useTaskStore((s) => s.providerHistory);
  const providerHistoryLoading = useTaskStore((s) => s.providerHistoryLoading);
  const providerHistoryError = useTaskStore((s) => s.providerHistoryError);

  const [selectedModel, setSelectedModel] = useState("");
  const [selectedReasoning, setSelectedReasoning] = useState("");
  const [cwd, setCwd] = useState("");
  const [selectedHistoryId, setSelectedHistoryId] = useState(
    NEW_PROVIDER_SESSION_VALUE,
  );
  const effectiveCwd = useMemo(
    () => resolveProviderHistoryWorkspace(cwd, providerSession),
    [cwd, providerSession],
  );

  const [connecting, setConnecting] = useState(false);
  const locked = codexTuiRunning;
  const prevRunningRef = useRef(codexTuiRunning);
  const [justConnected, setJustConnected] = useState(false);

  useEffect(() => {
    if (codexTuiRunning && !prevRunningRef.current) {
      setConnecting(false);
      setJustConnected(true);
      const t = setTimeout(() => setJustConnected(false), 600);
      return () => clearTimeout(t);
    }
    if (!codexTuiRunning) setConnecting(false);
    prevRunningRef.current = codexTuiRunning;
  }, [codexTuiRunning]);

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
  const workspaceHistory = effectiveCwd
    ? providerHistory[effectiveCwd] ?? []
    : [];
  const historyOptions = useMemo(
    () => buildProviderHistoryOptions("codex", workspaceHistory),
    [workspaceHistory],
  );
  const selectedHistory = useMemo(
    () => findProviderHistoryEntry("codex", workspaceHistory, selectedHistoryId),
    [selectedHistoryId, workspaceHistory],
  );
  const connectionLabel = useMemo(
    () => formatProviderConnectionLabel(providerSession),
    [providerSession],
  );
  const historyLoading = effectiveCwd
    ? providerHistoryLoading[effectiveCwd]
    : false;
  const historyError = effectiveCwd ? providerHistoryError[effectiveCwd] : null;

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

  const handlePickDir = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) {
      setCwd(dir);
      setSelectedHistoryId(NEW_PROVIDER_SESSION_VALUE);
    }
  }, [pickDirectory]);

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
    try {
      await applyConfig(
        buildCodexLaunchConfig({
          model: selectedModel,
          reasoningEffort: selectedReasoning,
          cwd: effectiveCwd,
          resumeThreadId: selectedHistory?.externalId,
        }),
      );
    } catch {
      setConnecting(false);
    }
  }, [
    applyConfig,
    selectedModel,
    selectedReasoning,
    effectiveCwd,
    selectedHistory,
  ]);

  return (
    <div
      className={cn(
        "rounded-lg border bg-card p-3 card-depth transition-all duration-300",
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
        cwd={effectiveCwd}
        handlePickDir={handlePickDir}
      />

      <div className="mt-2 flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">History</span>
        <CyberSelect
          value={selectedHistoryId}
          options={historyOptions}
          onChange={setSelectedHistoryId}
          disabled={locked || !effectiveCwd || connecting}
          placeholder="New session"
        />
      </div>

      {locked && (
        <Button
          size="sm"
          variant="secondary"
          className="w-full mt-2 active:scale-[0.98] transition-all duration-200"
          onClick={stopCodexTui}
        >
          Disconnect Codex
        </Button>
      )}

      {!locked && (
        <Button
          className="w-full mt-2 bg-codex text-white hover:bg-codex/90 hover:shadow-[0_0_16px_#22c55e40] active:scale-[0.98] transition-all duration-200 btn-ripple"
          size="sm"
          disabled={
            !canConnectCodex({
              cwd: effectiveCwd,
              connecting,
              running: !!codexTuiRunning,
            })
          }
          onClick={handleConnect}
        >
          {connecting ? (
            <span className="flex items-center gap-2">
              <span className="size-3 border-2 border-white/30 border-t-white rounded-full animate-spin" />
              Connecting…
            </span>
          ) : (
            "Connect Codex"
          )}
        </Button>
      )}

      {!locked && <AuthActions />}

      {!locked && effectiveCwd && historyLoading && (
        <div className="mt-1.5 text-[11px] text-muted-foreground">
          Loading Codex history...
        </div>
      )}
      {!locked && effectiveCwd && historyError && (
        <div className="mt-1.5 text-[11px] text-destructive">{historyError}</div>
      )}
      {!!codexTuiRunning && (
        <div className="mt-1.5 text-[11px] text-muted-foreground">
          Codex app-server is starting...
        </div>
      )}
    </div>
  );
}
