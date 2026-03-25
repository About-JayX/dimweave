import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { MessageMarkdown } from "@/components/MessageMarkdown";
import { useBridgeStore } from "@/stores/bridge-store";
import type { BridgeMessage } from "@/types";
import { PermissionQueue } from "./PermissionQueue";
import { SourceBadge } from "./SourceBadge";
import { TabBtn } from "./TabBtn";

type Tab = "messages" | "logs" | "approvals";

interface MessagePanelProps {
  messages: BridgeMessage[];
  onTabChange?: (tab: Tab) => void;
}

export function MessagePanel({ messages, onTabChange }: MessagePanelProps) {
  const [tab, setTabState] = useState<Tab>("messages");
  const setTab = (t: Tab) => {
    setTabState(t);
    onTabChange?.(t);
  };
  const bottomRef = useRef<HTMLDivElement>(null);
  const logRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  const clearMessages = useBridgeStore((s) => s.clearMessages);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);
  const permissionPrompts = useBridgeStore((s) => s.permissionPrompts);
  const respondToPermission = useBridgeStore((s) => s.respondToPermission);

  const chatMessages = useMemo(
    () => messages.filter((m) => m.from !== "system"),
    [messages],
  );
  const errorLines = useMemo(
    () => allTerminalLines.filter((l) => l.kind === "error"),
    [allTerminalLines],
  );

  // Smart auto-scroll: only if user is near the bottom
  const isNearBottom = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return true;
    return el.scrollHeight - el.scrollTop - el.clientHeight < 100;
  }, []);

  useEffect(() => {
    if (tab === "messages" && isNearBottom()) {
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages, tab, isNearBottom]);

  return (
    <div className="flex flex-1 flex-col min-h-0">
      {/* Tabs */}
      <div className="flex items-center px-4 py-2 border-b border-border/50 gap-3 relative">
        <TabBtn active={tab === "messages"} onClick={() => setTab("messages")}>
          Messages ({chatMessages.length})
        </TabBtn>
        <TabBtn active={tab === "logs"} onClick={() => setTab("logs")}>
          Logs {errorLines.length > 0 && `(${errorLines.length})`}
        </TabBtn>
        <TabBtn
          active={tab === "approvals"}
          onClick={() => setTab("approvals")}
        >
          Approvals
          {permissionPrompts.length > 0 && ` (${permissionPrompts.length})`}
        </TabBtn>
        <div className="flex-1" />
        {tab !== "approvals" && (
          <Button variant="secondary" size="xs" onClick={clearMessages}>
            Clear
          </Button>
        )}
        <div className="absolute bottom-0 left-0 right-0 h-px bg-linear-to-r from-transparent via-primary/15 to-transparent" />
      </div>

      {/* Messages */}
      {tab === "messages" && (
        <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-2">
          {chatMessages.length === 0 && (
            <div className="py-10 text-center text-[13px] text-muted-foreground animate-in fade-in duration-500">
              No messages yet. Connect Claude and Codex to start bridging.
            </div>
          )}
          {chatMessages.map((msg) => {
            const isUser = msg.from === "user";
            return (
              <div
                key={msg.id}
                className={`flex py-2.5 msg-enter ${isUser ? "justify-end" : "justify-start"}`}
              >
                <div
                  className={`max-w-[80%] rounded-xl px-3 py-2 ${
                    isUser
                      ? "bg-sky-500/15 border border-sky-500/30"
                      : "bg-card/60 border border-border/50"
                  }`}
                >
                  <div
                    className={`flex items-center gap-2 mb-1 ${isUser ? "justify-end" : ""}`}
                  >
                    <SourceBadge source={msg.from} />
                    <span className="font-mono text-[11px] text-muted-foreground">
                      {new Date(msg.timestamp).toLocaleTimeString()}
                    </span>
                  </div>
                  <MessageMarkdown content={msg.content} />
                </div>
              </div>
            );
          })}
          <div ref={bottomRef} />
        </div>
      )}

      {/* Logs */}
      {tab === "logs" && (
        <div
          ref={logRef}
          className="flex-1 overflow-y-auto px-4 py-2 font-mono text-[11px] leading-relaxed"
        >
          {allTerminalLines.length === 0 && (
            <div className="py-10 text-center text-[13px] text-muted-foreground font-sans">
              No logs.
            </div>
          )}
          {allTerminalLines.map((l) => (
            <div
              key={l.id}
              className={`py-0.5 ${l.kind === "error" ? "text-destructive" : "text-muted-foreground"}`}
            >
              <span className="opacity-50 mr-2">
                {new Date(l.timestamp).toLocaleTimeString()}
              </span>
              <span className="mr-1 text-secondary-foreground">
                [{l.agent}]
              </span>
              {l.line}
            </div>
          ))}
        </div>
      )}

      {tab === "approvals" && (
        <PermissionQueue
          prompts={permissionPrompts}
          onResolve={respondToPermission}
        />
      )}
    </div>
  );
}
