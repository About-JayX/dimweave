import {
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Search, X } from "lucide-react";
import { Virtuoso } from "react-virtuoso";
import type { Attachment } from "@/types";
import { useBridgeStore } from "@/stores/bridge-store";
import { selectMessages } from "@/stores/bridge-store/selectors";
import { MessageList } from "./MessageList";
import { MessageImageLightbox } from "./MessageBubble";
import {
  filterMessagesByQuery,
  filterRenderableChatMessages,
  formatTerminalTimestamp,
  getMessageSearchSummary,
} from "./view-model";
import type { ShellMainSurface } from "@/components/shell-layout-state";

interface MessagePanelProps {
  surfaceMode: ShellMainSurface;
}

export function MessagePanel({ surfaceMode }: MessagePanelProps) {
  const [lightboxAttachment, setLightboxAttachment] =
    useState<Attachment | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const searchInputRef = useRef<HTMLInputElement>(null);
  const messages = useBridgeStore(selectMessages);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);
  const claudeNeedsAttention = useBridgeStore((s) => s.claudeNeedsAttention);
  const clearClaudeAttention = useBridgeStore((s) => s.clearClaudeAttention);
  const deferredSearchQuery = useDeferredValue(searchQuery);

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

  useEffect(() => {
    if (claudeNeedsAttention) {
      clearClaudeAttention();
    }
  }, [claudeNeedsAttention, clearClaudeAttention]);

  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      {surfaceMode === "chat" && (
        <>
          {chatMessages.length > 0 && (
            <div className="flex items-center border-b border-border/35 px-4 py-1.5">
              {searchOpen ? (
                <div className="flex flex-1 items-center gap-2">
                  <input
                    ref={searchInputRef}
                    aria-label="Search messages"
                    type="search"
                    value={searchQuery}
                    onChange={(event) => setSearchQuery(event.target.value)}
                    placeholder="Search messages"
                    className="flex-1 rounded-lg border border-border/45 bg-background/65 px-3 py-1.5 text-[13px] text-foreground outline-none transition-colors focus:border-primary/50"
                    // eslint-disable-next-line jsx-a11y/no-autofocus
                    autoFocus
                  />
                  <button
                    type="button"
                    onClick={() => {
                      setSearchQuery("");
                      setSearchOpen(false);
                    }}
                    className="shrink-0 rounded-md p-1 text-muted-foreground hover:text-foreground"
                    aria-label="Close search"
                  >
                    <X className="size-4" />
                  </button>
                </div>
              ) : (
                <button
                  type="button"
                  onClick={() => {
                    setSearchOpen(true);
                    requestAnimationFrame(() =>
                      searchInputRef.current?.focus(),
                    );
                  }}
                  className="rounded-md p-1 text-muted-foreground/50 hover:text-foreground transition-colors"
                  aria-label="Search messages"
                >
                  <Search className="size-4" />
                </button>
              )}
              {searchOpen && searchSummary ? (
                <span className="ml-2 shrink-0 text-[11px] text-muted-foreground/70">
                  {searchSummary}
                </span>
              ) : null}
            </div>
          )}
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
              data={allTerminalLines}
              className="h-full px-4 py-2 font-mono text-[11px] leading-relaxed"
              increaseViewportBy={160}
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
