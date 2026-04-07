import { useRef, useState, useCallback, useEffect, useMemo } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { useBridgeStore } from "@/stores/bridge-store";
import type { Attachment, BridgeMessage } from "@/types";
import { MessageBubble } from "./MessageBubble";
import { CodexStreamIndicator } from "./CodexStreamIndicator";
import { ClaudeStreamIndicator } from "./ClaudeStreamIndicator";
import {
  getCodexStreamIndicatorViewModel,
  getMessageListDisplayState,
} from "./view-model";

interface Props {
  emptyStateMessage?: string;
  messages: BridgeMessage[];
  onOpenImage?: (attachment: Attachment) => void;
}

export function MessageList({
  emptyStateMessage,
  messages,
  onOpenImage,
}: Props) {
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const didInitialScrollRef = useRef(false);
  const [atBottom, setAtBottom] = useState(true);
  const claudeThinking = useBridgeStore((s) => s.claudeStream.thinking);
  const codexVisible = useBridgeStore(
    (s) => getCodexStreamIndicatorViewModel(s.codexStream).visible,
  );
  const streamRailIndicators = useMemo(
    () => [
      ...(claudeThinking ? (["claude"] as const) : []),
      ...(codexVisible ? (["codex"] as const) : []),
    ],
    [claudeThinking, codexVisible],
  );

  const displayState = useMemo(
    () => getMessageListDisplayState(messages.length, streamRailIndicators),
    [messages.length, streamRailIndicators],
  );
  const totalCount = displayState.timelineCount;

  const handleAtBottomChange = useCallback((bottom: boolean) => {
    setAtBottom(bottom);
  }, []);

  const scrollToBottom = useCallback(() => {
    virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "smooth" });
  }, []);

  useEffect(() => {
    if (totalCount === 0) {
      didInitialScrollRef.current = false;
      return;
    }
    if (didInitialScrollRef.current) return;
    didInitialScrollRef.current = true;
    const raf = window.requestAnimationFrame(() => {
      virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "auto" });
    });
    return () => window.cancelAnimationFrame(raf);
  }, [totalCount]);

  if (!displayState.hasContent) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-[13px] text-muted-foreground animate-in fade-in duration-500">
          {emptyStateMessage ??
            "No messages yet. Connect Claude and Codex to start bridging."}
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 min-h-0 relative flex flex-col">
      <div className="flex-1 min-h-0">
        <Virtuoso
          ref={virtuosoRef}
          totalCount={totalCount}
          atBottomStateChange={handleAtBottomChange}
          atBottomThreshold={80}
          followOutput="smooth"
          className="h-full"
          increaseViewportBy={200}
          itemContent={(index) => (
            <div className="px-4">
              <MessageBubble msg={messages[index]} onOpenImage={onOpenImage} />
            </div>
          )}
        />
      </div>
      {!atBottom && (
        <div className="flex justify-center py-1.5">
          <button
            onClick={scrollToBottom}
            className="z-10 px-3 py-1.5 rounded-full text-[11px] bg-primary/90 text-primary-foreground shadow-lg hover:bg-primary transition-colors"
          >
            ↓ Back to bottom
          </button>
        </div>
      )}
      {displayState.streamRailIndicators.length > 0 && (
        <div className="px-4 pb-2">
          {displayState.streamRailIndicators.map((indicator) =>
            indicator === "claude" ? (
              <ClaudeStreamIndicator key={indicator} />
            ) : (
              <CodexStreamIndicator key={indicator} />
            ),
          )}
        </div>
      )}
    </div>
  );
}
