import { useState, useEffect, useRef } from "react";
import { Button } from "@/components/ui/button";
import { MessageMarkdown } from "@/components/MessageMarkdown";
import { useBridgeStore, type TerminalLine } from "@/stores/bridge-store";
import type { BridgeMessage } from "@/types";
import { SourceBadge } from "./SourceBadge";
import { TabBtn } from "./TabBtn";
import { TerminalView } from "./TerminalView";

type Tab = "messages" | "terminal" | "logs";

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

  const clearMessages = useBridgeStore((s) => s.clearMessages);
  const codexPhase = useBridgeStore((s) => s.codexPhase);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);

  const chatMessages = messages.filter((m) => m.from !== "system");

  const errorLines: TerminalLine[] = [];
  for (const l of allTerminalLines) {
    if (l.kind === "error") errorLines.push(l);
  }

  // Scroll messages
  useEffect(() => {
    if (tab === "messages")
      bottomRef.current?.scrollIntoView({ behavior: "smooth" });
    else if (tab === "logs" && logRef.current)
      logRef.current.scrollTop = logRef.current.scrollHeight;
  }, [messages, allTerminalLines, tab]);

  // Listen for switch-to-terminal event
  useEffect(() => {
    const handler = () => setTab("terminal");
    window.addEventListener("switch-to-terminal", handler);
    return () => window.removeEventListener("switch-to-terminal", handler);
  }, []);

  return (
    <div className="flex flex-1 flex-col min-h-0">
      {/* Tabs */}
      <div className="flex items-center px-4 py-2 border-b border-border/50 gap-3 relative">
        <TabBtn active={tab === "messages"} onClick={() => setTab("messages")}>
          Messages ({chatMessages.length})
        </TabBtn>
        <TabBtn active={tab === "terminal"} onClick={() => setTab("terminal")}>
          Terminal
        </TabBtn>
        <TabBtn active={tab === "logs"} onClick={() => setTab("logs")}>
          Logs {errorLines.length > 0 && `(${errorLines.length})`}
        </TabBtn>
        <div className="flex-1" />
        <Button variant="secondary" size="xs" onClick={clearMessages}>
          Clear
        </Button>
        <div className="absolute bottom-0 left-0 right-0 h-px bg-linear-to-r from-transparent via-primary/15 to-transparent" />
      </div>

      {/* Messages */}
      {tab === "messages" && (
        <div className="flex-1 overflow-y-auto px-4 py-2">
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
          {codexPhase !== "idle" && (
            <div className="flex items-center gap-2 py-2 text-[12px] text-muted-foreground msg-enter">
              <span className="relative inline-flex size-1.5">
                <span className="absolute inset-0 rounded-full bg-codex animate-ping opacity-40" />
                <span className="relative inline-block size-1.5 rounded-full bg-codex shadow-[0_0_6px_#22c55e]" />
              </span>
              {codexPhase === "thinking"
                ? "Codex is thinking\u2026"
                : "Codex is responding\u2026"}
            </div>
          )}
          <div ref={bottomRef} />
        </div>
      )}

      {/* Terminal (xterm.js - real PTY) */}
      <TerminalView visible={tab === "terminal"} />

      {/* Logs (errors only) */}
      {tab === "logs" && (
        <div
          ref={logRef}
          className="flex-1 overflow-y-auto px-4 py-2 font-mono text-[11px] leading-relaxed"
        >
          {errorLines.length === 0 && (
            <div className="py-10 text-center text-[13px] text-muted-foreground font-sans">
              No errors.
            </div>
          )}
          {errorLines.map((l, i) => (
            <div key={i} className="py-0.5 text-destructive">
              <span className="text-destructive/50 mr-2">
                {new Date(l.timestamp).toLocaleTimeString()}
              </span>
              <span className="mr-1">[{l.agent}]</span>
              {l.line}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
