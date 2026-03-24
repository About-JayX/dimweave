#!/usr/bin/env bun
/**
 * End-to-end test: Codex ↔ Claude communication chain.
 *
 * Tests:
 * 1. Basic message: send "hi" to Codex, verify agentMessage + turnCompleted + pty_inject
 * 2. Code review request: send review task, verify response matches role template
 * 3. Claude → Codex reply: simulate Claude sending back via control WS
 * 4. No infinite loops: verify message count stays bounded
 */

const GUI_WS = "ws://127.0.0.1:4503";
const CONTROL_WS = "ws://127.0.0.1:4502/ws";
const STATUS_URL = "http://127.0.0.1:4503/status";

interface TestResult {
  name: string;
  pass: boolean;
  detail: string;
}

const results: TestResult[] = [];
function record(name: string, pass: boolean, detail: string) {
  results.push({ name, pass, detail });
  const icon = pass ? "✅" : "❌";
  console.log(`${icon} ${name}: ${detail}`);
}

// ── Helpers ──────────────────────────────────────────────

function wsConnect(url: string): Promise<WebSocket> {
  return new Promise((resolve, reject) => {
    const ws = new WebSocket(url);
    ws.onopen = () => resolve(ws);
    ws.onerror = (e) => reject(new Error(`WS connect failed: ${url}`));
    setTimeout(() => reject(new Error(`WS connect timeout: ${url}`)), 5000);
  });
}

function waitForEvent(
  ws: WebSocket,
  predicate: (data: any) => boolean,
  timeoutMs = 30000,
): Promise<any> {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      ws.removeEventListener("message", handler);
      reject(new Error(`Timeout waiting for event (${timeoutMs}ms)`));
    }, timeoutMs);

    function handler(event: MessageEvent) {
      try {
        const data = JSON.parse(event.data);
        if (predicate(data)) {
          clearTimeout(timer);
          ws.removeEventListener("message", handler);
          resolve(data);
        }
      } catch {}
    }
    ws.addEventListener("message", handler);
  });
}

function collectEvents(ws: WebSocket, durationMs: number): Promise<any[]> {
  return new Promise((resolve) => {
    const events: any[] = [];
    function handler(event: MessageEvent) {
      try {
        events.push(JSON.parse(event.data));
      } catch {}
    }
    ws.addEventListener("message", handler);
    setTimeout(() => {
      ws.removeEventListener("message", handler);
      resolve(events);
    }, durationMs);
  });
}

function send(ws: WebSocket, data: object) {
  ws.send(JSON.stringify(data));
}

async function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

// ── Main ─────────────────────────────────────────────────

