import type { BridgeMessage } from "../types";

export type AgentStatus = "disconnected" | "connecting" | "connected" | "error";

export interface AgentAdapter {
  readonly name: string;
  readonly displayName: string;
  readonly status: AgentStatus;

  start(): Promise<void>;
  stop(): void;
  sendMessage(content: string): Promise<{ success: boolean; error?: string }>;

  on(event: "message", handler: (msg: BridgeMessage) => void): this;
  on(event: "statusChange", handler: (status: AgentStatus) => void): this;
  on(event: "error", handler: (err: Error) => void): this;

  emit(event: "message", msg: BridgeMessage): boolean;
  emit(event: "statusChange", status: AgentStatus): boolean;
  emit(event: "error", err: Error): boolean;
}
