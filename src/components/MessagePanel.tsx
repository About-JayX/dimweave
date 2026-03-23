import { useState, useEffect, useRef } from "react";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { MessageMarkdown } from "@/components/MessageMarkdown";
import { useBridgeStore, type TerminalLine } from "@/stores/bridge-store";
import type { BridgeMessage, MessageSource } from "@/types";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

const sourceStyle: Record<string, { label: string; className: string }> = {
  claude: {
    label: "Claude",
    className: "border-claude/40 bg-claude/10 text-claude",
  },
  codex: {
    label: "Codex",
    className: "border-codex/40 bg-codex/10 text-codex",
  },
  system: {
    label: "System",
    className: "border-system/40 bg-system/10 text-system",
  },
};

function SourceBadge({ source }: { source: MessageSource }) {
  const style = sourceStyle[source] ?? sourceStyle.system;
  return (
    <Badge variant="outline" className={cn("uppercase", style.className)}>
      {style.label}
    </Badge>
  );
}

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
  const xtermContainerRef = useRef<HTMLDivElement>(null);
  const xtermRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const ptyBufferRef = useRef<string[]>([]);

  const clearMessages = useBridgeStore((s) => s.clearMessages);
  const codexPhase = useBridgeStore((s) => s.codexPhase);
  const allTerminalLines = useBridgeStore((s) => s.terminalLines);

  const chatMessages = messages.filter((m) => m.source !== "system");

  const errorLines: TerminalLine[] = [];
  for (const l of allTerminalLines) {
    if (l.kind === "error") errorLines.push(l);
  }

  // Buffer PTY data immediately (before xterm is ready)
  useEffect(() => {
    const unlisten = listen<string>("pty-data", (event) => {
      if (xtermRef.current) {
        xtermRef.current.write(event.payload);
      } else {
        ptyBufferRef.current.push(event.payload);
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Initialize xterm.js when terminal tab becomes visible
  useEffect(() => {
    if (tab !== "terminal" || !xtermContainerRef.current || xtermRef.current) {
      if (tab === "terminal" && xtermRef.current) {
        setTimeout(() => fitAddonRef.current?.fit(), 50);
      }
      return;
    }

    const term = new Terminal({
      theme: {
        background: "#0a0a0a",
        foreground: "#e5e5e5",
        cursor: "#e5e5e5",
        selectionBackground: "#8b5cf644",
      },
      fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
      fontSize: 13,
      cursorBlink: true,
      scrollback: 5000,
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(xtermContainerRef.current);
    setTimeout(() => fitAddon.fit(), 100);

    // Keystrokes → Rust PTY (direct invoke, no WS)
    term.onData((data) => {
      invoke("pty_write", { data }).catch(() => {});
    });
    term.onResize(({ cols, rows }) => {
      invoke("pty_resize", { cols, rows }).catch(() => {});
    });

    xtermRef.current = term;
    fitAddonRef.current = fitAddon;

    // Flush buffered PTY data
    for (const chunk of ptyBufferRef.current) {
      term.write(chunk);
    }
    ptyBufferRef.current = [];
  }, [tab]);

  // Handle resize (debounced)
  useEffect(() => {
    if (!fitAddonRef.current || !xtermContainerRef.current) return;
    let timer: ReturnType<typeof setTimeout>;
    const observer = new ResizeObserver(() => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        if (tab === "terminal") fitAddonRef.current?.fit();
      }, 100);
    });
    observer.observe(xtermContainerRef.current);
    return () => {
      clearTimeout(timer);
      observer.disconnect();
    };
  }, [tab]);

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
      <div className="flex items-center px-4 py-2 border-b border-border gap-3">
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
      </div>

      {/* Messages */}
      {tab === "messages" && (
        <div className="flex-1 overflow-y-auto px-4 py-2">
          {chatMessages.length === 0 && (
            <div className="py-10 text-center text-[13px] text-muted-foreground">
              No messages yet. Connect Claude and Codex to start bridging.
            </div>
          )}
          {chatMessages.map((msg) => (
            <div key={msg.id} className="py-2.5 border-b border-card">
              <div className="flex items-center gap-2 mb-1">
                <SourceBadge source={msg.source} />
                <span className="font-mono text-[11px] text-muted-foreground">
                  {new Date(msg.timestamp).toLocaleTimeString()}
                </span>
              </div>
              <MessageMarkdown content={msg.content} />
            </div>
          ))}
          {codexPhase !== "idle" && (
            <div className="flex items-center gap-2 py-2 text-[12px] text-muted-foreground">
              <span className="inline-block size-1.5 rounded-full bg-codex animate-pulse" />
              {codexPhase === "thinking"
                ? "Codex is thinking…"
                : "Codex is responding…"}
            </div>
          )}
          <div ref={bottomRef} />
        </div>
      )}

      {/* Terminal (xterm.js — real PTY) */}
      <div
        ref={xtermContainerRef}
        className={cn("flex-1 min-h-0", tab !== "terminal" && "hidden")}
      />

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

function TabBtn({
  active,
  onClick,
  children,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "text-sm font-semibold transition-colors",
        active
          ? "text-foreground"
          : "text-muted-foreground hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}
