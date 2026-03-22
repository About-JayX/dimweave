import { spawn, type ChildProcess } from "node:child_process";
import { createInterface } from "node:readline";
import { appendFileSync } from "node:fs";

const LOG_FILE = "/tmp/agentbridge.log";

export interface ClaudeTerminalEvent {
  kind: "text" | "tool_use" | "tool_result" | "status" | "error" | "cost";
  content: string;
}

export type TerminalEventCallback = (event: ClaudeTerminalEvent) => void;

/**
 * Manages a headless Claude Code process in --print stream-json mode.
 * Parses JSON output into readable terminal events.
 */
export class ClaudeProcess {
  private proc: ChildProcess | null = null;
  private onEvent: TerminalEventCallback;
  private onExit: ((code: number | null) => void) | null = null;

  constructor(onEvent: TerminalEventCallback) {
    this.onEvent = onEvent;
  }

  get running() {
    return this.proc !== null && this.proc.exitCode === null;
  }

  start(cwd?: string) {
    if (this.running) return;

    const dir = cwd ?? ".";
    this.log(`Starting Claude in ${dir}`);

    const bridgePath = new URL("./bridge.ts", import.meta.url).pathname;
    const mcpConfig = JSON.stringify({
      mcpServers: {
        agentbridge: { command: "bun", args: ["run", bridgePath] },
      },
    });

    this.proc = spawn(
      "claude",
      [
        "--print",
        "--output-format",
        "stream-json",
        "--input-format",
        "stream-json",
        "--verbose",
        "--dangerously-skip-permissions",
        "--mcp-config",
        mcpConfig,
      ],
      {
        cwd: dir,
        stdio: ["pipe", "pipe", "pipe"],
        env: { ...process.env, NO_COLOR: "1" },
      },
    );

    // Send initial prompt via stdin (stream-json mode ignores -p flag)
    this.proc.stdin?.write(
      JSON.stringify({
        type: "user",
        message: {
          role: "user",
          content:
            "You are connected to AgentBridge. Use the agentbridge MCP tools (reply, check_messages, get_status) to communicate with Codex. Call get_status first, then check_messages.",
        },
      }) + "\n",
    );

    if (this.proc.stdout) {
      const rl = createInterface({ input: this.proc.stdout });
      rl.on("line", (raw) => this.parseStreamJson(raw));
    }

    if (this.proc.stderr) {
      const rl = createInterface({ input: this.proc.stderr });
      rl.on("line", (raw) => {
        const clean = raw.trim();
        if (clean) this.onEvent({ kind: "error", content: clean });
      });
    }

    this.proc.on("exit", (code) => {
      this.log(`Claude process exited (code ${code})`);
      this.proc = null;
      this.onEvent({ kind: "status", content: `Claude exited (code ${code})` });
      this.onExit?.(code);
    });

    this.proc.on("error", (err) => {
      this.log(`Claude process error: ${err.message}`);
      this.onEvent({ kind: "error", content: err.message });
    });

    this.onEvent({ kind: "status", content: "Claude starting..." });
  }

  sendInput(text: string) {
    if (!this.proc?.stdin?.writable) {
      this.log("Cannot send input: Claude not running");
      return;
    }
    const msg = JSON.stringify({
      type: "user",
      message: { role: "user", content: text },
    });
    this.proc.stdin.write(msg + "\n");
    this.log(`Sent input to Claude (${text.length} chars)`);
  }

  stop() {
    if (!this.proc) return;
    this.log("Stopping Claude process");
    this.proc.kill("SIGTERM");
    setTimeout(() => {
      try {
        this.proc?.kill("SIGKILL");
      } catch {}
    }, 3000);
  }

  setOnExit(cb: (code: number | null) => void) {
    this.onExit = cb;
  }

  private parseStreamJson(raw: string) {
    const trimmed = raw.trim();
    if (!trimmed) return;

    let data: any;
    try {
      data = JSON.parse(trimmed);
    } catch {
      // Not JSON, forward as plain text
      this.onEvent({ kind: "text", content: trimmed });
      return;
    }

    switch (data.type) {
      case "system":
        if (data.subtype === "init") {
          const tools = (data.mcp_servers ?? [])
            .map((s: any) => s.name)
            .join(", ");
          this.onEvent({
            kind: "status",
            content: `Session started (MCP: ${tools || "none"})`,
          });
        }
        break;

      case "assistant": {
        const content = data.message?.content;
        if (!Array.isArray(content)) break;
        for (const block of content) {
          if (block.type === "text" && block.text) {
            this.onEvent({ kind: "text", content: block.text });
          }
          if (block.type === "tool_use") {
            const args = block.input
              ? JSON.stringify(block.input).slice(0, 100)
              : "";
            this.onEvent({
              kind: "tool_use",
              content: `${block.name}(${args})`,
            });
          }
        }
        break;
      }

      case "user": {
        // Tool results
        const content = data.message?.content;
        if (!Array.isArray(content)) break;
        for (const block of content) {
          if (block.type === "tool_result" && block.content) {
            const text =
              typeof block.content === "string"
                ? block.content
                : Array.isArray(block.content)
                  ? block.content.map((c: any) => c.text ?? "").join("")
                  : JSON.stringify(block.content);
            if (text)
              this.onEvent({
                kind: "tool_result",
                content: text.slice(0, 200),
              });
          }
        }
        break;
      }

      case "result": {
        const cost = data.total_cost_usd;
        if (cost != null) {
          this.onEvent({ kind: "cost", content: `$${cost.toFixed(4)}` });
        }
        if (data.result) {
          this.onEvent({ kind: "text", content: data.result.slice(0, 300) });
        }
        break;
      }

      case "rate_limit_event":
        // Silently ignore
        break;

      default:
        // Unknown type, log for debugging
        this.log(`[stream] unknown type: ${data.type}`);
        break;
    }
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [ClaudeProcess] ${msg}\n`;
    process.stderr.write(line);
    try {
      appendFileSync(LOG_FILE, line);
    } catch {}
  }
}
