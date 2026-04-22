import { useEffect, useMemo, useState } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import { makeActiveCodexStreamSelector } from "@/stores/bridge-store/selectors";
import { SourceBadge } from "./SourceBadge";
import {
  getExpandableTextState,
  getCodexStreamIndicatorViewModel,
  getStreamTextTail,
} from "./view-model";
import { getStreamSurfacePresentation } from "./surface-styles";

export function CodexStreamIndicator() {
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const selectCodexStream = useMemo(
    () => makeActiveCodexStreamSelector(activeTaskId),
    [activeTaskId],
  );
  const stream = useBridgeStore(selectCodexStream);
  const { currentDelta, activity, reasoning, commandOutput } = stream;
  const codexStream = {
    thinking: stream.thinking,
    currentDelta,
    lastMessage: "",
    turnStatus: "",
    activity,
    reasoning,
    commandOutput,
  };
  const viewModel = getCodexStreamIndicatorViewModel(codexStream);
  const [reasoningExpanded, setReasoningExpanded] = useState(false);
  const displayReasoning = useMemo(
    () => getExpandableTextState(reasoning, 300, reasoningExpanded),
    [reasoning, reasoningExpanded],
  );
  const displayCommandOutput = useMemo(
    () => getStreamTextTail(commandOutput, 500),
    [commandOutput],
  );
  const surface = getStreamSurfacePresentation("codex");

  useEffect(() => {
    setReasoningExpanded(false);
  }, [reasoning]);

  if (!viewModel.visible) return null;

  return (
    <CodexStreamIndicatorView
      currentDelta={currentDelta}
      displayCommandOutput={displayCommandOutput}
      displayReasoning={displayReasoning}
      reasoningExpanded={reasoningExpanded}
      surface={surface}
      viewModel={viewModel}
      onToggleReasoning={() => setReasoningExpanded((value) => !value)}
    />
  );
}

export function CodexStreamIndicatorView({
  currentDelta,
  displayCommandOutput,
  displayReasoning,
  reasoningExpanded,
  surface,
  viewModel,
  onToggleReasoning,
}: {
  currentDelta: string;
  displayCommandOutput: string;
  displayReasoning: ReturnType<typeof getExpandableTextState>;
  reasoningExpanded: boolean;
  surface: ReturnType<typeof getStreamSurfacePresentation>;
  viewModel: ReturnType<typeof getCodexStreamIndicatorViewModel>;
  onToggleReasoning: () => void;
}) {
  return (
    <div className="py-1.5">
      <div className="flex justify-start">
        <div className="max-w-[82%] rounded-xl bg-codex/8 px-3.5 py-2.5">
          <div className="flex items-center gap-2 mb-1">
            <SourceBadge source="codex" />
            {viewModel.showStatusLabel && (
              <span
                className={`${surface.statusClass} ${viewModel.animatePulse ? "animate-pulse" : ""}`}
              >
                {currentDelta ? "working draft" : viewModel.statusLabel}
              </span>
            )}
          </div>
          {displayReasoning.text && !currentDelta && (
            <div
              className={`${surface.metaClass} mb-1 whitespace-pre-wrap ${
                reasoningExpanded ? "" : "max-h-24 overflow-y-auto"
              }`}
            >
              {displayReasoning.text}
            </div>
          )}
          {displayReasoning.canExpand && !currentDelta && (
            <button
              type="button"
              onClick={onToggleReasoning}
              className="mb-1 text-[11px] font-medium text-codex hover:text-codex/80"
            >
              {displayReasoning.toggleLabel}
            </button>
          )}
          {displayCommandOutput && !currentDelta && (
            <div className={`${surface.commandClass} mb-1`}>
              {displayCommandOutput}
            </div>
          )}
          {currentDelta && (
            <div className="text-[13px] text-foreground/82 whitespace-pre-wrap max-h-40 overflow-y-auto leading-relaxed">
              {currentDelta}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
