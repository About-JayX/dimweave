import { spawn, type ChildProcess } from "node:child_process";
import { createInterface } from "node:readline";
import { appendFileSync } from "node:fs";

const LOG_FILE = "/tmp/agentbridge.log";
const ANSI_RE = /\x1B(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~]|\].*?(?:\x07|\x1B\\))/g;

export type TerminalLineCallback = (line: string) => void;

/**
 * Manages a headless Claude Code process with PTY wrapping.
 * Terminal output is forwarded line-by-line to a callback.
 * User input can be sent via sendInput().
 */
export class ClaudeProcess {
  private proc: ChildProcess | null = null;
  private onLine: TerminalLineCallback;
  private onExit: ((code: number | null) => void) | null = null;

  constructor(onLine: TerminalLineCallback) {
    this.onLine = onLine;
  }

  get running() {
    return this.proc !== null && this.proc.exitCode === null;
  }

  start(cwd?: string) {
    if (this.running) return;

    const dir = cwd ?? process.cwd();
    this.log(`Starting Claude in ${dir}`);

    // Build MCP config inline so --print mode loads the bridge
    const bridgePath = new URL("./bridge.ts", import.meta.url).pathname;
    const mcpConfig = JSON.stringify({
      mcpServers: {
        agentbridge: {
          command: "bun",
          args: ["run", bridgePath],
        },
      },
    });

    this.proc = spawn(
      "claude",
      [
        "-p",
        "You are connected to AgentBridge. Use the agentbridge MCP tools (reply, check_messages, get_status) to communicate with Codex. Call get_status first, then check_messages to see if there are pending messages.",
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

    // Read stdout line by line, strip ANSI, forward
    if (this.proc.stdout) {
      const rl = createInterface({ input: this.proc.stdout });
      rl.on("line", (raw) => {
        const clean = stripAnsi(raw).trim();
        if (clean) this.onLine(clean);
      });
    }

    if (this.proc.stderr) {
      const rl = createInterface({ input: this.proc.stderr });
      rl.on("line", (raw) => {
        const clean = stripAnsi(raw).trim();
        if (clean) this.onLine(clean);
      });
    }

    this.proc.on("exit", (code) => {
      this.log(`Claude process exited (code ${code})`);
      this.proc = null;
      this.onExit?.(code);
    });

    this.proc.on("error", (err) => {
      this.log(`Claude process error: ${err.message}`);
    });
  }

  sendInput(text: string) {
    if (!this.proc?.stdin?.writable) {
      this.log("Cannot send input: Claude not running");
      return;
    }
    // stream-json input format: one JSON object per line
    const msg = JSON.stringify({ type: "user_message", content: text });
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

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [ClaudeProcess] ${msg}\n`;
    process.stderr.write(line);
    try {
      appendFileSync(LOG_FILE, line);
    } catch {}
  }
}

function stripAnsi(str: string): string {
  return str
    .replace(ANSI_RE, "")
    .replace(/[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]/g, "");
}
