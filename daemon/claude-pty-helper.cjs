#!/usr/bin/env node
// Runs in Node.js (not Bun) because node-pty needs native Node addons.
// Communicates with parent via JSON lines on stdin/stdout.

const pty = require("node-pty");
const { createInterface } = require("node:readline");

const cols = parseInt(process.env.PTY_COLS || "120", 10);
const rows = parseInt(process.env.PTY_ROWS || "30", 10);
const claudePath = process.env.CLAUDE_PATH || "claude";
const agentRole = process.env.CLAUDE_AGENT_ROLE || "";
const agentsJson = process.env.CLAUDE_AGENTS_JSON || "";

const args = ["--dangerously-skip-permissions"];

// Use --agent + --agents for role-based hard enforcement
if (agentRole && agentsJson) {
  args.push("--agent", agentRole, "--agents", agentsJson);
}

const term = pty.spawn(claudePath, args, {
  name: "xterm-256color",
  cols,
  rows,
  cwd: process.cwd(),
  env: { ...process.env, TERM: "xterm-256color" },
});

term.onData((data) => {
  process.stdout.write(JSON.stringify({ type: "data", data }) + "\n");
});

term.onExit(({ exitCode }) => {
  process.stdout.write(JSON.stringify({ type: "exit", code: exitCode }) + "\n");
  setTimeout(() => process.exit(0), 100);
});

const rl = createInterface({ input: process.stdin });
rl.on("line", (line) => {
  try {
    const msg = JSON.parse(line);
    if (msg.type === "input") term.write(msg.data);
    else if (msg.type === "resize") term.resize(msg.cols, msg.rows);
    else if (msg.type === "kill") term.kill();
  } catch {}
});

process.stdin.on("end", () => { term.kill(); process.exit(0); });
