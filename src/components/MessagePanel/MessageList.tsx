import { useRef, useState, useCallback, useEffect, useMemo } from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { useBridgeStore } from "@/stores/bridge-store";
import type { Attachment, BridgeMessage } from "@/types";
import { MessageBubble } from "./MessageBubble";
import { CodexStreamIndicator } from "./CodexStreamIndicator";
import { ClaudeStreamIndicator } from "./ClaudeStreamIndicator";
import { BackToBottomButton } from "./search-chrome";
import {
  getCodexStreamIndicatorViewModel,
  getMessageListDisplayState,
  type StreamIndicatorId,
} from "./view-model";

interface Props {
  emptyStateMessage?: string;
  messages: BridgeMessage[];
  onOpenImage?: (attachment: Attachment) => void;
}

type FooterContext = { indicators: StreamIndicatorId[] };

export function StreamTailFooter({ context }: { context?: FooterContext }) {
  const indicators = context?.indicators ?? [];
  if (indicators.length === 0) return null;
  return (
    <div className="px-4 pb-2">
      {indicators.map((indicator) =>
        indicator === "codex" ? <CodexStreamIndicator key={indicator} /> : null,
      )}
    </div>
  );
}

export function MessageList({
  emptyStateMessage,
  messages,
  onOpenImage,
}: Props) {
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const scrollerRef = useRef<HTMLElement | null>(null);
  const didInitialScrollRef = useRef(false);
  const [atBottom, setAtBottom] = useState(true);
  const claudeThinking = useBridgeStore((s) => s.claudeStream.thinking);
  const claudePreviewText = useBridgeStore((s) => s.claudeStream.previewText);
  const hasClaudeDraft = claudeThinking || claudePreviewText.length > 0;
  const codexVisible = useBridgeStore(
    (s) => getCodexStreamIndicatorViewModel(s.codexStream).visible,
  );
  const streamRailIndicators = useMemo(
    () => [...(codexVisible ? (["codex"] as const) : [])],
    [codexVisible],
  );

  const displayState = useMemo(
    () =>
      getMessageListDisplayState({
        messageCount: messages.length,
        hasClaudeDraft,
        streamRailIndicators,
      }),
    [messages.length, hasClaudeDraft, streamRailIndicators],
  );
  const totalCount = displayState.timelineCount;

  const footerContext = useMemo<FooterContext>(
    () => ({ indicators: streamRailIndicators }),
    [streamRailIndicators],
  );

  const handleAtBottomChange = useCallback((bottom: boolean) => {
    setAtBottom(bottom);
  }, []);

  const scrollToBottom = useCallback(() => {
    if (scrollerRef.current) {
      scrollerRef.current.scrollTo({
        top: scrollerRef.current.scrollHeight,
        behavior: "smooth",
      });
    } else {
      virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "smooth" });
    }
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
          scrollerRef={(el) => {
            scrollerRef.current = el as HTMLElement | null;
          }}
          totalCount={totalCount}
          atBottomStateChange={handleAtBottomChange}
          atBottomThreshold={80}
          followOutput="smooth"
          className="h-full"
          increaseViewportBy={200}
          context={footerContext}
          components={{ Footer: StreamTailFooter }}
          itemContent={(index) => {
            const isClaudeDraftRow =
              hasClaudeDraft && index === messages.length;
            if (isClaudeDraftRow) {
              return (
                <div className="px-4">
                  <ClaudeStreamIndicator />
                </div>
              );
            }
            return (
              <div className="px-4">
                <MessageBubble
                  msg={messages[index]}
                  onOpenImage={onOpenImage}
                />
              </div>
            );
          }}
        />
      </div>
      {!atBottom && (
        <div className="absolute bottom-2 left-1/2 -translate-x-1/2 z-10">
          <BackToBottomButton onClick={scrollToBottom} />
        </div>
      )}
    </div>
  );
}
