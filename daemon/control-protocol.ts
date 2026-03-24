import type { BridgeMessage } from "./types";

export interface DaemonStatus {
  bridgeReady: boolean;
  tuiConnected: boolean;
  threadId: string | null;
  queuedMessageCount: number;
  proxyUrl: string;
  appServerUrl: string;
  pid: number;
}

export type ControlClientMessage =
  | { type: "claude_connect" }
  | { type: "claude_disconnect" }
  | { type: "route_message"; requestId: string; message: BridgeMessage }
  | { type: "fetch_messages"; requestId: string }
  | { type: "status" };

export type ControlServerMessage =
  | { type: "routed_message"; message: BridgeMessage }
  | {
      type: "route_result";
      requestId: string;
      success: boolean;
      error?: string;
    }
  | {
      type: "fetch_messages_result";
      requestId: string;
      messages: BridgeMessage[];
    }
  | { type: "status"; status: DaemonStatus };
