import { useEffect, useMemo } from "react";
import { Virtuoso } from "react-virtuoso";
import { useBridgeStore } from "@/stores/bridge-store";
import { useTaskStore } from "@/stores/task-store";
import {
  makeActiveClaudeStreamSelector,
  makeActiveCodexStreamSelector,
} from "@/stores/bridge-store/selectors";
import type { Attachment, BridgeMessage } from "@/types";
import { MessageBubble } from "./MessageBubble";
import { CodexStreamIndicator } from "./CodexStreamIndicator";
import { ClaudeStreamIndicator } from "./ClaudeStreamIndicator";
import { BackToBottomButton } from "./search-chrome";
import {
  getCodexStreamIndicatorViewModel,
  getMessageListDisplayState,
  STICKY_BOTTOM_THRESHOLD,
  type StreamIndicatorId,
} from "./view-model";
import { useScrollAnchor } from "./use-scroll-anchor";

interface Props {
  emptyStateMessage?: string;
  messages: BridgeMessage[];
  searchActive?: boolean;
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
  searchActive = false,
  onOpenImage,
}: Props) {
  const activeTaskId = useTaskStore((s) => s.activeTaskId);
  const selectClaudeStream = useMemo(
    () => makeActiveClaudeStreamSelector(activeTaskId),
    [activeTaskId],
  );
  const selectCodexStream = useMemo(
    () => makeActiveCodexStreamSelector(activeTaskId),
    [activeTaskId],
  );
  const claudeStream = useBridgeStore(selectClaudeStream);
  const codexStream = useBridgeStore(selectCodexStream);
  const { thinking: claudeThinking, previewText: claudePreviewText } =
    claudeStream;
  const hasClaudeDraft = claudeThinking || claudePreviewText.length > 0;
  const codexVisible = getCodexStreamIndicatorViewModel(codexStream).visible;
  const codexStreamTail =
    codexStream.currentDelta ||
    codexStream.activity ||
    codexStream.reasoning ||
    codexStream.commandOutput;
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

  const anchor = useScrollAnchor({ searchActive, totalCount });

  // Stream-tail nudge: Claude draft row and Codex Footer grow vertically
  // between renders without triggering Virtuoso's followOutput. Nudge the
  // scroller to absolute bottom; `nudgeToBottom` internally no-ops when
  // search is active or the user scrolled away.
  useEffect(() => {
    if (!hasClaudeDraft && !codexVisible) return;
    const raf = window.requestAnimationFrame(() => anchor.nudgeToBottom());
    return () => window.cancelAnimationFrame(raf);
  }, [
    hasClaudeDraft,
    claudePreviewText,
    codexVisible,
    codexStreamTail,
    anchor,
  ]);

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
          ref={anchor.virtuosoRef}
          scrollerRef={anchor.scrollerRefCallback}
          totalCount={totalCount}
          // Land at the last item on initial mount so remounts (e.g. tab
          // switch away from chat and back) restore the bottom view.
          // Virtuoso handles this before first paint, which our rAF-based
          // initial-scroll effect can't match once heights are unmeasured.
          initialTopMostItemIndex={totalCount > 0 ? totalCount - 1 : 0}
          atBottomStateChange={anchor.onAtBottomStateChange}
          atBottomThreshold={STICKY_BOTTOM_THRESHOLD}
          followOutput={anchor.followOutputMode}
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
      {anchor.showBackToBottom && (
        <div className="absolute bottom-2 left-1/2 -translate-x-1/2 z-10">
          <BackToBottomButton onClick={anchor.scrollToBottom} />
        </div>
      )}
    </div>
  );
}
