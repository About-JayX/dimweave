import { useEffect, useLayoutEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import type { ClaudeTerminalChunk } from "@/stores/bridge-store/types";
import { createClaudeTerminalOptions } from "./claude-terminal-config";

interface ClaudeTerminalPaneProps {
  chunks: ClaudeTerminalChunk[];
  connected: boolean;
  running: boolean;
  detail?: string;
}

export function ClaudeTerminalPane({
  chunks,
  connected,
  running,
  detail,
}: ClaudeTerminalPaneProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const renderedCountRef = useRef(0);

  useLayoutEffect(() => {
    const terminal = new Terminal(createClaudeTerminalOptions());
    const fitAddon = new FitAddon();
    const unicodeAddon = new Unicode11Addon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(unicodeAddon);
    terminal.unicode.activeVersion = "11";
    termRef.current = terminal;
    fitRef.current = fitAddon;

    const host = hostRef.current;
    if (host) {
      terminal.open(host);
    }
    fitTerminal(terminal, fitAddon);

    const disposable = terminal.onData((data) => {
      void invoke("claude_terminal_input", { data }).catch(() => {});
    });

    const observer = new ResizeObserver(() => {
      const term = termRef.current;
      const fit = fitRef.current;
      if (!term || !fit) return;
      fitTerminal(term, fit);
    });

    if (host) {
      observer.observe(host);
    }

    return () => {
      observer.disconnect();
      disposable.dispose();
      terminal.dispose();
      termRef.current = null;
      fitRef.current = null;
      renderedCountRef.current = 0;
    };
  }, []);

  useLayoutEffect(() => {
    const terminal = termRef.current;
    if (!terminal) return;

    if (chunks.length < renderedCountRef.current) {
      terminal.reset();
      renderedCountRef.current = 0;
    }

    for (const chunk of chunks.slice(renderedCountRef.current)) {
      terminal.write(chunk.data);
    }
    renderedCountRef.current = chunks.length;
  }, [chunks]);

  useEffect(() => {
    const terminal = termRef.current;
    const fit = fitRef.current;
    if (!terminal || !fit) return;

    const raf = window.requestAnimationFrame(() => {
      fitTerminal(terminal, fit);
      if (connected) {
        terminal.focus();
      }
    });

    return () => {
      window.cancelAnimationFrame(raf);
    };
  }, [connected]);

  return (
    <div className="flex flex-1 flex-col min-h-0 bg-[#090b10]">
      {!connected && chunks.length === 0 && (
        <div className="flex flex-1 items-center justify-center px-6 text-center text-[13px] text-muted-foreground">
          Claude terminal is idle. Connect Claude to start an embedded session.
        </div>
      )}
      {!running && chunks.length > 0 && detail && (
        <div className="border-b border-border/40 bg-card/60 px-3 py-2 text-[11px] text-muted-foreground">
          {detail}
        </div>
      )}
      <div
        ref={hostRef}
        className="min-h-0 flex-1 overflow-hidden px-2 py-2 [&_.xterm]:h-full"
      />
    </div>
  );
}

function fitTerminal(terminal: Terminal, fitAddon: FitAddon) {
  fitAddon.fit();
  void invoke("claude_terminal_resize", {
    cols: terminal.cols,
    rows: terminal.rows,
  }).catch(() => {});
}
