import type { ServerWebSocket } from "bun";
import type { CodexAdapter } from "./adapters/codex-adapter";
import type { TuiConnectionState } from "./tui-connection-state";
import { ClaudePty } from "./claude-pty";
import {
  state,
  sendGuiEvent,
  broadcastToGui,
  type GuiSocketData,
  type GuiEvent,
} from "./daemon-state";

let claudePty: ClaudePty | null = null;

interface GuiServerDeps {
  codex: CodexAdapter;
  tuiState: TuiConnectionState;
  currentStatus: () => any;
  broadcastStatus: () => void;
  log: (msg: string) => void;
}

export function startGuiServer(port: number, deps: GuiServerDeps) {
  const { codex, tuiState, currentStatus, broadcastStatus, log } = deps;

  state.guiServer = Bun.serve({
    port,
    hostname: "127.0.0.1",
    fetch(req, server) {
      const url = new URL(req.url);
      const corsHeaders = {
        "Access-Control-Allow-Origin": "*",
        "Access-Control-Allow-Methods": "GET, OPTIONS",
        "Access-Control-Allow-Headers": "*",
      };

      if (req.method === "OPTIONS")
        return new Response(null, { headers: corsHeaders });
      if (url.pathname === "/healthz")
        return Response.json(
          { ok: true, pid: process.pid },
          { headers: corsHeaders },
        );
      if (url.pathname === "/status")
        return Response.json(currentStatus(), { headers: corsHeaders });
      if (server.upgrade(req, { data: { clientId: 0 } })) return undefined;
      return new Response("AgentBridge GUI Server", { headers: corsHeaders });
    },
    websocket: {
      open: (ws: ServerWebSocket<GuiSocketData>) => {
        ws.data.clientId = ++state.nextGuiClientId;
        state.guiClients.add(ws);
        log(`GUI client connected (#${ws.data.clientId})`);
        sendGuiEvent(ws, {
          type: "daemon_status",
          payload: currentStatus(),
          timestamp: Date.now(),
        });
      },
      close: (ws: ServerWebSocket<GuiSocketData>) => {
        state.guiClients.delete(ws);
        log(`GUI client disconnected (#${ws.data.clientId})`);
      },
      message: (ws: ServerWebSocket<GuiSocketData>, raw) => {
        handleGuiMessage(ws, raw, deps);
      },
    },
  });
}

