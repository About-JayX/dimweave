import { Paperclip, Send } from "lucide-react";
import { Button } from "@/components/ui/button";
import { TargetPicker, type Target } from "./TargetPicker";

type ReplyInputFooterProps = {
  target: Target;
  setTarget: (target: Target) => void;
  onPickFiles: () => void;
  activeTaskTitle: string | null;
  taskSessionWarning: string | null;
  connected: boolean;
  sendOnEnter: boolean;
  onToggleSendOnEnter: () => void;
  isMac: boolean;
  canSend: boolean;
  onSend: () => void;
};

export function ReplyInputFooter({
  target,
  setTarget,
  onPickFiles,
  activeTaskTitle,
  taskSessionWarning,
  connected,
  sendOnEnter,
  onToggleSendOnEnter,
  isMac,
  canSend,
  onSend,
}: ReplyInputFooterProps) {
  return (
    <div className="flex items-center justify-between gap-2 border-t border-border/35 px-3 py-2">
      <div className="flex min-w-0 items-center gap-2">
        <TargetPicker target={target} setTarget={setTarget} />
        <button
          onClick={onPickFiles}
          className="flex size-7 items-center justify-center rounded-full text-muted-foreground/60 transition-colors hover:bg-muted hover:text-foreground"
          title="Attach files"
        >
          <Paperclip className="size-3.5" />
        </button>
        {activeTaskTitle ? (
          <span className="truncate text-[10px] text-foreground/80">
            {activeTaskTitle}
          </span>
        ) : (
          <span className="text-[10px] text-muted-foreground/55">No active task</span>
        )}
        {taskSessionWarning ? (
          <span className="rounded-full border border-amber-500/25 bg-amber-500/10 px-2 py-0.5 text-[10px] text-amber-600">
            {taskSessionWarning}
          </span>
        ) : !connected ? (
          <span className="rounded-full border border-destructive/25 bg-destructive/10 px-2 py-0.5 text-[10px] text-destructive">
            Disconnected
          </span>
        ) : null}
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <button
          onClick={onToggleSendOnEnter}
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
          disabled={!canSend}
          onClick={onSend}
          className="h-7 gap-1.5 rounded-full px-3 text-[11px]"
        >
          <Send className="size-3" />
          Send
        </Button>
      </div>
    </div>
  );
}
