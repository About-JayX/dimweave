import { useCallback, useRef, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { useBridgeStore } from "@/stores/bridge-store";
import { Send } from "lucide-react";

const MIN_ROWS = 2;
const MAX_ROWS = 8;

interface ReplyInputProps {
  connected: boolean;
}

export function ReplyInput({ connected }: ReplyInputProps) {
  const draft = useBridgeStore((s) => s.draft);
  const setDraft = useBridgeStore((s) => s.setDraft);
  const sendToCodex = useBridgeStore((s) => s.sendToCodex);
  const composingRef = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleSend = useCallback(() => {
    const trimmed = draft.trim();
    if (!trimmed || !connected) return;
    sendToCodex(trimmed);
    setDraft("");
  }, [draft, connected, sendToCodex, setDraft]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      // Never send during IME composition
      if (
        composingRef.current ||
        e.nativeEvent.isComposing ||
        e.keyCode === 229
      )
        return;

      // Cmd/Ctrl+Enter to send
      if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        handleSend();
      }
      // Plain Enter and Shift+Enter = newline (default textarea behavior)
    },
    [handleSend],
  );

  // Autosize: adjust textarea height based on content and window size
  const autosize = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    // Measure actual line height from computed style
    const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 20;
    const paddingTop = parseFloat(getComputedStyle(el).paddingTop) || 0;
    const paddingBottom = parseFloat(getComputedStyle(el).paddingBottom) || 0;
    const padding = paddingTop + paddingBottom;
    const minH = MIN_ROWS * lineHeight + padding;
    const maxH = MAX_ROWS * lineHeight + padding;
    el.style.height = "auto";
    el.style.height = `${Math.min(Math.max(el.scrollHeight, minH), maxH)}px`;
  }, []);

  useEffect(() => {
    autosize();
  }, [draft, autosize]);

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout>;
    const debounced = () => {
      clearTimeout(timer);
      timer = setTimeout(autosize, 100);
    };
    window.addEventListener("resize", debounced);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("resize", debounced);
    };
  }, [autosize]);

  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/.test(navigator.userAgent);
  const modKey = isMac ? "⌘" : "Ctrl";

  return (
    <div className="border-t border-border px-4 py-3">
      <div className="rounded-lg border border-input bg-card focus-within:border-ring focus-within:ring-1 focus-within:ring-ring/30 transition-colors">
        {/* Textarea */}
        <textarea
          ref={textareaRef}
          className="block w-full resize-none bg-transparent px-3 pt-2.5 pb-1 text-[13px] leading-relaxed text-foreground font-[inherit] outline-none placeholder:text-muted-foreground"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={handleKeyDown}
          onCompositionStart={() => {
            composingRef.current = true;
          }}
          onCompositionEnd={() => {
            composingRef.current = false;
          }}
          placeholder="Type your message..."
          aria-label="Message to Codex"
          name="message"
          autoComplete="off"
          rows={MIN_ROWS}
        />

        {/* Toolbar */}
        <div className="flex items-center justify-between px-3 py-1.5">
          <div className="flex items-center gap-2">
            <Badge
              variant="outline"
              className="text-[10px] font-normal text-codex border-codex/30"
            >
              To Codex
            </Badge>
            {!connected && (
              <span className="text-[10px] text-destructive">Disconnected</span>
            )}
          </div>
          <div className="flex items-center gap-2">
            <span className="text-[10px] text-muted-foreground">
              {modKey}+Enter to send
            </span>
            <Button
              size="sm"
              disabled={!connected || !draft.trim()}
              onClick={handleSend}
              className="h-7 gap-1.5 px-3 text-[12px]"
            >
              <Send className="size-3" />
              Send
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
