import { useState, useEffect, useCallback, useMemo, useRef } from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { useBridgeStore } from "@/stores/bridge-store";
import { useCodexAccountStore } from "@/stores/codex-account-store";
import { AuthActions } from "./AuthActions";
import { CodexHeader } from "./CodexHeader";
import { CodexUsageSection, type CodexUsageData } from "./CodexUsageSection";
import { CodexConfigRows } from "./CodexConfigRows";

interface CodexPanelProps {
  codexTuiRunning: boolean;
  codexReady: boolean;
  threadId: string | null;
  stopCodexTui: () => void;
  profile: { name?: string; planType?: string } | null;
  usage: CodexUsageData | null;
  refreshing: boolean;
  refreshUsage: () => void;
}

export function CodexPanel({
  codexTuiRunning,
  codexReady,
  threadId,
  stopCodexTui,
  profile,
  usage,
  refreshing,
  refreshUsage,
}: CodexPanelProps) {
  const models = useCodexAccountStore((s) => s.models);
  const fetchModels = useCodexAccountStore((s) => s.fetchModels);
  const pickDirectory = useCodexAccountStore((s) => s.pickDirectory);
  const applyConfig = useBridgeStore((s) => s.applyConfig);

  const [selectedModel, setSelectedModel] = useState("");
  const [selectedReasoning, setSelectedReasoning] = useState("");
  const [cwd, setCwd] = useState("");

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
      setSelectedReasoning(
        first.defaultReasoningLevel || first.reasoningLevels[0]?.effort || "",
      );
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

  const handleModelChange = useCallback(
    (slug: string) => {
      setSelectedModel(slug);
      const m = models.find((x) => x.slug === slug);
      if (m) {
        setSelectedReasoning(
          m.defaultReasoningLevel || m.reasoningLevels[0]?.effort || "",
        );
      }
    },
    [models],
  );

  const handlePickDir = useCallback(async () => {
    const dir = await pickDirectory();
    if (dir) setCwd(dir);
  }, [pickDirectory]);

  const handleConnect = useCallback(async () => {
    setConnecting(true);
    try {
      await applyConfig({
        model: selectedModel || undefined,
        cwd: cwd || undefined,
      });
    } catch {
      setConnecting(false);
    }
  }, [applyConfig, selectedModel, selectedReasoning, cwd]);

  return (
    <div
      className={cn(
        "rounded-lg border bg-card p-3 card-depth transition-all duration-300",
        codexTuiRunning
          ? "border-codex/40 glow-codex-subtle border-glow-codex"
          : codexReady
            ? "border-yellow-500/30"
            : "border-input hover:border-input/80",
        justConnected && "card-connect-anim",
      )}
    >
      <CodexHeader
        running={codexTuiRunning}
        ready={codexReady}
        threadId={threadId}
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
        cwd={cwd}
        handlePickDir={handlePickDir}
      />

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
          disabled={!codexReady || connecting}
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

      {!codexReady && (
        <div className="mt-1.5 text-[11px] text-muted-foreground">
          Codex app-server is starting...
        </div>
      )}
    </div>
  );
}
