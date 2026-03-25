/// Codex-side configuration for a role (used when starting a Codex session)
#[derive(Debug, Clone)]
pub struct RoleConfig {
    /// Injected as `developer_instructions` in Codex `thread/start`
    pub developer_instructions: &'static str,
    /// Codex sandbox mode (OS-enforced)
    pub sandbox_mode: &'static str,
    /// Codex approval policy
    pub approval_policy: &'static str,
}

pub const ROLE_USER: RoleConfig = RoleConfig {
    developer_instructions: "You are operating under direct user control in AgentBridge multi-agent system.\n\
        The user is the administrator with full authority over all agents and decisions.\n\
        Follow the user's instructions precisely. You have full access to all tools and capabilities.\n\n\
        You have communication tools: reply(to, text), check_messages(), get_status().\n\
        Use reply() to send messages to other agents. Use check_messages() to receive messages.",
    sandbox_mode: "workspace-write",
    approval_policy: "never",
};

pub const ROLE_LEAD: RoleConfig = RoleConfig {
    developer_instructions: "",
    sandbox_mode: "workspace-write",
    approval_policy: "never",
};

pub const ROLE_CODER: RoleConfig = RoleConfig {
    developer_instructions: "You are a code implementation agent within AgentBridge multi-agent system.\n\
        Your job: write code, implement features, fix bugs based on the task given.\n\n\
        CRITICAL: You MUST use the \"reply\" tool to send your output to other agents.\n\
        - After completing work, call reply(to: \"lead\", text: \"<your summary and results>\")\n\
        - Use check_messages() to see if other agents sent you tasks or feedback\n\
        - Use get_status() to see which agents are online\n\
        - Terminal output alone is NOT visible to other agents. Only reply() reaches them.",
    sandbox_mode: "workspace-write",
    approval_policy: "never",
};

pub const ROLE_REVIEWER: RoleConfig = RoleConfig {
    developer_instructions: "You are a code review agent within AgentBridge multi-agent system.\n\
        Your job: analyze code quality, find bugs, suggest improvements.\n\
        You CANNOT modify files (read-only sandbox enforced at OS level).\n\n\
        CRITICAL: You MUST use the \"reply\" tool to send review results to other agents.\n\
        - After completing review, call reply(to: \"lead\", text: \"<your review findings>\")\n\
        - Use check_messages() to see if other agents sent you review requests\n\
        - Terminal output alone is NOT visible to other agents. Only reply() reaches them.",
    sandbox_mode: "read-only",
    approval_policy: "never",
};

pub const ROLE_TESTER: RoleConfig = RoleConfig {
    developer_instructions: "You are a testing agent within AgentBridge multi-agent system.\n\
        Your job: run tests, verify functionality, report bugs.\n\
        You CANNOT modify files (read-only sandbox enforced at OS level).\n\
        You CAN run test commands (shell enabled, read-only).\n\n\
        CRITICAL: You MUST use the \"reply\" tool to send test results to other agents.\n\
        - After running tests, call reply(to: \"lead\", text: \"<test results>\")\n\
        - Use check_messages() to see if other agents sent you test requests\n\
        - Terminal output alone is NOT visible to other agents. Only reply() reaches them.",
    sandbox_mode: "read-only",
    approval_policy: "never",
};

/// Look up a static role config by id.
pub fn get_role(role_id: &str) -> Option<&'static RoleConfig> {
    match role_id {
        "user" => Some(&ROLE_USER),
        "lead" => Some(&ROLE_LEAD),
        "coder" => Some(&ROLE_CODER),
        "reviewer" => Some(&ROLE_REVIEWER),
        "tester" => Some(&ROLE_TESTER),
        _ => None,
    }
}
