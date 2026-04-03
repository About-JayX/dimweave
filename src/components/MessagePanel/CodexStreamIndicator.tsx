import { useMemo } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { SourceBadge } from "./SourceBadge";
import {
  getCodexStreamIndicatorViewModel,
  getStreamTextTail,
} from "./view-model";
import { getStreamSurfacePresentation } from "./surface-styles";

export function CodexStreamIndicator() {
  const thinking = useBridgeStore((s) => s.codexStream.thinking);
  const currentDelta = useBridgeStore((s) => s.codexStream.currentDelta);
  const activity = useBridgeStore((s) => s.codexStream.activity);
  const reasoning = useBridgeStore((s) => s.codexStream.reasoning);
  const commandOutput = useBridgeStore((s) => s.codexStream.commandOutput);
  const codexStream = {
    thinking,
    currentDelta,
    lastMessage: "",
    turnStatus: "",
    activity,
    reasoning,
    commandOutput,
  };
  const viewModel = getCodexStreamIndicatorViewModel(codexStream);
  const displayReasoning = useMemo(
    () => getStreamTextTail(reasoning, 300),
    [reasoning],
  );
  const displayCommandOutput = useMemo(
    () => getStreamTextTail(commandOutput, 500),
    [commandOutput],
  );
  const surface = getStreamSurfacePresentation("codex");

  if (!viewModel.visible) return null;

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
          {reasoning && !currentDelta && (
            <div
              className={`${surface.metaClass} mb-1 whitespace-pre-wrap max-h-24 overflow-y-auto`}
            >
              {displayReasoning}
            </div>
          )}
          {commandOutput && !currentDelta && (
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
