import { spawn, type ChildProcess } from "node:child_process";
import { appendFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const LOG_FILE = "/tmp/agentbridge.log";

export type PtyDataCallback = (data: string) => void;

/**
 * Spawns a Node.js subprocess that runs node-pty (Bun can't run node-pty natively).
 * The subprocess handles the real PTY and communicates via JSON messages on stdio.
 */
export class ClaudePty {
  private proc: ChildProcess | null = null;
  private onData: PtyDataCallback;
  private onExitCb: ((code: number) => void) | null = null;
  private exitFired = false;

  constructor(onData: PtyDataCallback) {
    this.onData = onData;
  }

  get running() {
    return this.proc !== null && this.proc.exitCode === null;
  }

  start(
    cwd?: string,
    cols = 120,
    rows = 30,
    agentConfig?: { roleId: string; agentsJson: string },
  ) {
    if (this.running) return;
    this.exitFired = false;

    const dir = cwd || process.cwd();
    this.log(`Starting Claude PTY in ${dir} (${cols}x${rows})`);

    // Spawn a Node.js process that runs the PTY helper
    const helperPath = fileURLToPath(
      new URL("./claude-pty-helper.cjs", import.meta.url),
    );

    const nodePath = new URL("../node_modules", import.meta.url).pathname;
    this.log(
      `Helper: ${helperPath}, NODE_PATH: ${nodePath}, CLAUDE: ${process.env.CLAUDE_PATH || "claude"}`,
    );

    this.proc = spawn("node", [helperPath], {
      cwd: dir,
      stdio: ["pipe", "pipe", "pipe"],
      env: {
        ...process.env,
        PTY_COLS: String(cols),
        PTY_ROWS: String(rows),
        NODE_PATH: nodePath,
        CLAUDE_PATH: process.env.CLAUDE_PATH || "claude",
        CLAUDE_AGENT_ROLE: agentConfig?.roleId || "",
        CLAUDE_AGENTS_JSON: agentConfig?.agentsJson || "",
      },
    });

    this.proc.on("error", (err) => {
      this.log(`Helper spawn error: ${err.message}`);
    });

    // Read JSON messages from helper stdout
    let buffer = "";
    this.proc.stdout?.on("data", (chunk: Buffer) => {
      buffer += chunk.toString();
      let nl: number;
      while ((nl = buffer.indexOf("\n")) >= 0) {
        const line = buffer.slice(0, nl);
        buffer = buffer.slice(nl + 1);
        try {
          const msg = JSON.parse(line);
          if (msg.type === "data") this.onData(msg.data);
          else if (msg.type === "exit") {
            this.log(`PTY helper reported exit (code ${msg.code})`);
            if (!this.exitFired) {
              this.exitFired = true;
              this.onExitCb?.(msg.code);
            }
          }
        } catch {}
      }
    });

    this.proc.stderr?.on("data", (chunk: Buffer) => {
      this.log(`[pty-helper stderr] ${chunk.toString().trim()}`);
    });

    this.proc.on("exit", (code) => {
      this.log(`PTY helper process exited (code ${code})`);
      this.proc = null;
      if (!this.exitFired) {
        this.exitFired = true;
        this.onExitCb?.(code ?? 1);
      }
    });
  }

  write(data: string) {
    if (!this.proc?.stdin?.writable) return;
    this.proc.stdin.write(JSON.stringify({ type: "input", data }) + "\n");
  }

  resize(cols: number, rows: number) {
    if (!this.proc?.stdin?.writable) return;
    this.proc.stdin.write(
      JSON.stringify({ type: "resize", cols, rows }) + "\n",
    );
  }

  stop() {
    if (!this.proc) return;
    this.log("Stopping Claude PTY");
    this.proc.stdin?.write(JSON.stringify({ type: "kill" }) + "\n");
    setTimeout(() => {
      try {
        this.proc?.kill("SIGKILL");
      } catch {}
    }, 3000);
  }

  setOnExit(cb: (code: number) => void) {
    this.onExitCb = cb;
  }

  private log(msg: string) {
    const line = `[${new Date().toISOString()}] [ClaudePty] ${msg}\n`;
    process.stderr.write(line);
    try {
      appendFileSync(LOG_FILE, line);
    } catch {}
  }
}
