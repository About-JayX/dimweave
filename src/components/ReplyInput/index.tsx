import { useCallback, useRef, useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useBridgeStore } from "@/stores/bridge-store";
import { selectAnyAgentConnected } from "@/stores/bridge-store/selectors";
import { useTaskStore } from "@/stores/task-store";
import { selectActiveTask } from "@/stores/task-store/selectors";
import { ReviewGateBadge } from "@/components/TaskPanel/ReviewGateBadge";
import { getReviewBadge } from "@/components/TaskPanel/view-model";
import { Send, Paperclip } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { TargetPicker, type Target } from "./TargetPicker";
import { AttachmentStrip } from "./AttachmentStrip";
import { useAttachments } from "./use-attachments";

const MIN_ROWS = 2;
const MAX_ROWS = 8;

export function ReplyInput() {
  const connected = useBridgeStore(selectAnyAgentConnected);
  const draft = useBridgeStore((s) => s.draft);
  const setDraft = useBridgeStore((s) => s.setDraft);
  const sendToCodex = useBridgeStore((s) => s.sendToCodex);
  const [target, setTarget] = useState<Target>("auto");
  const [sendOnEnter, setSendOnEnter] = useState(true);
  const [dragOver, setDragOver] = useState(false);
  const composingRef = useRef(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const activeTask = useTaskStore(selectActiveTask);
  const reviewBadge = getReviewBadge(activeTask?.reviewStatus);
  const { attachments, addFiles, removeAt, clear } = useAttachments();

  const handleSend = useCallback(() => {
    const trimmed = draft.trim();
    if (!trimmed || !connected) return;
    sendToCodex(
      trimmed,
      target,
      attachments.length > 0 ? attachments : undefined,
    );
    setDraft("");
    clear();
  }, [draft, connected, sendToCodex, setDraft, target, attachments, clear]);

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
          if (e.shiftKey) return;
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

  const handlePickFiles = useCallback(async () => {
    const paths = await invoke<string[] | null>("pick_files");
    if (paths) addFiles(paths);
  }, [addFiles]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "over") setDragOver(true);
        else if (event.payload.type === "drop") {
          setDragOver(false);
          if (event.payload.paths.length > 0) addFiles(event.payload.paths);
        } else setDragOver(false);
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => {
      unlisten?.();
    };
  }, [addFiles]);

  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/.test(navigator.userAgent);

  return (
    <div className="relative px-4 py-3">
      <div
        className={`rounded-xl border bg-card/85 transition-colors focus-within:border-primary/35 focus-within:ring-1 focus-within:ring-primary/15 ${dragOver ? "border-primary/50 ring-2 ring-primary/20" : "border-border/50"}`}
      >
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

        <AttachmentStrip attachments={attachments} onRemove={removeAt} />

        <div className="flex items-center justify-between gap-2 border-t border-border/35 px-3 py-2">
          <div className="flex min-w-0 items-center gap-2">
            <TargetPicker target={target} setTarget={setTarget} />
            <button
              onClick={handlePickFiles}
              className="flex size-7 items-center justify-center rounded-full text-muted-foreground/60 transition-colors hover:bg-muted hover:text-foreground"
              title="Attach files"
            >
              <Paperclip className="size-3.5" />
            </button>
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
