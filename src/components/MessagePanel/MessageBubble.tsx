import { memo } from "react";
import { MessageMarkdown } from "@/components/MessageMarkdown";
import { SourceBadge } from "./SourceBadge";
import type { BridgeMessage } from "@/types";
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
    prev.msg.displaySource === next.msg.displaySource
  );
}

function MessageBubbleInner({ msg }: { msg: BridgeMessage }) {
  const isUser = msg.from === "user";
  const { badgeSource, roleLabel } = getMessageIdentityPresentation(msg);
  const surface = getMessageSurfacePresentation(isUser);
  return (
    <div
      className={`flex py-2.5 msg-enter ${isUser ? "justify-end" : "justify-start"}`}
    >
      <div className={`max-w-[80%] rounded-xl px-3 py-2.5 ${surface.containerClass}`}>
        <div
          className={`flex items-center gap-2 mb-1 ${isUser ? "justify-end" : ""}`}
        >
          <SourceBadge source={badgeSource} />
          {roleLabel ? (
            <span className="font-mono text-[10px] uppercase tracking-[0.18em] text-muted-foreground/70">
              {roleLabel}
            </span>
          ) : null}
          <span className="font-mono text-[10px] text-muted-foreground/75">
            {new Date(msg.timestamp).toLocaleTimeString()}
          </span>
        </div>
        <MessageMarkdown content={msg.content} />
      </div>
    </div>
  );
}

export const MessageBubble = memo(MessageBubbleInner, areMessageBubblePropsEqual);
MessageBubble.displayName = "MessageBubble";
