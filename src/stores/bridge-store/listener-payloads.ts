import type {
  BridgeMessage,
  PermissionPrompt,
  ProviderSessionInfo,
  RuntimeHealthInfo,
} from "@/types";

export interface AgentMessagePayload {
  payload: BridgeMessage;
  timestamp: number;
}

export interface SystemLogPayload {
  level: string;
  message: string;
}

export interface AgentStatusPayload {
  agent: string;
  online: boolean;
  exitCode?: number;
  providerSession?: ProviderSessionInfo;
  role?: string;
}

export interface RuntimeHealthPayload {
  health?: RuntimeHealthInfo | null;
}

export interface PermissionPromptPayload extends PermissionPrompt {}

export interface CodexStreamPayload {
  kind:
    | "thinking"
    | "delta"
    | "message"
    | "turnDone"
    | "activity"
    | "reasoning"
    | "commandOutput";
  text?: string;
  status?: string;
  label?: string;
}

export interface ClaudeStreamPayload {
  kind: "thinkingStarted" | "preview" | "done" | "reset";
  text?: string;
}
