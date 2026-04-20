import { useCallback, useEffect, useRef, useState } from "react";
import type { VirtuosoHandle } from "react-virtuoso";

/// Single owner of "should the chat follow bottom?" for MessageList.
///
/// **Design principles** (answers to why we iterated on this logic 36+
/// commits in two weeks):
///
/// 1. **Separate "user intent" from "am I at bottom".** Content growth
///    (new message, streaming footer) can push scrollHeight past scrollTop
///    and make Virtuoso fire `atBottomStateChange(false)` even though the
///    user didn't move. If we drove `followOutputMode` directly from that
///    signal, the first new message post-growth returns `false` and
///    Virtuoso never scrolls — the exact bug the old immunity window
///    was papering over.
/// 2. **User intent comes from input events, not scrollTop deltas.**
///    Wheel, touchmove, and arrow/Page keys = user scrolling away.
///    Virtuoso's internal scroll corrections don't fire these events,
///    so we don't need a 300ms immunity window to suppress false
///    positives. Programmatic scrollTo never mutates the latch.
/// 3. **Auto-clear on return-to-bottom.** When the user scrolls back down
///    (or hits the Back-to-Bottom pill), `atBottomStateChange(true)`
///    fires; we clear the away latch so follow-output re-engages.
/// 4. **Stream-tail nudge is independent.** When a footer/draft row grows
///    between renders (Virtuoso doesn't fire followOutput for non-item
///    growth), the caller invokes `nudgeToBottom()` from an effect keyed
///    on delta state. The nudge respects `userAway`.
///
/// The hook exposes a single boolean (`userAway`) as the governor. Three
/// separate decision layers (Virtuoso auto-follow, DOM scroll listener,
/// stream-tail effect) now all read from the same latch.

export interface UseScrollAnchorOptions {
  /** True when a filter/search is active — disables all auto-follow. */
  searchActive: boolean;
  /** Items count (excluding footer). Drives the one-shot initial scroll. */
  totalCount: number;
}

export interface UseScrollAnchorResult {
  virtuosoRef: React.RefObject<VirtuosoHandle | null>;
  scrollerRefCallback: (el: HTMLElement | Window | null) => void;
  showBackToBottom: boolean;
  onAtBottomStateChange: (bottom: boolean) => void;
  followOutputMode: () => false | "smooth";
  scrollToBottom: () => void;
  /** Nudge the scroller to absolute bottom during stream-tail growth. */
  nudgeToBottom: () => void;
}

export function useScrollAnchor(
  opts: UseScrollAnchorOptions,
): UseScrollAnchorResult {
  const { searchActive, totalCount } = opts;
  const virtuosoRef = useRef<VirtuosoHandle | null>(null);
  const scrollerRef = useRef<HTMLElement | null>(null);
  const [scrollerNode, setScrollerNode] = useState<HTMLElement | null>(null);
  const didInitialScrollRef = useRef(false);

  // `userAway` is the canonical "should we stop following?" latch.
  // Set by user intent events, cleared by return-to-bottom.
  // Ref (not state) so readers don't re-render on flip; render-relevant
  // consumers read via React state `showBackToBottom`.
  const userAwayRef = useRef(false);
  const [showBackToBottom, setShowBackToBottom] = useState(false);

  const markUserAway = useCallback(() => {
    if (userAwayRef.current) return;
    userAwayRef.current = true;
    setShowBackToBottom(true);
  }, []);

  const clearUserAway = useCallback(() => {
    if (!userAwayRef.current) return;
    userAwayRef.current = false;
    setShowBackToBottom(false);
  }, []);

  // Virtuoso caches the scrollerRef callback internally; mutating identity
  // would churn its effects (null → el → null oscillation). Freeze.
  const scrollerRefCallback = useCallback((el: HTMLElement | Window | null) => {
    const node = el instanceof HTMLElement ? el : null;
    scrollerRef.current = node;
    setScrollerNode((prev) => (prev === node ? prev : node));
  }, []);

  // Virtuoso fires atBottomStateChange(true) once scroll lands at the
  // bottom threshold (50px). That's the auto-clear signal. We do NOT
  // consume `false` here — see design note 1.
  const onAtBottomStateChange = useCallback(
    (bottom: boolean) => {
      if (bottom) clearUserAway();
    },
    [clearUserAway],
  );

  const followOutputMode = useCallback((): false | "smooth" => {
    if (searchActive) return false;
    return userAwayRef.current ? false : "smooth";
  }, [searchActive]);

  const scrollToBottom = useCallback(() => {
    clearUserAway();
    virtuosoRef.current?.scrollToIndex({
      index: "LAST",
      behavior: "smooth",
    });
  }, [clearUserAway]);

  const nudgeToBottom = useCallback(() => {
    if (searchActive || userAwayRef.current) return;
    const el = scrollerRef.current;
    if (el) {
      el.scrollTo({ top: el.scrollHeight });
    } else {
      virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "auto" });
    }
  }, [searchActive]);

  // User-intent listeners on the Virtuoso scroller. wheel/touchmove/keyboard
  // only — scroll-event-based detection was the source of the immunity-window
  // regressions.
  useEffect(() => {
    if (!scrollerNode) return;
    const onWheel = (e: WheelEvent) => {
      if (e.deltaY < 0) markUserAway();
    };
    const onTouchMove = () => markUserAway();
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "ArrowUp" || e.key === "PageUp" || e.key === "Home") {
        markUserAway();
      }
    };
    scrollerNode.addEventListener("wheel", onWheel, { passive: true });
    scrollerNode.addEventListener("touchmove", onTouchMove, { passive: true });
    scrollerNode.addEventListener("keydown", onKeyDown);
    return () => {
      scrollerNode.removeEventListener("wheel", onWheel);
      scrollerNode.removeEventListener("touchmove", onTouchMove);
      scrollerNode.removeEventListener("keydown", onKeyDown);
    };
  }, [scrollerNode, markUserAway]);

  // One-shot initial scroll when the list first becomes non-empty (and we're
  // not in search mode). Search-active → empty → active flips reset the latch.
  useEffect(() => {
    if (searchActive || totalCount === 0) {
      didInitialScrollRef.current = false;
      return;
    }
    if (didInitialScrollRef.current) return;
    didInitialScrollRef.current = true;
    const raf = window.requestAnimationFrame(() => {
      virtuosoRef.current?.scrollToIndex({ index: "LAST", behavior: "auto" });
    });
    return () => window.cancelAnimationFrame(raf);
  }, [searchActive, totalCount]);

  return {
    virtuosoRef,
    scrollerRefCallback,
    showBackToBottom: showBackToBottom && !searchActive,
    onAtBottomStateChange,
    followOutputMode,
    scrollToBottom,
    nudgeToBottom,
  };
}
