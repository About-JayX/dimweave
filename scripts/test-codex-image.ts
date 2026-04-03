/**
 * Test script: send a localImage input to Codex app-server and capture the stream.
 *
 * Usage:
 *   1. Start codex app-server: codex app-server --listen ws://127.0.0.1:4500
 *   2. Run: bun run scripts/test-codex-image.ts /path/to/image.png
 *
 * This bypasses the daemon and talks directly to Codex WS to observe
 * how it handles localImage inputs.
 */

const imagePath = process.argv[2];
if (!imagePath) {
  console.error("Usage: bun run scripts/test-codex-image.ts <image-path>");
  process.exit(1);
}

const WS_URL = "ws://127.0.0.1:4500";
let rpcId = 0;

function send(ws: WebSocket, method: string, params: Record<string, unknown>) {
  const msg = JSON.stringify({ method, id: ++rpcId, params });
  console.log(`→ [${rpcId}] ${method}`, JSON.stringify(params, null, 2));
  ws.send(msg);
  return rpcId;
}

const ws = new WebSocket(WS_URL);

ws.onopen = () => {
  console.log("=== Connected to Codex app-server ===\n");

  // Step 1: initialize
  send(ws, "initialize", {
    clientInfo: { name: "dimweave-test", version: "0.0.1" },
    capabilities: { experimentalApi: true },
  });
};

let threadId: string | null = null;
let initDone = false;

ws.onmessage = (event) => {
  const data = JSON.parse(String(event.data));

  // Pretty print
  const method = data.method ?? `response:${data.id}`;
  console.log(`← [${method}]`, JSON.stringify(data, null, 2));

  // After initialize response, start a thread
  if (data.id === 1 && !initDone) {
    initDone = true;
    send(ws, "thread/start", {
      cwd: process.cwd(),
    });
    return;
  }

  // After thread/start response, send a turn with localImage
  if (data.id === 2 && data.result?.threadId) {
    threadId = data.result.threadId;
    console.log(`\n=== Thread started: ${threadId} ===`);
    console.log(`=== Sending localImage: ${imagePath} ===\n`);

    send(ws, "turn/start", {
      threadId,
      input: [
        { type: "text", text: "Describe this image in detail." },
        { type: "localImage", path: imagePath },
      ],
    });
    return;
  }

  // Log all streaming events
  if (data.method === "turn/completed") {
    console.log("\n=== Turn completed ===");
    console.log("Status:", data.params?.turn?.status);
    setTimeout(() => {
      console.log("\nDone. Closing.");
      ws.close();
      process.exit(0);
    }, 500);
  }
};

ws.onerror = (e) => {
  console.error("WS error:", e);
  process.exit(1);
};

ws.onclose = () => {
  console.log("WS closed");
};

// Timeout after 60s
setTimeout(() => {
  console.error("Timeout after 60s");
  ws.close();
  process.exit(1);
}, 60000);