function handleGuiMessage(
  ws: ServerWebSocket<GuiSocketData>,
  raw: string | Buffer,
  deps: GuiServerDeps,
) {
  const { codex, tuiState, currentStatus, broadcastStatus, log } = deps;
  let message: any;
  try {
    message = JSON.parse(typeof raw === "string" ? raw : raw.toString());
  } catch {
    return;
  }

  switch (message.type) {
    case "send_to_codex": {
      if (!tuiState.canReply()) {
        sendGuiEvent(ws, {
          type: "system_log",
          payload: { level: "error", message: "Codex is not ready." },
          timestamp: Date.now(),
        });
        return;
      }
      const injected = codex.injectMessage(message.content);
      if (injected) {
        broadcastToGui({
          type: "agent_message",
          payload: {
            id: `gui_${Date.now()}`,
            source: "claude",
            content: message.content,
            timestamp: Date.now(),
          },
          timestamp: Date.now(),
        });
      }
      return;
    }
    case "get_status":
      sendGuiEvent(ws, {
        type: "daemon_status",
        payload: currentStatus(),
        timestamp: Date.now(),
      });
      return;
    case "launch_codex_tui": {
      if (!state.codexBootstrapped) {
        sendGuiEvent(ws, {
          type: "system_log",
          payload: {
            level: "error",
            message: "Codex app-server is not ready yet.",
          },
          timestamp: Date.now(),
        });
        return;
      }
      if (codex.activeThreadId) {
        sendGuiEvent(ws, {
          type: "system_log",
          payload: {
            level: "warn",
            message: "Codex session is already active.",
          },
          timestamp: Date.now(),
        });
        return;
      }
      log("Initializing Codex session from GUI...");
      broadcastToGui({
        type: "system_log",
        payload: { level: "info", message: "Connecting to Codex..." },
        timestamp: Date.now(),
      });

      codex.initSession().then((result) => {
        if (result.success) {
          log("Codex session initialized successfully");
          tuiState.markBridgeReady();
          broadcastToGui({
            type: "agent_status",
            payload: {
              agent: "codex",
              status: "connected",
              threadId: codex.activeThreadId,
            },
            timestamp: Date.now(),
          });
          broadcastToGui({
            type: "system_log",
            payload: {
              level: "info",
              message: `Codex connected! Thread: ${codex.activeThreadId}`,
            },
            timestamp: Date.now(),
          });
          broadcastStatus();
        } else {
          log(`Codex session init failed: ${result.error}`);
          broadcastToGui({
            type: "system_log",
            payload: {
              level: "error",
              message: `Codex connection failed: ${result.error}`,
            },
            timestamp: Date.now(),
          });
        }
      });
      return;
    }
    case "apply_config": {
      const { model, reasoningEffort, cwd } = message;
      log(
        `Applying config: model=${model ?? "-"}, reasoning=${reasoningEffort ?? "-"}, cwd=${cwd ?? "-"}`,
      );

      // Disconnect current session
      codex.disconnect();
      tuiState.handleCodexExit();
      broadcastToGui({
        type: "system_log",
        payload: { level: "info", message: "Reconnecting with new config..." },
        timestamp: Date.now(),
      });

      // Reconnect with new settings
      codex
        .ensureConnected()
        .then(() => {
          return codex.initSession({ model, reasoningEffort, cwd });
        })
        .then((result) => {
          if (result.success) {
            tuiState.markBridgeReady();
            broadcastToGui({
              type: "agent_status",
              payload: {
                agent: "codex",
                status: "connected",
                threadId: codex.activeThreadId,
              },
              timestamp: Date.now(),
            });
            broadcastToGui({
              type: "system_log",
              payload: {
                level: "info",
                message: `Config applied! Model: ${model ?? "default"}`,
              },
              timestamp: Date.now(),
            });
            broadcastStatus();
          } else {
            broadcastToGui({
              type: "system_log",
              payload: {
                level: "error",
                message: `Config apply failed: ${result.error}`,
              },
              timestamp: Date.now(),
            });
          }
        });
      return;
    }
    case "launch_claude": {
      if (claudePty?.running) {
        sendGuiEvent(ws, {
          type: "system_log",
          payload: { level: "warn", message: "Claude is already running." },
          timestamp: Date.now(),
        });
        return;
      }
      const cwd = message.cwd ?? process.cwd();
      const cols = message.cols ?? 120;
      const rows = message.rows ?? 30;
      log(`Launching Claude PTY in ${cwd} (${cols}x${rows})`);

      claudePty = new ClaudePty((data) => {
        // Forward raw PTY output to all GUI clients
        broadcastToGui({
          type: "pty_data",
          payload: { data },
          timestamp: Date.now(),
        });
      });

      claudePty.setOnExit((code: number) => {
        claudePty = null;
        broadcastToGui({
          type: "agent_status",
          payload: { agent: "claude", status: "disconnected" },
          timestamp: Date.now(),
        });
        broadcastToGui({
          type: "pty_data",
          payload: { data: `\r\n[Process exited with code ${code}]\r\n` },
          timestamp: Date.now(),
        });
        broadcastStatus();
      });

      claudePty.start(cwd, cols, rows);
      return;
    }
    case "pty_input": {
      if (!claudePty?.running) return;
      claudePty.write(message.data);
      return;
    }
    case "pty_resize": {
      if (!claudePty?.running) return;
      claudePty.resize(message.cols, message.rows);
      return;
    }
    case "stop_claude": {
      if (claudePty?.running) claudePty.stop();
      claudePty = null;
      log("Claude stopped from GUI");
      broadcastToGui({
        type: "agent_status",
        payload: { agent: "claude", status: "disconnected" },
        timestamp: Date.now(),
      });
      broadcastStatus();
      return;
    }
    case "stop_codex_tui": {
      log("Disconnecting Codex from GUI...");
      codex.disconnect();
      tuiState.handleCodexExit();
      broadcastToGui({
        type: "agent_status",
        payload: { agent: "codex", status: "disconnected" },
        timestamp: Date.now(),
      });
      broadcastToGui({
        type: "system_log",
        payload: { level: "info", message: "Codex disconnected." },
        timestamp: Date.now(),
      });
      broadcastStatus();
      return;
    }
  }
}
