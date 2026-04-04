import { useCallback, useEffect, useRef } from "react";
import {
  REPLY_INPUT_HEIGHT_STORAGE_KEY,
  REPLY_INPUT_MIN_ROWS,
  getReplyInputHeightBounds,
  normalizeReplyInputMinHeight,
  resolveDraggedReplyInputMinHeight,
  resolveReplyInputHeight,
  type ReplyInputHeightBounds,
} from "./height";

function measureBaseTextareaHeight(el: HTMLTextAreaElement): number {
  const style = getComputedStyle(el);
  const lineHeight = parseFloat(style.lineHeight) || 20;
  const paddingTop = parseFloat(style.paddingTop) || 0;
  const paddingBottom = parseFloat(style.paddingBottom) || 0;
  return REPLY_INPUT_MIN_ROWS * lineHeight + paddingTop + paddingBottom;
}

export function useReplyInputResizer(draft: string) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const minHeightRef = useRef<number | null>(null);
  const baseMinHeightRef = useRef<number | null>(null);
  const dragFrameRef = useRef(0);

  const getHeightBounds = useCallback((): ReplyInputHeightBounds | null => {
    const el = textareaRef.current;
    if (!el) {
      return null;
    }
    if (baseMinHeightRef.current == null) {
      baseMinHeightRef.current = measureBaseTextareaHeight(el);
    }
    return getReplyInputHeightBounds(baseMinHeightRef.current, window.innerHeight);
  }, []);

  const persistMinHeight = useCallback((nextMinHeight: number) => {
    minHeightRef.current = nextMinHeight;
    try {
      localStorage.setItem(REPLY_INPUT_HEIGHT_STORAGE_KEY, String(nextMinHeight));
    } catch {}
  }, []);

  const syncTextareaHeight = useCallback(
    (requestedMinHeight?: number) => {
      const el = textareaRef.current;
      const bounds = getHeightBounds();
      if (!el || !bounds) {
        return null;
      }
      const nextMinHeight = Math.min(
        Math.max(requestedMinHeight ?? minHeightRef.current ?? bounds.min, bounds.min),
        bounds.max,
      );
      el.style.height = "auto";
      const { height, overflowY } = resolveReplyInputHeight(
        el.scrollHeight,
        nextMinHeight,
        bounds,
      );
      el.style.minHeight = `${bounds.min}px`;
      el.style.height = `${height}px`;
      el.style.overflowY = overflowY;
      return { bounds, nextMinHeight };
    },
    [getHeightBounds],
  );

  useEffect(() => {
    const bounds = getHeightBounds();
    if (!bounds) {
      return;
    }
    const persistedMinHeight = normalizeReplyInputMinHeight(
      (() => {
        try {
          return localStorage.getItem(REPLY_INPUT_HEIGHT_STORAGE_KEY);
        } catch {
          return null;
        }
      })(),
      bounds,
    );
    minHeightRef.current = persistedMinHeight;
    syncTextareaHeight(persistedMinHeight);
  }, [getHeightBounds, syncTextareaHeight]);

  useEffect(() => {
    syncTextareaHeight();
  }, [draft, syncTextareaHeight]);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    const debounced = () => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        baseMinHeightRef.current = null;
        const next = syncTextareaHeight();
        if (!next) {
          return;
        }
        if (next.nextMinHeight !== minHeightRef.current) {
          persistMinHeight(next.nextMinHeight);
        }
      }, 100);
    };
    window.addEventListener("resize", debounced);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("resize", debounced);
    };
  }, [persistMinHeight, syncTextareaHeight]);

  const handleResizePointerDown = useCallback(
    (event: React.PointerEvent<HTMLDivElement>) => {
      event.preventDefault();
      const bounds = getHeightBounds();
      if (!bounds) {
        return;
      }
      const startMinHeight = minHeightRef.current ?? bounds.min;
      const startY = event.clientY;

      const flushDragHeight = (nextMinHeight: number) => {
        if (dragFrameRef.current) {
          return;
        }
        dragFrameRef.current = requestAnimationFrame(() => {
          dragFrameRef.current = 0;
          syncTextareaHeight(nextMinHeight);
        });
      };

      const finish = (nextMinHeight: number) => {
        document.removeEventListener("pointermove", onMove);
        document.removeEventListener("pointerup", onUp);
        document.removeEventListener("pointercancel", onCancel);
        document.body.style.userSelect = "";
        if (dragFrameRef.current) {
          cancelAnimationFrame(dragFrameRef.current);
          dragFrameRef.current = 0;
        }
        const next = syncTextareaHeight(nextMinHeight);
        if (!next) {
          return;
        }
        persistMinHeight(next.nextMinHeight);
      };

      const onMove = (nextEvent: PointerEvent) => {
        const nextBounds = getHeightBounds();
        if (!nextBounds) {
          return;
        }
        const nextMinHeight = resolveDraggedReplyInputMinHeight(
          startMinHeight,
          startY,
          nextEvent.clientY,
          nextBounds,
        );
        flushDragHeight(nextMinHeight);
      };

      const onUp = (nextEvent: PointerEvent) => {
        const nextBounds = getHeightBounds();
        if (!nextBounds) {
          return finish(startMinHeight);
        }
        finish(
          resolveDraggedReplyInputMinHeight(
            startMinHeight,
            startY,
            nextEvent.clientY,
            nextBounds,
          ),
        );
      };

      const onCancel = () => {
        finish(startMinHeight);
      };

      document.body.style.userSelect = "none";
      document.addEventListener("pointermove", onMove);
      document.addEventListener("pointerup", onUp);
      document.addEventListener("pointercancel", onCancel);
    },
    [getHeightBounds, persistMinHeight, syncTextareaHeight],
  );

  return {
    textareaRef,
    handleResizePointerDown,
  };
}
