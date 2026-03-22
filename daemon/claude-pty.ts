import * as pty from "node-pty";
import { appendFileSync } from "node:fs";

const LOG_FILE = "/tmp/agentbridge.log";

export type PtyDataCallback = (data: string) => void;

/**
 * Spawns `claude` in a real PTY — identical to macOS Terminal.
 * Raw output (including ANSI escape codes) is forwarded via callback.
 * User input can be written via write().
 */
export class ClaudePty {
  private term: pty.IPty | null = null;
  private onData: PtyDataCallback;
  private onExitCb: ((code: number) => void) | null = null;

  constructor(onData: PtyDataCallback) {
    this.onData = onData;
  }

  get running() {
    return this.term !== null;
  }

  start(cwd?: string, cols = 120, rows = 30) {
    if (this.running) return;

    const dir = cwd || process.cwd();
    this.log(`Starting Claude PTY in ${dir} (${cols}x${rows})`);

    this.term = pty.spawn("claude", ["--dangerously-skip-permissions"], {
      name: "xterm-256color",
      cols,
      rows,
      cwd: dir,
      env: { ...process.env, TERM: "xterm-256color" },
    });

    this.term.onData((data) => {
      this.onData(data);
    });

    this.term.onExit(({ exitCode }) => {
      this.log(`Claude PTY exited (code ${exitCode})`);
      this.term = null;
      this.onExitCb?.(exitCode);
    });
  }

  write(data: string) {
    if (!this.term) return;
    this.term.write(data);
  }

  resize(cols: number, rows: number) {
    if (!this.term) return;
    this.term.resize(cols, rows);
  }

  stop() {
    if (!this.term) return;
    this.log("Stopping Claude PTY");
    this.term.kill();
    this.term = null;
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
