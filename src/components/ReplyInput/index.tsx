import { useCallback, useEffect, useRef, useState } from "react";
import { useBridgeStore } from "@/stores/bridge-store";
import { selectAnyAgentConnected } from "@/stores/bridge-store/selectors";
import { useTaskStore } from "@/stores/task-store";
import {
  selectActiveTask,
  selectActiveReplyTarget,
  selectActiveTaskSessions,
} from "@/stores/task-store/selectors";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { hasMessagePayload } from "@/lib/message-payload";
import type { Target } from "./TargetPicker";
import { AttachmentStrip } from "./AttachmentStrip";
import { createAsyncUnlistenCleanup } from "./async-unlisten";
import { REPLY_INPUT_MIN_ROWS } from "./height";
import { ReplyInputFooter } from "./Footer";
import { getTaskSessionWarning } from "./task-session-guard";
import { useReplyInputResizer } from "./use-reply-input-resizer";
import { collectPastedAttachmentPaths } from "./paste-attachments";
import { useAttachments } from "./use-attachments";

export function ReplyInput() {
  const connected = useBridgeStore(selectAnyAgentConnected);
  const agents = useBridgeStore((s) => s.agents);
  const claudeRole = useBridgeStore((s) => s.claudeRole);
  const codexRole = useBridgeStore((s) => s.codexRole);
  const draft = useBridgeStore((s) => s.draft);
  const setDraft = useBridgeStore((s) => s.setDraft);
  const sendToCodex = useBridgeStore((s) => s.sendToCodex);
  const target = useTaskStore(selectActiveReplyTarget);
  const setReplyTarget = useTaskStore((s) => s.setReplyTarget);
  const [sendOnEnter, setSendOnEnter] = useState(true);
  const [dragOver, setDragOver] = useState(false);
  const composingRef = useRef(false);
  const { textareaRef, handleResizePointerDown } = useReplyInputResizer(draft);
  const activeTask = useTaskStore(selectActiveTask);
  const activeTaskSessions = useTaskStore(selectActiveTaskSessions);
  const { attachments, addFiles, removeAt, clear } = useAttachments();
  const taskSessionWarning = getTaskSessionWarning({
    target,
    activeTask,
    sessions: activeTaskSessions,
    agents,
    claudeRole,
    codexRole,
  });
  const canSend =
    !!activeTask &&
    connected &&
    !taskSessionWarning &&
    hasMessagePayload(draft, attachments);

  const handleSend = useCallback(() => {
    const trimmed = draft.trim();
    if (!hasMessagePayload(trimmed, attachments) || !canSend) return;
    sendToCodex(
      trimmed,
      target,
      attachments.length > 0 ? attachments : undefined,
      activeTask?.taskId,
    );
    setDraft("");
    clear();
  }, [attachments, canSend, clear, draft, sendToCodex, setDraft, target]);

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

  const handlePaste = useCallback(() => {
    void collectPastedAttachmentPaths().then((paths) => {
      if (paths.length > 0) addFiles(paths);
    });
  }, [addFiles]);

  const handlePickFiles = useCallback(async () => {
    const paths = await invoke<string[] | null>("pick_files");
    if (paths) addFiles(paths);
  }, [addFiles]);

  const addFilesRef = useRef(addFiles);
  addFilesRef.current = addFiles;
  useEffect(() => {
    return createAsyncUnlistenCleanup(() =>
      getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over") setDragOver(true);
        else if (event.payload.type === "drop") {
          setDragOver(false);
          if (event.payload.paths.length > 0)
            addFilesRef.current(event.payload.paths);
        } else setDragOver(false);
      }),
    );
  }, []);

  const isMac =
    typeof navigator !== "undefined" &&
    /Mac|iPhone|iPad/.test(navigator.userAgent);

  return (
    <div className="relative px-4 py-3">
      <div
        className={`relative rounded-xl border bg-card/85 transition-colors focus-within:border-primary/35 focus-within:ring-1 focus-within:ring-primary/15 ${dragOver ? "border-primary/50 ring-2 ring-primary/20" : "border-border/50"}`}
      >
        <div
          data-reply-input-resize-handle="true"
          onPointerDown={handleResizePointerDown}
          className="group absolute left-1/2 top-0 z-10 flex h-3 w-14 -translate-x-1/2 touch-none items-start justify-center pt-1"
          title="Resize input"
          aria-label="Resize input"
        >
          <span
            data-reply-input-resize-grip="true"
            className="h-1 w-8 rounded-full bg-border/70 transition-colors group-hover:bg-muted-foreground/35 group-active:bg-primary/35"
          />
        </div>
        <textarea
          ref={textareaRef}
          disabled={!activeTask}
          className="block w-full min-h-[44px] resize-none bg-transparent px-5 py-3 text-[13px] leading-relaxed text-foreground outline-none placeholder:text-muted-foreground disabled:cursor-not-allowed disabled:opacity-50"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onPaste={handlePaste}
          onKeyDown={handleKeyDown}
          onCompositionStart={() => {
            composingRef.current = true;
          }}
          onCompositionEnd={() => {
            composingRef.current = false;
          }}
          placeholder="Describe the next step, ask for a review, or route a task to an agent."
          rows={REPLY_INPUT_MIN_ROWS}
        />

        <AttachmentStrip attachments={attachments} onRemove={removeAt} />

        <ReplyInputFooter
          target={target}
          setTarget={setReplyTarget}
          onPickFiles={handlePickFiles}
          activeTaskTitle={activeTask?.title ?? null}
          taskSessionWarning={taskSessionWarning}
          connected={connected}
          sendOnEnter={sendOnEnter}
          onToggleSendOnEnter={() => setSendOnEnter((value) => !value)}
          isMac={isMac}
          canSend={canSend}
          onSend={handleSend}
        />
      </div>
    </div>
  );
}
