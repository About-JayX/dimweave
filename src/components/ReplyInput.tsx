import { useCallback, useRef, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useBridgeStore } from "@/stores/bridge-store";
import { selectAnyAgentConnected } from "@/stores/bridge-store/selectors";
import { useTaskStore } from "@/stores/task-store";
import { selectActiveTask } from "@/stores/task-store/selectors";
import { ReviewGateBadge } from "@/components/TaskPanel/ReviewGateBadge";
import { getReviewBadge } from "@/components/TaskPanel/view-model";
import { Send, ChevronDown } from "lucide-react";

const MIN_ROWS = 2;
const MAX_ROWS = 8;
const TARGETS = ["auto", "lead", "coder", "reviewer"] as const;
type Target = (typeof TARGETS)[number];

const TARGET_COLORS: Record<Target, string> = {
  auto: "text-purple-400 border-purple-400/30",
  lead: "text-yellow-400 border-yellow-400/30",
  coder: "text-emerald-400 border-emerald-400/30",
  reviewer: "text-orange-400 border-orange-400/30",
};

export function ReplyInput() {
  const connected = useBridgeStore(selectAnyAgentConnected);
  const draft = useBridgeStore((s) => s.draft);
  const setDraft = useBridgeStore((s) => s.setDraft);
  const sendToCodex = useBridgeStore((s) => s.sendToCodex);
  const [target, setTarget] = useState<Target>("auto");
  const [showPicker, setShowPicker] = useState(false);
  const [sendOnEnter, setSendOnEnter] = useState(true);
  const composingRef = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const pickerRef = useRef<HTMLDivElement>(null);
  const activeTask = useTaskStore(selectActiveTask);
  const reviewBadge = getReviewBadge(activeTask?.reviewStatus);

  const handleSend = useCallback(() => {
    const trimmed = draft.trim();
    if (!trimmed || !connected) return;
    sendToCodex(trimmed, target);
    setDraft("");
  }, [draft, connected, sendToCodex, setDraft, target]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (
        composingRef.current ||
        e.nativeEvent.isComposing ||
        e.keyCode === 229
      )
        return;
      if (e.key === "Enter") {
        if (sendOnEnter) {
          if (e.shiftKey) return; // Shift+Enter = newline
          e.preventDefault();
          handleSend();
        } else if (e.metaKey || e.ctrlKey) {
          e.preventDefault();
          handleSend();
        }
      }
    },
    [handleSend, sendOnEnter],
  );

  const autosize = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 20;
    const pt = parseFloat(getComputedStyle(el).paddingTop) || 0;
    const pb = parseFloat(getComputedStyle(el).paddingBottom) || 0;
    const minH = MIN_ROWS * lineHeight + pt + pb;
    const maxH = MAX_ROWS * lineHeight + pt + pb;
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

  // Close picker on outside click
  useEffect(() => {
    if (!showPicker) return;
    const handler = (e: MouseEvent) => {
      if (pickerRef.current && !pickerRef.current.contains(e.target as Node))
        setShowPicker(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [showPicker]);
  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/.test(navigator.userAgent);

  return (
    <div className="relative px-4 py-3">
      <div className="rounded-xl border border-border/50 bg-card/85 focus-within:border-primary/35 focus-within:ring-1 focus-within:ring-primary/15 transition-colors">
        <textarea
          ref={textareaRef}
          className="block w-full min-h-[44px] resize-none bg-transparent px-5 py-3 text-[13px] leading-relaxed text-foreground outline-none placeholder:text-muted-foreground"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={handleKeyDown}
          onCompositionStart={() => {
            composingRef.current = true;
          }}
          onCompositionEnd={() => {
            composingRef.current = false;
          }}
          placeholder="Describe the next step, ask for a review, or route a task to an agent."
          rows={MIN_ROWS}
        />

        <div className="flex items-center justify-between gap-2 border-t border-border/35 px-3 py-2">
          <div className="flex min-w-0 items-center gap-2">
            <div className="relative shrink-0" ref={pickerRef}>
              <button
                onClick={() => setShowPicker(!showPicker)}
                className={`flex items-center gap-1 rounded-full border px-2.5 py-1 text-[10px] font-medium transition-colors ${TARGET_COLORS[target]}`}
              >
                To {target}
                <ChevronDown className="size-3 opacity-60" />
              </button>
              {showPicker && (
                <div className="absolute bottom-full left-0 z-50 mb-2 min-w-[110px] rounded-xl border border-border bg-popover py-1 shadow-xl">
                  {TARGETS.map((t) => (
                    <button
                      key={t}
                      onClick={() => {
                        setTarget(t);
                        setShowPicker(false);
                      }}
                      className={`block w-full px-3 py-1.5 text-left text-[11px] transition-colors hover:bg-accent ${t === target ? "font-bold" : ""} ${TARGET_COLORS[t].split(" ")[0]}`}
                    >
                      {t}
                    </button>
                  ))}
                </div>
              )}
            </div>
            {activeTask ? (
              <span className="truncate text-[10px] text-foreground/80">
                {activeTask.title}
              </span>
            ) : (
              <span className="text-[10px] text-muted-foreground/55">
                No active task
              </span>
            )}
            {activeTask && reviewBadge && (
              <ReviewGateBadge badge={reviewBadge} />
            )}
            {!connected && (
              <span className="rounded-full border border-destructive/25 bg-destructive/10 px-2 py-0.5 text-[10px] text-destructive">
                Disconnected
              </span>
            )}
          </div>

          <div className="flex items-center gap-2 shrink-0">
            <button
              onClick={() => setSendOnEnter((v) => !v)}
              className="rounded-full border border-border/35 px-2 py-0.5 text-[10px] text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
              title={
                sendOnEnter
                  ? "Click to switch: ⌘+Enter to send"
                  : "Click to switch: Enter to send"
              }
            >
              {sendOnEnter ? "Enter ↵" : `${isMac ? "⌘" : "Ctrl"}+Enter`}
            </button>
            <Button
              size="sm"
              disabled={!connected || !draft.trim()}
              onClick={handleSend}
              className="h-7 gap-1.5 rounded-full px-3 text-[11px]"
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
