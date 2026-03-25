import type { EventEmitter } from "node:events";
import type { AdapterState } from "./types";
import { patchResponse } from "./codex-response-patcher";
import { scheduleReconnect } from "./lifecycle";

/** Handler for dynamic tool calls — set by the adapter owner (daemon) */
export type DynamicToolHandler = (
  toolName: string,
  args: Record<string, any>,
) => Promise<{
  content: Array<{ type: string; text: string }>;
  success?: boolean;
}>;

let dynamicToolHandler: DynamicToolHandler | null = null;

export function setDynamicToolHandler(handler: DynamicToolHandler) {
  dynamicToolHandler = handler;
}

export function connectToAppServer(
  state: AdapterState,
  emitter: EventEmitter,
  log: (msg: string) => void,
  isReconnect = false,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const appWs = new WebSocket(`ws://127.0.0.1:${state.appPort}`);
    let settled = false;

    appWs.onopen = () => {
      settled = true;
      state.appServerWs = appWs;
      state.intentionalDisconnect = false;
      state.reconnectAttempts = 0;
      log(
        isReconnect
          ? "Reconnected to app-server"
          : "Connected to app-server (persistent)",
      );
      resolve();
    };

    appWs.onmessage = (event) => handleAppServerMessage(state, event, log);

    appWs.onerror = () => {
      log("App-server connection error");
      if (!settled) {
        settled = true;
        reject(new Error("Failed to connect to app-server"));
      }
    };

    appWs.onclose = () => {
      log("App-server connection closed");
      state.appServerWs = null;
      // Only auto-reconnect for established connections that drop unexpectedly.
      // If the promise was rejected by onerror (settled && !open), the caller
      // (scheduleReconnect) handles retry — don't double-schedule.
      if (!settled && !state.intentionalDisconnect) {
        settled = true;
        reject(new Error("Connection closed before open"));
      } else if (
        appWs.readyState !== WebSocket.CONNECTING &&
        !state.intentionalDisconnect &&
        state.reconnectAttempts === 0
      ) {
        // Connection was established then dropped — schedule reconnect
        scheduleReconnect(state, emitter, log);
      }
    };
  });
}

function handleAppServerMessage(
  state: AdapterState,
  event: MessageEvent,
  log: (msg: string) => void,
) {
  const data =
    typeof event.data === "string" ? event.data : event.data.toString();
  let forwarded = data;

  try {
    const parsed = JSON.parse(data);

    // Handle dynamic tool calls from app-server (server→client request)
    if (
      parsed.method === "item/tool/call" &&
      parsed.id !== undefined &&
      dynamicToolHandler
    ) {
      const toolName = parsed.params?.name ?? parsed.params?.tool;
      const toolArgs = parsed.params?.arguments ?? {};
      log(`[dynamicTool] ${toolName} called (id=${parsed.id})`);
      dynamicToolHandler(toolName, toolArgs)
        .then((result) => {
          state.appServerWs?.send(JSON.stringify({ id: parsed.id, result }));
          log(`[dynamicTool] ${toolName} responded`);
        })
        .catch((err: any) => {
          state.appServerWs?.send(
            JSON.stringify({
              id: parsed.id,
              error: { code: -1, message: err.message ?? String(err) },
            }),
          );
        });
      return; // don't forward to TUI
    }

    // Protocol discovery: log method + key params
    if (parsed.method) {
      const extra =
        parsed.method === "item/started"
          ? ` type=${parsed.params?.item?.type}`
          : parsed.method === "item/agentMessage/delta"
            ? ` itemId=${parsed.params?.itemId} len=${parsed.params?.delta?.length}`
            : parsed.method === "error"
              ? ` ${JSON.stringify(parsed.params).slice(0, 200)}`
              : "";
      // Detect 402 / deactivated workspace → emit auth_error
      if (parsed.method === "error") {
        const errMsg = parsed.params?.error?.message ?? "";
        const errInfo = parsed.params?.error?.codexErrorInfo ?? {};
        const httpStatus = errInfo?.responseStreamDisconnected?.httpStatusCode;
        if (
          httpStatus === 402 ||
          errMsg.includes("deactivated_workspace") ||
          errMsg.includes("402")
        ) {
          state.emitter?.emit("authError", {
            code: 402,
            message: errMsg.slice(0, 200),
          });
        }
      }
      log(`[proto] notification: ${parsed.method}${extra}`);
    } else if (parsed.result) {
      log(
        `[proto] response id=${parsed.id} keys=${Object.keys(parsed.result).join(",")}`,
      );
    }
    const mapping =
      parsed.id !== undefined
        ? state.upstreamToClient.get(parsed.id)
        : undefined;

    if (mapping) {
      state.upstreamToClient.delete(parsed.id);
      if (mapping.connId !== state.tuiConnId) {
        log(`Dropping stale response (upstream id ${parsed.id})`);
        return;
      }
      parsed.id = mapping.clientId;
      const raw = JSON.stringify(parsed);
      forwarded = patchResponse(parsed, raw, log);
      // If response was patched, intercept the patched version so captureAccountData sees result
      const interceptObj = forwarded !== raw ? JSON.parse(forwarded) : parsed;
      state.handler.intercept(interceptObj, mapping.connId);
    } else {
      forwarded = patchResponse(parsed, data, log);
      const interceptObj = forwarded !== data ? JSON.parse(forwarded) : parsed;
      state.handler.intercept(interceptObj);
    }
  } catch {}

  if (state.tuiWs) {
    try {
      state.tuiWs.send(forwarded);
    } catch {}
  }
}
