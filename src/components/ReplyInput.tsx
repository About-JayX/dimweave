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
      if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend],
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
    <div className="border-t border-border/50 px-4 py-3 relative">
      <div className="absolute top-0 left-4 right-4 h-px bg-linear-to-r from-transparent via-primary/10 to-transparent" />
      <div className="rounded-lg border border-input bg-card/80 backdrop-blur-sm focus-within:border-claude/50 focus-within:ring-1 focus-within:ring-claude/20 transition-all duration-300">
        <textarea
          ref={textareaRef}
          className="block w-full resize-none bg-transparent px-3 pt-2.5 pb-1 text-[13px] leading-relaxed text-foreground outline-none placeholder:text-muted-foreground"
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
          rows={MIN_ROWS}
        />
        <div className="flex items-center justify-between px-3 py-1.5">
          <div className="flex min-w-0 items-center gap-2">
            <div className="relative shrink-0" ref={pickerRef}>
              <button
                onClick={() => setShowPicker(!showPicker)}
                className={`flex items-center gap-1 rounded border px-2 py-0.5 text-[10px] font-medium transition-colors ${TARGET_COLORS[target]}`}
              >
                To {target}
                <ChevronDown className="size-3 opacity-60" />
              </button>
              {showPicker && (
                <div className="absolute bottom-full left-0 z-20 mb-1 min-w-[100px] rounded-md border border-border bg-popover py-1 shadow-lg">
                  {TARGETS.map((t) => (
                    <button
                      key={t}
                      onClick={() => {
                        setTarget(t);
                        setShowPicker(false);
                      }}
                      className={`block w-full px-3 py-1 text-left text-[11px] transition-colors hover:bg-accent ${t === target ? "font-bold" : ""} ${TARGET_COLORS[t].split(" ")[0]}`}
                    >
                      {t}
                    </button>
                  ))}
                </div>
              )}
            </div>
            {activeTask ? (
              <div className="min-w-0 space-y-0.5">
                <div className="truncate text-[10px] font-medium text-foreground/85">
                  {activeTask.title}
                </div>
                <div className="flex items-center gap-2">
                  <span className="truncate text-[10px] text-muted-foreground/55">
                    {activeTask.workspaceRoot}
                  </span>
                  {reviewBadge && <ReviewGateBadge badge={reviewBadge} />}
                </div>
              </div>
            ) : (
              <span className="text-[10px] text-muted-foreground/55">
                No active task
              </span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {!connected && (
              <span className="text-[10px] text-destructive">Disconnected</span>
            )}
            <span className="text-[10px] text-muted-foreground">
              {isMac ? "⌘" : "Ctrl"}+Enter
            </span>
            <Button
              size="sm"
              disabled={!connected || !draft.trim()}
              onClick={handleSend}
              className="h-7 gap-1.5 px-3 text-[12px] hover:shadow-[0_0_12px_#8b5cf630] active:scale-[0.96] transition-all"
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
