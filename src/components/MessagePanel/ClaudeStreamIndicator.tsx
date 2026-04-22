import { useEffect, useMemo, useState } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import { makeActiveClaudeStreamSelector } from "@/stores/bridge-store/selectors";
import { getExpandableTextState, getStreamTextTail } from "./view-model";
import { SourceBadge } from "./SourceBadge";
import { getStreamSurfacePresentation } from "./surface-styles";
import type { ClaudeBlockType } from "@/stores/bridge-store/types";

function blockLabel(blockType: ClaudeBlockType, toolName: string): string {
  switch (blockType) {
    case "thinking":
      return "thinking…";
    case "text":
      return "writing…";
    case "tool":
      return toolName ? `using ${toolName}` : "using tool…";
    default:
      return "thinking…";
  }
}

export function ClaudeStreamIndicator() {
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const selectClaudeStream = useMemo(
    () => makeActiveClaudeStreamSelector(activeTaskId),
    [activeTaskId],
  );
  const stream = useBridgeStore(selectClaudeStream);
  const { thinking, previewText, thinkingText, blockType, toolName } = stream;
  const surface = getStreamSurfacePresentation("claude");
  const [thinkingExpanded, setThinkingExpanded] = useState(false);

  const displayText = useMemo(
    () => getStreamTextTail(previewText, 3000),
    [previewText],
  );
  const displayThinking = useMemo(
    () => getExpandableTextState(thinkingText, 300, thinkingExpanded),
    [thinkingText, thinkingExpanded],
  );

  useEffect(() => {
    setThinkingExpanded(false);
  }, [thinkingText]);

  if (!thinking && !previewText && !thinkingText) return null;

  const hasText = previewText.length > 0;
  const label = blockLabel(blockType, toolName);
  const isAnimating = blockType === "thinking" && !thinkingText;

  return (
    <div className="py-1.5">
      <div className="flex justify-start">
        <div className="max-w-[82%] rounded-xl bg-claude/8 px-3.5 py-2.5">
          <div className="flex items-center gap-2 mb-1">
            <SourceBadge source="claude" />
            <span
              className={`${surface.statusClass} ${isAnimating ? "animate-pulse" : ""}`}
            >
              {label}
            </span>
          </div>
          {hasText && <div className={surface.commandClass}>{displayText}</div>}
          {displayThinking.text && (
            <div
              className={`text-[11px] text-muted-foreground/50 italic whitespace-pre-wrap mt-1 ${
                thinkingExpanded ? "" : "max-h-24 overflow-hidden"
              }`}
            >
              {displayThinking.text}
            </div>
          )}
          {displayThinking.canExpand && (
            <button
              type="button"
              onClick={() => setThinkingExpanded((v) => !v)}
              className="mt-1 text-[11px] font-medium text-claude hover:text-claude/80 transition-colors active:scale-95"
            >
              {displayThinking.toggleLabel}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
