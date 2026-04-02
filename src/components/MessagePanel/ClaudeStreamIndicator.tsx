import { useBridgeStore } from "@/stores/bridge-store";
import { SourceBadge } from "./SourceBadge";
import { useEffect, useRef } from "react";

export function ClaudeStreamIndicator() {
  const thinking = useBridgeStore((s) => s.claudeStream.thinking);
  const previewText = useBridgeStore((s) => s.claudeStream.previewText);
  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto-scroll the streaming content container
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [previewText]);

  if (!thinking && !previewText) return null;

  const hasContent = previewText.length > 0;
  // Show tail of long content to keep it responsive
  const displayText =
    previewText.length > 3000 ? "…" + previewText.slice(-3000) : previewText;

  return (
    <div className="py-2">
      <div className="flex py-2.5 justify-start">
        <div className="max-w-[85%] rounded-xl px-3 py-2 bg-claude/10 border border-claude/30">
          <div className="flex items-center gap-2 mb-1">
            <SourceBadge source="claude" />
            <span
              className={`text-[11px] text-claude ${!hasContent ? "animate-pulse" : ""}`}
            >
              {hasContent ? "streaming…" : "thinking…"}
            </span>
            {hasContent && (
              <span className="text-[10px] text-muted-foreground/60">
                {previewText.length} chars
              </span>
            )}
          </div>
          {hasContent && (
            <div
              ref={scrollRef}
              className="text-[13px] text-card-foreground leading-relaxed whitespace-pre-wrap max-h-60 overflow-y-auto"
            >
              {displayText}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
