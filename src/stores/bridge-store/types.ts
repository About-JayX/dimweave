import type {
  AgentInfo,
  BridgeMessage,
  PermissionBehavior,
  PermissionPrompt,
} from "@/types";

export interface TerminalLine {
  id: number;
  agent: string;
  kind: "text" | "error";
  line: string;
  timestamp: number;
}

export interface BridgeState {
  connected: boolean;
  messages: BridgeMessage[];
  agents: Record<string, AgentInfo>;
  terminalLines: TerminalLine[];
  permissionPrompts: PermissionPrompt[];
  claudeRole: string;
  codexRole: string;
  draft: string;

  setDraft: (text: string) => void;
  sendToCodex: (content: string) => void;
  clearMessages: () => void;
  launchCodexTui: () => void;
  stopCodexTui: () => void;
  respondToPermission: (
    requestId: string,
    behavior: PermissionBehavior,
  ) => Promise<void>;
  applyConfig: (config: {
    model?: string;
    reasoningEffort?: string;
    cwd?: string;
  }) => void;
  setRole: (agent: "claude" | "codex", role: string) => void;
  cleanup: () => void;
}
