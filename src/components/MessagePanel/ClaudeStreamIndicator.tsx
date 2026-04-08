import { useMemo } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { getStreamTextTail } from "./view-model";
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
  const thinking = useBridgeStore((s) => s.claudeStream.thinking);
  const previewText = useBridgeStore((s) => s.claudeStream.previewText);
  const thinkingText = useBridgeStore((s) => s.claudeStream.thinkingText);
  const blockType = useBridgeStore((s) => s.claudeStream.blockType);
  const toolName = useBridgeStore((s) => s.claudeStream.toolName);
  const surface = getStreamSurfacePresentation("claude");

  const displayText = useMemo(
    () => getStreamTextTail(previewText, 3000),
    [previewText],
  );
  const displayThinking = useMemo(
    () => getStreamTextTail(thinkingText, 1000),
    [thinkingText],
  );

  if (!thinking && !previewText && !thinkingText) return null;

  const hasText = previewText.length > 0;
  const hasThinking = thinkingText.length > 0;
  const label = blockLabel(blockType, toolName);
  const isAnimating = blockType === "thinking" && !hasThinking;

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
          {hasThinking && blockType === "thinking" && (
            <div className="text-[11px] text-muted-foreground/50 italic whitespace-pre-wrap max-h-24 overflow-hidden mb-1">
              {displayThinking}
            </div>
          )}
          {hasText && (
            <div className={surface.commandClass}>{displayText}</div>
          )}
        </div>
      </div>
    </div>
  );
}
