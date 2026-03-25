#!/usr/bin/env bun

import { spawn } from "node:child_process";
import { appendFileSync, readFileSync, unlinkSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { AgentMcpAdapter } from "./adapters/claude-adapter";
import { DaemonClient } from "./daemon-client";
import type { BridgeMessage } from "./types";

const AGENT_ID = process.env.AGENTBRIDGE_AGENT ?? "claude";
const CONTROL_PORT = parseInt(
  process.env.AGENTBRIDGE_CONTROL_PORT ?? "4502",
  10,
);
const PID_FILE =
  process.env.AGENTBRIDGE_PID_FILE ??
  `/tmp/agentbridge-daemon-${CONTROL_PORT}.pid`;
const CONTROL_HEALTH_URL = `http://127.0.0.1:${CONTROL_PORT}/healthz`;
const CONTROL_WS_URL = `ws://127.0.0.1:${CONTROL_PORT}/ws`;
const LOG_FILE = "/tmp/agentbridge.log";
const DAEMON_PATH = fileURLToPath(new URL("./daemon.ts", import.meta.url));

const adapter = new AgentMcpAdapter();
const daemonClient = new DaemonClient(CONTROL_WS_URL);

let shuttingDown = false;

// Local buffer for messages pushed via WebSocket while agent is attached.
// check_messages drains this buffer first, then falls back to daemon fetch.
const pushedMessages: BridgeMessage[] = [];
const MAX_PUSHED = 100;

// Wire up tools → daemon
adapter.setReplySender(async (msg) => daemonClient.sendReply(msg));
adapter.setMessageFetcher(async () => {
  if (pushedMessages.length > 0) {
    const drained = pushedMessages.splice(0, pushedMessages.length);
    log(`Returning ${drained.length} pushed message(s) to ${AGENT_ID}`);
    return drained;
  }
  return daemonClient.fetchMessages();
});
adapter.setStatusFetcher(async () => {
  const res = await fetch(CONTROL_HEALTH_URL).catch(() => null);
  if (!res?.ok)
    return { bridgeReady: false, codexTuiRunning: false, threadId: null };
  return res.json();
});

// Buffer pushed messages locally so check_messages can return them
daemonClient.on("routedMessage", (message) => {
  pushedMessages.push(message);
  if (pushedMessages.length > MAX_PUSHED) {
    pushedMessages.splice(0, pushedMessages.length - MAX_PUSHED);
  }
  log(
    `Buffered routed message for ${AGENT_ID} (${message.content.length} chars, queue: ${pushedMessages.length})`,
  );
});

daemonClient.on("disconnect", () => {
  if (!shuttingDown) log("Daemon control connection closed");
});

adapter.on("ready", async () => {
  log(`MCP server ready for ${AGENT_ID} - ensuring AgentBridge daemon...`);
  try {
    await ensureDaemonRunning();
    await daemonClient.connect();
    daemonClient.attachAgent(AGENT_ID);
    log(`Connected to daemon as ${AGENT_ID}, tools ready`);
  } catch (err: any) {
    log(`Failed to connect to daemon: ${err.message}`);
  }
});

// ── Daemon lifecycle ───────────────────────────────────────

async function ensureDaemonRunning() {
  if (await isDaemonHealthy()) return;

  const existingPid = readDaemonPid();
  if (existingPid) {
    if (isProcessAlive(existingPid)) {
      try {
        await waitForDaemonHealthy(12, 250);
        return;
      } catch {
        throw new Error(
          `Daemon process ${existingPid} exists but port ${CONTROL_PORT} not healthy.`,
        );
      }
    }
    removeStalePidFile();
  }

  launchDaemon();
  await waitForDaemonHealthy();
}

function launchDaemon() {
  log(`Launching detached daemon on control port ${CONTROL_PORT}`);
  const daemonProc = spawn(process.execPath, ["run", DAEMON_PATH], {
    cwd: process.cwd(),
    env: { ...process.env },
    detached: true,
    stdio: "ignore",
  });
  daemonProc.unref();
}

async function isDaemonHealthy() {
  try {
    return (await fetch(CONTROL_HEALTH_URL)).ok;
  } catch {
    return false;
  }
}

async function waitForDaemonHealthy(maxRetries = 40, delayMs = 250) {
  for (let i = 0; i < maxRetries; i++) {
    if (await isDaemonHealthy()) return;
    await new Promise((r) => setTimeout(r, delayMs));
  }
  throw new Error(
    `Timed out waiting for daemon health on ${CONTROL_HEALTH_URL}`,
  );
}

function readDaemonPid() {
  try {
    const raw = readFileSync(PID_FILE, "utf-8").trim();
    const pid = Number.parseInt(raw, 10);
    return Number.isFinite(pid) ? pid : null;
  } catch {
    return null;
  }
}

function isProcessAlive(pid: number) {
  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function removeStalePidFile() {
  try {
    unlinkSync(PID_FILE);
  } catch {}
}

// ── Shutdown ───────────────────────────────────────────────

function shutdown(reason: string) {
  if (shuttingDown) return;
  shuttingDown = true;
  log(`Shutting down bridge/${AGENT_ID} (${reason})...`);
  const hardExit = setTimeout(() => process.exit(0), 3000);
  void daemonClient.disconnect().finally(() => {
    clearTimeout(hardExit);
    process.exit(0);
  });
}

process.on("SIGINT", () => shutdown("SIGINT"));
process.on("SIGTERM", () => shutdown("SIGTERM"));
process.stdin.on("end", () => shutdown("stdin closed"));
process.stdin.on("close", () => shutdown("stdin closed"));
process.on("exit", () => {
  if (!shuttingDown) void daemonClient.disconnect();
});
process.on("uncaughtException", (err) =>
  log(`UNCAUGHT: ${err.stack ?? err.message}`),
);
process.on("unhandledRejection", (reason: any) =>
  log(`UNHANDLED: ${reason?.stack ?? reason}`),
);

function log(msg: string) {
  const line = `[${new Date().toISOString()}] [Bridge/${AGENT_ID}] ${msg}\n`;
  process.stderr.write(line);
  try {
    appendFileSync(LOG_FILE, line);
  } catch {}
}

log(`Starting bridge for ${AGENT_ID} (daemon ws ${CONTROL_WS_URL})`);
void adapter.start().catch((err: any) => log(`Fatal: ${err.message}`));
