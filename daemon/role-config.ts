export type RoleId = "lead" | "coder" | "reviewer" | "tester";

export interface AgentRole {
  id: RoleId;
  label: string;
  /** developer_instructions injected into Codex thread/start */
  developerInstructions: string;
  /** Codex sandbox mode (OS-enforced) */
  sandboxMode: "read-only" | "workspace-write" | "danger-full-access";
  /** Codex approval policy */
  approvalPolicy: "on-request" | "never";
  /** Whether shell tool is enabled */
  shellTool: boolean;
  /** Prompt appended when forwarding this agent's output to Lead */
  forwardPrompt: string;
}

export const ROLES: Record<RoleId, AgentRole> = {
  lead: {
    id: "lead",
    label: "Lead",
    developerInstructions: "",
    sandboxMode: "workspace-write",
    approvalPolicy: "never",
    shellTool: true,
    forwardPrompt: "",
  },
  coder: {
    id: "coder",
    label: "Coder",
    developerInstructions: `You are a code implementation agent within AgentBridge.
Your job: write code, implement features, fix bugs based on the task given.
Generate clear, well-structured code with proper error handling.
Your output will be reviewed by a Lead agent who decides whether to apply it.`,
    sandboxMode: "workspace-write",
    approvalPolicy: "never",
    shellTool: true,
    forwardPrompt:
      "[Coder completed task]\nReview the implementation below and decide whether to apply, modify, or reject it.",
  },
  reviewer: {
    id: "reviewer",
    label: "Reviewer",
    developerInstructions: `You are a code review agent within AgentBridge.
Your job: analyze code quality, find bugs, suggest improvements.
You CANNOT modify files (read-only sandbox enforced at OS level).
Provide specific, actionable feedback with file paths and line numbers.
Your review will be sent to the Lead agent for final decision.`,
    sandboxMode: "read-only",
    approvalPolicy: "on-request",
    shellTool: false,
    forwardPrompt:
      "[Reviewer completed review]\nReview results below. Apply suggested fixes if appropriate.",
  },
  tester: {
    id: "tester",
    label: "Tester",
    developerInstructions: `You are a testing agent within AgentBridge.
Your job: run tests, verify functionality, report bugs.
You CANNOT modify files (read-only sandbox enforced at OS level).
You CAN run test commands (shell enabled, read-only).
Report test results with pass/fail status and error details.`,
    sandboxMode: "read-only",
    approvalPolicy: "on-request",
    shellTool: true,
    forwardPrompt:
      "[Tester completed testing]\nTest results below. Fix any failures if needed.",
  },
};

export const ROLE_OPTIONS: { value: RoleId; label: string }[] = [
  { value: "lead", label: "Lead" },
  { value: "coder", label: "Coder" },
  { value: "reviewer", label: "Reviewer" },
  { value: "tester", label: "Tester" },
];
