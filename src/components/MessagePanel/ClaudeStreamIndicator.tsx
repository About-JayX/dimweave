import { useMemo } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { getStreamTextTail } from "./view-model";
import { SourceBadge } from "./SourceBadge";
import { getStreamSurfacePresentation } from "./surface-styles";

export function ClaudeStreamIndicator() {
  const thinking = useBridgeStore((s) => s.claudeStream.thinking);
  const previewText = useBridgeStore((s) => s.claudeStream.previewText);
  if (!thinking && !previewText) return null;

  const hasContent = previewText.length > 0;
  const surface = getStreamSurfacePresentation("claude");
  // Show tail of long content to keep it responsive
  const displayText = useMemo(
    () => getStreamTextTail(previewText, 3000),
    [previewText],
  );

  return (
    <div className="py-2">
      <div className="flex py-2.5 justify-start">
        <div className={surface.containerClass}>
          <div className="flex items-center gap-2 mb-1">
            <SourceBadge source="claude" />
            <span
              className={`${surface.statusClass} ${!hasContent ? "animate-pulse" : ""}`}
            >
              {hasContent ? "working draft" : "thinking…"}
            </span>
            {hasContent && (
              <span className={surface.metaClass}>
                {previewText.length} chars
              </span>
            )}
          </div>
          {hasContent && (
            <div className={surface.commandClass}>
              {displayText}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
