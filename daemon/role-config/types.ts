export type RoleId = "user" | "lead" | "coder" | "reviewer" | "tester";

export interface AgentRole {
  id: RoleId;
  label: string;
  /** developer_instructions injected into Codex thread/start */
  developerInstructions: string;
  /** Codex sandbox mode (OS-enforced) */
  sandboxMode: "read-only" | "workspace-write" | "danger-full-access";
  /** Codex approval policy */
  approvalPolicy: "on-request" | "never";
  /** Whether shell tool is enabled for Codex */
  shellTool: boolean;
  /** Default routing target for this role's output */
  defaultTarget: RoleId;
  /** Prompt appended when forwarding this agent's output */
  forwardPrompt: string;
  /** Claude --agents JSON config for this role (hard enforcement) */
  claudeAgent: {
    description: string;
    prompt: string;
    tools?: string;
    disallowedTools?: string;
    permissionMode?: string;
    model?: string;
  };
}

export const ROLE_OPTIONS: { value: RoleId; label: string }[] = [
  { value: "user", label: "User (Admin)" },
  { value: "lead", label: "Lead" },
  { value: "coder", label: "Coder" },
  { value: "reviewer", label: "Reviewer" },
  { value: "tester", label: "Tester" },
];
