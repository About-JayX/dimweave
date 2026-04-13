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
  getMessageListFollowOutputMode,
  shouldClearStickyOnScroll,
  shouldResetMessageListInitialScroll,
  shouldScrollOnStreamTail,
  PROGRAMMATIC_SCROLL_IMMUNITY_MS,
  STICKY_BOTTOM_THRESHOLD,
  type StreamIndicatorId,
} from "./view-model";

interface Props {
  emptyStateMessage?: string;
  messages: BridgeMessage[];
  searchActive?: boolean;
  onOpenImage?: (attachment: Attachment) => void;
}

/**
 * Determines the scroll strategy for draft anchor effects.
 * "scroller-bottom" uses scrollTo(scrollHeight) to reach the absolute content
 * bottom — required when a growing item is already visible but its bottom edge
 * is off-screen. "last-index" falls back to scrollToIndex for SSR / before the
 * scroller element mounts.
 */
export function getDraftScrollStrategy(
  hasScrollerElement: boolean,
): "scroller-bottom" | "last-index" {
  return hasScrollerElement ? "scroller-bottom" : "last-index";
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
  const virtuosoRef = useRef<VirtuosoHandle>(null);
  const scrollerRef = useRef<HTMLElement | null>(null);
  const didInitialScrollRef = useRef(false);
  const stickyRef = useRef(true);
  const programmaticScrollRef = useRef<number>(0);
  const [showBackToBottom, setShowBackToBottom] = useState(false);
  const [scrollerNode, setScrollerNode] = useState<HTMLElement | null>(null);
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

  // Only restore sticky on return-to-bottom; ignore false (content growth)
  const handleAtBottomChange = useCallback((bottom: boolean) => {
    if (bottom) {
      stickyRef.current = true;
      setShowBackToBottom(false);
    }
  }, []);

  // Detect user-initiated scroll-away. Programmatic scrolls (followOutput,
  // scrollToBottom, initial scroll) set an immunity window to prevent false positives.
  useEffect(() => {
    if (!scrollerNode) return;
    let lastScrollTop = scrollerNode.scrollTop;
    const onScroll = () => {
      const el = scrollerRef.current;
      if (!el) return;
      const top = el.scrollTop;
      const scrolledUp = top < lastScrollTop;
      lastScrollTop = top;
      const immunityActive =
        Date.now() - programmaticScrollRef.current <
        PROGRAMMATIC_SCROLL_IMMUNITY_MS;
      const dist = el.scrollHeight - top - el.clientHeight;
      if (!shouldClearStickyOnScroll(scrolledUp, dist, immunityActive)) return;
      stickyRef.current = false;
      setShowBackToBottom(true);
    };
    scrollerNode.addEventListener("scroll", onScroll, { passive: true });
    return () => scrollerNode.removeEventListener("scroll", onScroll);
  }, [scrollerNode]);

  const followOutputFn = useCallback(() => {
    const mode = getMessageListFollowOutputMode(
      searchActive,
      stickyRef.current,
    );
    if (mode !== false) programmaticScrollRef.current = Date.now();
    return mode;
  }, [searchActive]);

  const scrollToBottom = useCallback(() => {
    stickyRef.current = true;
    setShowBackToBottom(false);
    programmaticScrollRef.current = Date.now();
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
    if (shouldResetMessageListInitialScroll(searchActive, totalCount)) {
      didInitialScrollRef.current = false;
      return;
    }
    if (searchActive || totalCount === 0 || didInitialScrollRef.current) return;
    didInitialScrollRef.current = true;
    const raf = window.requestAnimationFrame(() => {
      programmaticScrollRef.current = Date.now();
      virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "auto" });
    });
    return () => window.cancelAnimationFrame(raf);
  }, [searchActive, totalCount]);

  // Pin viewport to the absolute content bottom whenever any stream tail is
  // active: Claude draft (inline row, may grow) or Codex thinking/activity
  // (StreamTailFooter). scrollTo(scrollHeight) reaches the actual bottom even
  // when the last item is already partially visible but its bottom is off-screen.
  // rAF throttles to one scroll per frame; cleanup cancels a pending rAF.
  useEffect(() => {
    if (
      !shouldScrollOnStreamTail(
        hasClaudeDraft,
        codexVisible,
        searchActive,
        stickyRef.current,
      )
    )
      return;
    const raf = window.requestAnimationFrame(() => {
      programmaticScrollRef.current = Date.now();
      const el = scrollerRef.current;
      if (getDraftScrollStrategy(el !== null) === "scroller-bottom" && el) {
        el.scrollTo({ top: el.scrollHeight });
      } else {
        virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "auto" });
      }
    });
    return () => window.cancelAnimationFrame(raf);
  }, [hasClaudeDraft, claudePreviewText, codexVisible, searchActive]);

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
            const node = el instanceof HTMLElement ? el : null;
            scrollerRef.current = node;
            setScrollerNode(node);
          }}
          totalCount={totalCount}
          atBottomStateChange={handleAtBottomChange}
          atBottomThreshold={STICKY_BOTTOM_THRESHOLD}
          followOutput={followOutputFn}
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
      {showBackToBottom && (
        <div className="absolute bottom-2 left-1/2 -translate-x-1/2 z-10">
          <BackToBottomButton onClick={scrollToBottom} />
        </div>
      )}
    </div>
  );
}
