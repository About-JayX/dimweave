import {
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import type { Attachment } from "@/types";
import { useBridgeStore } from "@/stores/bridge-store";
import { selectMessages } from "@/stores/bridge-store/selectors";
import { MessageList } from "./MessageList";
import { MessageImageLightbox } from "./MessageBubble";
import {
  filterMessagesByQuery,
  filterRenderableChatMessages,
  formatTerminalTimestamp,
  getLogsFollowOutputMode,
  getMessageSearchSummary,
  shouldAutoScrollLogsOnSurfaceChange,
} from "./view-model";
import type { ShellMainSurface } from "@/components/shell-layout-state";
import { MessageSearchChrome } from "./search-chrome";
export { MessageSearchChrome, SearchRow } from "./search-chrome";

interface MessagePanelProps {
  surfaceMode: ShellMainSurface;
  searchOpen: boolean;
  onSearchClose: () => void;
}

export function MessagePanel({ surfaceMode, searchOpen, onSearchClose }: MessagePanelProps) {
  const [lightboxAttachment, setLightboxAttachment] =
    useState<Attachment | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const searchInputRef = useRef<HTMLInputElement>(null);
  const messages = useBridgeStore(selectMessages);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);
  const claudeNeedsAttention = useBridgeStore((s) => s.claudeNeedsAttention);
  const clearClaudeAttention = useBridgeStore((s) => s.clearClaudeAttention);
  const deferredSearchQuery = useDeferredValue(searchQuery);
  const logsVirtuosoRef = useRef<VirtuosoHandle>(null);
  const previousSurfaceModeRef = useRef<ShellMainSurface | null>(surfaceMode);
  const [logsAtBottom, setLogsAtBottom] = useState(true);

  const chatMessages = useMemo(
    () => filterRenderableChatMessages(messages),
    [messages],
  );
  const filteredMessages = useMemo(
    () => filterMessagesByQuery(chatMessages, deferredSearchQuery),
    [chatMessages, deferredSearchQuery],
  );
  const searchSummary = useMemo(
    () => getMessageSearchSummary(deferredSearchQuery, filteredMessages.length),
    [deferredSearchQuery, filteredMessages.length],
  );
  const closeLightbox = useCallback(() => setLightboxAttachment(null), []);

  const handleCloseSearch = useCallback(() => {
    setSearchQuery("");
    onSearchClose();
  }, [onSearchClose]);

  useEffect(() => {
    if (searchOpen) {
      requestAnimationFrame(() => searchInputRef.current?.focus());
    }
  }, [searchOpen]);

  useEffect(() => {
    if (claudeNeedsAttention) {
      clearClaudeAttention();
    }
  }, [claudeNeedsAttention, clearClaudeAttention]);

  useEffect(() => {
    const previousSurface = previousSurfaceModeRef.current;
    if (
      shouldAutoScrollLogsOnSurfaceChange(
        previousSurface,
        surfaceMode,
        allTerminalLines.length,
      )
    ) {
      const raf = window.requestAnimationFrame(() => {
        logsVirtuosoRef.current?.scrollToIndex({
          index: "LAST",
          behavior: "auto",
        });
      });
      previousSurfaceModeRef.current = surfaceMode;
      return () => window.cancelAnimationFrame(raf);
    }

    previousSurfaceModeRef.current = surfaceMode;
  }, [allTerminalLines.length, surfaceMode]);

  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      {surfaceMode === "chat" && (
        <>
          <MessageSearchChrome
            searchOpen={searchOpen}
            searchQuery={searchQuery}
            searchSummary={searchSummary}
            inputRef={searchInputRef}
            onQueryChange={setSearchQuery}
            onClose={handleCloseSearch}
          />
          <MessageList
            messages={filteredMessages}
            emptyStateMessage={searchSummary ?? undefined}
            onOpenImage={setLightboxAttachment}
          />
        </>
      )}

      {surfaceMode === "logs" && (
        <div className="flex-1 min-h-0">
          {allTerminalLines.length === 0 && (
            <div className="py-10 text-center font-sans text-[13px] text-muted-foreground">
              No logs.
            </div>
          )}
          {allTerminalLines.length > 0 && (
            <Virtuoso
              ref={logsVirtuosoRef}
              data={allTerminalLines}
              className="h-full px-4 py-2 font-mono text-[11px] leading-relaxed"
              increaseViewportBy={160}
              atBottomStateChange={setLogsAtBottom}
              followOutput={getLogsFollowOutputMode(logsAtBottom)}
              itemContent={(_, line) => (
                <div
                  className={`py-0.5 ${line.kind === "error" ? "text-destructive" : "text-muted-foreground"}`}
                >
                  <span className="mr-2 opacity-50">
                    {formatTerminalTimestamp(line.timestamp)}
                  </span>
                  <span className="mr-1 text-secondary-foreground">
                    [{line.agent}]
                  </span>
                  {line.line}
                </div>
              )}
            />
          )}
        </div>
      )}

      {surfaceMode === "chat" && lightboxAttachment ? (
        <MessageImageLightbox
          attachment={lightboxAttachment}
          onClose={closeLightbox}
        />
      ) : null}
    </div>
  );
}
