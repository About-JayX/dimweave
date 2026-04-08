import { memo, useEffect } from "react";
import { Paperclip } from "lucide-react";
import { MessageMarkdown } from "@/components/MessageMarkdown";
import { SourceBadge } from "./SourceBadge";
import type { Attachment, BridgeMessage } from "@/types";
import { getMessageIdentityPresentation } from "./view-model";
import { getMessageSurfacePresentation } from "./surface-styles";

export function areMessageBubblePropsEqual(
  prev: { msg: BridgeMessage },
  next: { msg: BridgeMessage },
): boolean {
  return (
    prev.msg.id === next.msg.id &&
    prev.msg.from === next.msg.from &&
    prev.msg.to === next.msg.to &&
    prev.msg.content === next.msg.content &&
    prev.msg.timestamp === next.msg.timestamp &&
    prev.msg.displaySource === next.msg.displaySource &&
    prev.msg.attachments?.length === next.msg.attachments?.length
  );
}

function resolveAttachmentSrc(filePath: string) {
  if (
    typeof window === "undefined" ||
    !window.__TAURI_INTERNALS__?.convertFileSrc
  ) {
    return filePath;
  }
  return window.__TAURI_INTERNALS__.convertFileSrc(filePath, "asset");
}

export function MessageImageLightbox({
  attachment,
  onClose,
}: {
  attachment: Attachment;
  onClose: () => void;
}) {
  useEffect(() => {
    if (typeof document === "undefined") {
      return;
    }
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div
      role="dialog"
      aria-modal="true"
      className="absolute inset-0 z-20 flex items-center justify-center bg-background/88 px-4 py-6 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="w-full max-w-5xl rounded-2xl border border-border/40 bg-card/95 p-4 shadow-2xl"
        onClick={(event) => event.stopPropagation()}
      >
        <div className="mb-3 flex items-center justify-between gap-3">
          <div>
            <div className="text-[11px] uppercase tracking-[0.18em] text-muted-foreground/55">
              Image preview
            </div>
            <div className="mt-0.5 text-sm font-medium text-foreground">
              {attachment.fileName}
            </div>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="rounded-full border border-border/45 px-3 py-1 text-[11px] font-medium text-muted-foreground hover:border-border/70 hover:text-foreground"
          >
            Close preview
          </button>
        </div>
        <img
          src={resolveAttachmentSrc(attachment.filePath)}
          alt={attachment.fileName}
          className="max-h-[78vh] w-full rounded-xl border border-border/35 object-contain"
        />
      </div>
    </div>
  );
}

export function MessageBubbleView({
  msg,
  onOpenImage,
}: {
  msg: BridgeMessage;
  onOpenImage?: (attachment: Attachment) => void;
}) {
  const isUser = msg.from === "user";
  const { badgeSource, roleLabel } = getMessageIdentityPresentation(msg);
  const surface = getMessageSurfacePresentation(badgeSource);
  return (
    <div
      className={`flex py-1.5 msg-enter ${isUser ? "justify-end" : "justify-start"}`}
    >
      <div
        className={`max-w-[82%] rounded-xl px-3.5 py-2.5 ${surface.containerClass}`}
      >
        <div
          className={`flex items-center gap-1.5 mb-0.5 ${isUser ? "justify-end" : ""}`}
        >
          <SourceBadge source={badgeSource} />
          {roleLabel && (
            <span className="text-[10px] text-muted-foreground/50">
              {roleLabel}
            </span>
          )}
          <span className="text-[10px] text-muted-foreground/30">
            {new Date(msg.timestamp).toLocaleTimeString()}
          </span>
        </div>
        <MessageMarkdown content={msg.content} />
        {msg.attachments && msg.attachments.length > 0 && (
          <div className="mt-1.5 flex flex-wrap gap-1.5">
            {msg.attachments.map((att, i) =>
              att.isImage ? (
                <button
                  key={`${att.filePath}-${i}`}
                  type="button"
                  onClick={() => onOpenImage?.(att)}
                  className="group relative overflow-hidden rounded-lg border border-border/30"
                >
                  <img
                    src={resolveAttachmentSrc(att.filePath)}
                    alt={att.fileName}
                    className="max-h-48 max-w-64 object-cover transition-transform group-hover:scale-[1.01]"
                  />
                  <span className="sr-only">Open image {att.fileName}</span>
                </button>
              ) : (
                <span
                  key={`${att.filePath}-${i}`}
                  className="inline-flex items-center gap-1 rounded-md border border-border/40 bg-muted/30 px-2 py-0.5 text-[11px] text-muted-foreground"
                >
                  <Paperclip className="size-3" />
                  {att.fileName}
                </span>
              ),
            )}
          </div>
        )}
      </div>
    </div>
  );
}

export const MessageBubble = memo(
  MessageBubbleView,
  (
    prev: {
      msg: BridgeMessage;
      onOpenImage?: (attachment: Attachment) => void;
    },
    next: {
      msg: BridgeMessage;
      onOpenImage?: (attachment: Attachment) => void;
    },
  ) =>
    areMessageBubblePropsEqual(prev, next) &&
    prev.onOpenImage === next.onOpenImage,
);
MessageBubble.displayName = "MessageBubble";
