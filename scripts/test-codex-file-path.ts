/**
 * Test: send a plain text message containing a file path to Codex app-server.
 * Observe whether Codex automatically invokes tools to read/analyze the file.
 *
 * Usage:
 *   1. Ensure codex app-server is running on ws://127.0.0.1:4500
 *   2. bun run scripts/test-codex-file-path.ts /path/to/image.png
 */

const filePath = process.argv[2];
if (!filePath) {
  console.error("Usage: bun run scripts/test-codex-file-path.ts <file-path>");
  process.exit(1);
}

const WS_URL = "ws://127.0.0.1:4500";
let rpcId = 0;

function send(ws: WebSocket, method: string, params: Record<string, unknown>) {
  const msg = JSON.stringify({ method, id: ++rpcId, params });
  console.log(`\n→ [${rpcId}] ${method}`);
  ws.send(msg);
  return rpcId;
}

const ws = new WebSocket(WS_URL);

ws.onopen = () => {
  console.log("=== Connected ===");
  send(ws, "initialize", {
    clientInfo: { name: "dimweave-test", version: "0.0.1" },
    capabilities: { experimentalApi: true },
  });
};

let initDone = false;

ws.onmessage = (event) => {
  const data = JSON.parse(String(event.data));
  const label = data.method ?? `response:${data.id}`;
  console.log(`← [${label}]`, JSON.stringify(data, null, 2).slice(0, 500));

  if (data.id === 1 && !initDone) {
    initDone = true;
    send(ws, "thread/start", { cwd: process.cwd() });
    return;
  }

  const tid = data.result?.threadId ?? data.result?.thread?.id;
  if (data.id === 2 && tid) {
    const threadId = tid;
    console.log(`\n=== Thread: ${threadId} ===`);
    console.log(`=== Sending text with file path: ${filePath} ===\n`);
    send(ws, "turn/start", {
      threadId,
      input: [
        {
          type: "text",
          text: `Please analyze this file: ${filePath}\n\n[Attached files:\n- ${filePath}]`,
        },
      ],
    });
    return;
  }

  if (data.method === "turn/completed") {
    console.log("\n=== Turn completed ===");
    setTimeout(() => {
      ws.close();
      process.exit(0);
    }, 500);
  }
};

ws.onerror = (e) => {
  console.error("Error:", e);
  process.exit(1);
};
ws.onclose = () => console.log("Closed");
setTimeout(() => {
  console.error("Timeout");
  ws.close();
  process.exit(1);
}, 120000);