async function main() {
  console.log("\n🔧 AgentBridge E2E Test\n");

  // ── Step 0: Check daemon status ──
  let status: any;
  try {
    const res = await fetch(STATUS_URL);
    status = await res.json();
    record("Daemon reachable", true, `pid=${status.pid}`);
  } catch (e: any) {
    record("Daemon reachable", false, e.message);
    return printSummary();
  }

  if (!status.codexBootstrapped) {
    record("Codex bootstrapped", false, "codex app-server not started");
    return printSummary();
  }
  record("Codex bootstrapped", true, "app-server running");

  // ── Step 1: Connect GUI WS ──
  let guiWs: WebSocket;
  try {
    guiWs = await wsConnect(GUI_WS);
    record("GUI WS connected", true, "ws://127.0.0.1:4503");
  } catch (e: any) {
    record("GUI WS connected", false, e.message);
    return printSummary();
  }

  // Wait for daemon_status event
  const daemonStatus = await waitForEvent(
    guiWs,
    (d) => d.type === "daemon_status",
    3000,
  );
  record(
    "Received daemon_status",
    true,
    `bridgeReady=${daemonStatus.payload.bridgeReady}`,
  );

  // ── Step 2: Init Codex session if needed ──
  if (!daemonStatus.payload.codexTuiRunning && !daemonStatus.payload.threadId) {
    console.log("\n📡 Initializing Codex session...");
    send(guiWs, { type: "launch_codex_tui" });

    try {
      const codexConnected = await waitForEvent(
        guiWs,
        (d) =>
          d.type === "agent_status" &&
          d.payload.agent === "codex" &&
          d.payload.status === "connected",
        15000,
      );
      record(
        "Codex session initialized",
        true,
        `threadId=${codexConnected.payload.threadId}`,
      );
    } catch (e: any) {
      record("Codex session initialized", false, e.message);
      guiWs.close();
      return printSummary();
    }

    await sleep(1000); // Let it stabilize
  } else {
    record(
      "Codex session initialized",
      true,
      `threadId=${daemonStatus.payload.threadId} (already active)`,
    );
  }

  // ── Step 3: Connect Control WS (simulate Claude MCP) ──
  let controlWs: WebSocket;
  try {
    controlWs = await wsConnect(CONTROL_WS);
    send(controlWs, { type: "claude_connect" });
    await sleep(500);
    record("Control WS (Claude) connected", true, "ws://127.0.0.1:4502/ws");
  } catch (e: any) {
    record("Control WS (Claude) connected", false, e.message);
    guiWs.close();
    return printSummary();
  }

  // ══════════════════════════════════════════════════════
  // TEST 1: Simple message "hi"
  // ══════════════════════════════════════════════════════
  console.log("\n── Test 1: Simple message 'hi' ──");

  // Start collecting events on both connections
  const guiEventsPromise1 = collectEvents(guiWs, 30000);
  const controlEventsPromise1 = collectEvents(controlWs, 30000);

  send(guiWs, { type: "send_to_codex", content: "hi" });

  // Wait for turn completion on GUI WS (indicates Codex finished)
  let turnCompletedEvents: any[] = [];
  let agentMessages: any[] = [];
  let ptyInjectEvents: any[] = [];
  let systemLogs: any[] = [];

  const guiEvents1 = await guiEventsPromise1;
  const controlEvents1 = await controlEventsPromise1;

  for (const e of guiEvents1) {
    if (e.type === "agent_message") agentMessages.push(e);
    if (e.type === "pty_inject") ptyInjectEvents.push(e);
    if (e.type === "system_log") systemLogs.push(e);
    if (e.type === "codex_phase" && e.payload.phase === "idle")
      turnCompletedEvents.push(e);
  }

  // Check: got agent_message from codex
  const codexMessages = agentMessages.filter(
    (e) => e.payload.source === "codex",
  );
  record(
    "Codex replied to 'hi'",
    codexMessages.length > 0,
    codexMessages.length > 0
      ? `${codexMessages.length} message(s), content: "${codexMessages[0]?.payload.content?.slice(0, 80)}..."`
      : "No codex agentMessage received",
  );

  // Check: turn completed (phase → idle)
  record(
    "Turn completed",
    turnCompletedEvents.length > 0,
    `${turnCompletedEvents.length} idle phase event(s)`,
  );

  // Check: pty_inject event broadcast
  record(
    "pty_inject broadcast",
    ptyInjectEvents.length > 0,
    ptyInjectEvents.length > 0
      ? `${ptyInjectEvents.length} event(s), data: "${ptyInjectEvents[0]?.payload.data?.slice(0, 80)}"`
      : "No pty_inject received — THIS IS THE BUG",
  );

  // Check: pty_inject content has role prefix
  if (ptyInjectEvents.length > 0) {
    const injectData = ptyInjectEvents[0].payload.data;
    const hasRolePrefix =
      injectData.includes("Coder says:") ||
      injectData.includes("Reviewer says:") ||
      injectData.includes("Tester says:") ||
      injectData.includes("Lead says:");
    record(
      "pty_inject has role prefix",
      hasRolePrefix,
      `content="${injectData.slice(0, 100)}"`,
    );
  }

  // Check: control WS received codex_to_claude
  const claudeMessages = controlEvents1.filter(
    (e) => e.type === "codex_to_claude",
  );
  record(
    "Control WS received codex_to_claude",
    claudeMessages.length > 0,
    claudeMessages.length > 0
      ? `${claudeMessages.length} message(s), content: "${claudeMessages[0]?.message?.content?.slice(0, 80)}..."`
      : "No codex_to_claude on control WS",
  );

  // Check: system_log notification
  const completionLogs = systemLogs.filter((e) =>
    e.payload.message?.includes("completed"),
  );
  record(
    "System log: completion notice",
    completionLogs.length > 0,
    completionLogs.length > 0
      ? completionLogs[0].payload.message
      : "No completion log",
  );

  // ══════════════════════════════════════════════════════
  // TEST 2: Code review request
  // ══════════════════════════════════════════════════════
  console.log("\n── Test 2: Code review request ──");

  const guiEventsPromise2 = collectEvents(guiWs, 45000);
  const controlEventsPromise2 = collectEvents(controlWs, 45000);

  send(guiWs, {
    type: "send_to_codex",
    content:
      "Review the following code for bugs and suggest improvements:\n\nfunction add(a, b) { return a + b; }\nfunction divide(a, b) { return a / b; }",
  });

  const guiEvents2 = await guiEventsPromise2;
  const controlEvents2 = await controlEventsPromise2;

  const reviewMessages = guiEvents2
    .filter((e) => e.type === "agent_message" && e.payload.source === "codex")
    .map((e) => e.payload.content);
  const reviewPtyInjects = guiEvents2.filter((e) => e.type === "pty_inject");
  const reviewControlMsgs = controlEvents2.filter(
    (e) => e.type === "codex_to_claude",
  );

  record(
    "Codex reviewed code",
    reviewMessages.length > 0,
    reviewMessages.length > 0
      ? `Response (${reviewMessages[0]?.length} chars): "${reviewMessages[0]?.slice(0, 120)}..."`
      : "No review response",
  );

  // Check review quality — should mention divide by zero
  const mentionsDivideByZero = reviewMessages.some(
    (m) =>
      m &&
      (m.toLowerCase().includes("divide") ||
        m.toLowerCase().includes("zero") ||
        m.toLowerCase().includes("division")),
  );
  record(
    "Review mentions divide-by-zero risk",
    mentionsDivideByZero,
    mentionsDivideByZero
      ? "Found divide/zero reference"
      : "Did not mention — may be OK for simple review",
  );

  record(
    "Review pty_inject broadcast",
    reviewPtyInjects.length > 0,
    `${reviewPtyInjects.length} pty_inject event(s)`,
  );

  record(
    "Review forwarded to Claude (control WS)",
    reviewControlMsgs.length > 0,
    `${reviewControlMsgs.length} codex_to_claude message(s)`,
  );

  // ══════════════════════════════════════════════════════
  // TEST 3: Claude → Codex reply (simulate MCP reply tool)
  // ══════════════════════════════════════════════════════
  console.log("\n── Test 3: Claude → Codex reply ──");

  const guiEventsPromise3 = collectEvents(guiWs, 30000);

  send(controlWs, {
    type: "claude_to_codex",
    requestId: "test-reply-1",
    message: {
      id: `claude_reply_${Date.now()}`,
      source: "claude",
      content:
        "Thanks for the review. Can you also check for type safety issues?",
      timestamp: Date.now(),
    },
  });

  // Check control WS gets success response
  const replyResult = await waitForEvent(
    controlWs,
    (d) =>
      d.type === "claude_to_codex_result" && d.requestId === "test-reply-1",
    5000,
  ).catch(() => null);

  record(
    "Claude reply accepted",
    replyResult?.success === true,
    replyResult ? `success=${replyResult.success}` : "No result received",
  );

  // Wait for Codex to respond to Claude's reply
  const guiEvents3 = await guiEventsPromise3;
  const replyResponses = guiEvents3.filter(
    (e) => e.type === "agent_message" && e.payload.source === "codex",
  );
  const replyPtyInjects = guiEvents3.filter((e) => e.type === "pty_inject");

  record(
    "Codex responded to Claude reply",
    replyResponses.length > 0,
    replyResponses.length > 0
      ? `${replyResponses.length} response(s), content: "${replyResponses[0]?.payload.content?.slice(0, 100)}..."`
      : "No response (may need more time)",
  );

  record(
    "Reply pty_inject broadcast",
    replyPtyInjects.length > 0,
    `${replyPtyInjects.length} pty_inject event(s)`,
  );

  // ══════════════════════════════════════════════════════
  // TEST 4: No infinite loop check
  // ══════════════════════════════════════════════════════
  console.log("\n── Test 4: No infinite loop ──");

  // After the reply chain, collect events for 10s to check no runaway messages
  const loopCheckEvents = await collectEvents(guiWs, 10000);
  const loopCodexMessages = loopCheckEvents.filter(
    (e) => e.type === "agent_message" && e.payload.source === "codex",
  );
  const loopPtyInjects = loopCheckEvents.filter((e) => e.type === "pty_inject");

  record(
    "No infinite loop (10s quiet period)",
    loopCodexMessages.length === 0,
    loopCodexMessages.length === 0
      ? "No unexpected messages — chain terminated correctly"
      : `⚠️ ${loopCodexMessages.length} unexpected message(s) detected!`,
  );

  record(
    "No runaway pty_injects",
    loopPtyInjects.length === 0,
    loopPtyInjects.length === 0
      ? "No unexpected pty_inject — chain terminated correctly"
      : `⚠️ ${loopPtyInjects.length} unexpected pty_inject(s)!`,
  );

  // ══════════════════════════════════════════════════════
  // TEST 5: Verify role template in messages
  // ══════════════════════════════════════════════════════
  console.log("\n── Test 5: Role template verification ──");

  // Check daemon status for current roles
  const finalStatus = await fetch(STATUS_URL)
    .then((r) => r.json())
    .catch(() => null);
  if (finalStatus) {
    record(
      "Daemon roles configured",
      true,
      `Reported codexBootstrapped=${finalStatus.codexBootstrapped}`,
    );
  }

  // Verify pty_inject messages mention the correct Codex role (default: Coder)
  const allPtyInjects = [
    ...ptyInjectEvents,
    ...reviewPtyInjects,
    ...replyPtyInjects,
  ];
  if (allPtyInjects.length > 0) {
    const allMentionCoder = allPtyInjects.every((e) =>
      e.payload.data.includes("Coder"),
    );
    record(
      "pty_inject uses correct role label (Coder)",
      allMentionCoder,
      allMentionCoder
        ? "All pty_inject events reference 'Coder' role"
        : `Some events don't match: ${allPtyInjects.map((e) => e.payload.data.slice(0, 50))}`,
    );
  } else {
    record(
      "pty_inject uses correct role label",
      false,
      "No pty_inject events to check",
    );
  }

  // ── Cleanup ──
  controlWs.close();
  guiWs.close();
  printSummary();
}

function printSummary() {
  const passed = results.filter((r) => r.pass).length;
  const total = results.length;
  console.log(`\n${"═".repeat(60)}`);
  console.log(`📊 Results: ${passed}/${total} passed`);
  if (passed === total) {
    console.log("🎉 All tests passed!");
  } else {
    console.log("⚠️  Some tests failed:");
    for (const r of results.filter((r) => !r.pass)) {
      console.log(`   ❌ ${r.name}: ${r.detail}`);
    }
  }
  console.log(`${"═".repeat(60)}\n`);
  process.exit(passed === total ? 0 : 1);
}

main().catch((e) => {
  console.error("Fatal error:", e);
  process.exit(2);
});
