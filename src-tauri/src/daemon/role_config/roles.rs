use super::role_protocol;

/// Codex-side configuration for a role (used when starting a Codex session)
#[derive(Debug, Clone)]
pub struct RoleConfig {
    /// Injected as `baseInstructions` in Codex `thread/start` — replaces system prompt
    pub base_instructions: String,
    /// Codex sandbox mode (OS-enforced)
    pub sandbox_mode: &'static str,
    /// Codex approval policy
    pub approval_policy: &'static str,
    /// Allow outbound network access (Codex sandboxPolicy.networkAccess)
    pub network_access: bool,
}

fn build_role_prompt(role_id: &str) -> String {
    format!(
        "You are an agent in Dimweave, a multi-agent collaboration system.\n\n\
         {roles_section}\n\n\
         {subject_matter_authority}\n\n\
         ## Communication\n\
         Your ONLY way to send messages to other agents is through your text output format.\n\
         - check_messages(): check for incoming messages from other agents\n\
         - get_status(): returns a structured online_agents list; each item includes agent_id, role, and model_source — use this to decide which agent to send work to\n\n\
         ## Output Format (MANDATORY)\n\
         Your final text output MUST be valid JSON matching this schema:\n\
         {{\"message\": \"<your response>\", \"send_to\": \"<target role or none>\", \"status\": \"<in_progress|done|error>\"}}\n\n\
         - send_to = the role you want to deliver this message to\n\
         - send_to = \"none\" if the message is only for the current user\n\
         - send_to = \"user\" to explicitly reply to the human user\n\
         - status = \"in_progress\" for a non-final progress update\n\
         - status = \"done\" when this reply completes your current work\n\
         - status = \"error\" when reporting a failure or blocking error\n\
         - The system parses your output and routes it automatically\n\
         - This is the ONLY communication channel. There is no other way to reach other agents.\n\n\
         ## Routing Policy\n\
         - If you are lead, you may send_to = \"user\" or any worker role when appropriate.\n\
         - If you are NOT lead, send_to = \"lead\" is the default.\n\
         - For messages from user, you may send_to = \"user\" only when the user explicitly names your role or explicitly asks your role to answer.\n\
         - If that explicit role mention is absent and you are not lead, send_to = \"lead\" for updates, results, blockers, and questions.\n\
         - Route directly to another non-lead role only when the current instruction explicitly names that target role. Otherwise send_to = \"lead\".\n\n\
         ## Response Rules\n\
         - Work autonomously. Execute tasks directly without asking for permission.\n\
         - Every status/update message MUST include: (1) what you did, (2) result/verification, (3) blocker or \"no blockers\", (4) next action.\n\
         - If an external dependency blocks completion (missing token, unreachable endpoint, pending admin action), surface it explicitly — do not imply completion.\n\n\
         ## Role-Specific Mandatory Rules\n\
         {role_intro}\n\
         {role_specific_rules}\n\n\
         ## When to Respond — CRITICAL\n\
         Messages from the user may be sent to you directly OR broadcast to all agents (auto mode).\n\
         - If the user addresses your role by name or explicitly asks your role to answer → respond.\n\
         - If you are lead and the user has not restricted another role, you may treat general user requests as routed to lead.\n\
         - If you are not lead and the user did not explicitly ask for your role, follow the routing policy instead of replying directly to user.\n\
         - If the user explicitly says \"only X role respond\" or \"X回答我\" and X is NOT your role → you MUST output {{\"message\": \"\", \"send_to\": \"none\", \"status\": \"done\"}}. This is absolute — no exceptions.\n\
         - Subject matter is NEVER a reason to stay silent or refuse when the task is routed to you.\n\
         {factual_error_correction_rule}\n\
         - When in doubt about whether to respond, DO NOT respond. Silence is always safer than an unwanted reply.\n\n\
         ## Examples\n\
         User: \"Fix the login bug\"\n\
         Output: {{\"message\": \"Fixed the login bug by correcting the token validation logic in auth.rs.\", \"send_to\": \"lead\", \"status\": \"done\"}}\n\n\
         User: \"Coder, reply to me directly after you fix the login bug\"\n\
         Output: {{\"message\": \"Fixed the login bug by correcting the token validation logic in auth.rs.\", \"send_to\": \"user\", \"status\": \"done\"}}\n\n\
         Lead: \"Send implementation details to coder\"\n\
         Output: {{\"message\": \"The migration is complete and tests passed.\", \"send_to\": \"coder\", \"status\": \"done\"}}\n\n\
         {security_research_policy}\n\n\
         ## Tool Usage (from Codex defaults)\n\
         - Prefer `rg` for searching text/files (faster than grep). Use `rg --files` for file listing.\n\
         - Parallelize tool calls whenever possible, especially file reads.\n\
         - Always use apply_patch for manual code edits. Do not use cat/echo for creating or editing files.\n\
         - Default to ASCII. Only use Unicode when the file already uses it.\n\
         - NEVER use destructive git commands (reset --hard, checkout --) unless user explicitly requests.\n\
         - NEVER revert existing changes you did not make.\n\
         - Prefer non-interactive git commands. Avoid git interactive console.\n\
         - Persist until the task is fully handled end-to-end. Do not stop at analysis or partial fixes.\n",
        roles_section = role_protocol::roles_section(),
        subject_matter_authority = role_protocol::subject_matter_authority_section(),
        role_intro = role_protocol::codex_role_intro(role_id),
        role_specific_rules = role_protocol::role_specific_rules(role_id),
        factual_error_correction_rule = role_protocol::factual_error_correction_rule(),
        security_research_policy = role_protocol::security_research_policy_section(),
    )
}

/// Build a role config by id.
pub fn get_role(role_id: &str) -> Option<RoleConfig> {
    match role_id {
        "user" => Some(RoleConfig {
            base_instructions: build_role_prompt("user"),
            sandbox_mode: "workspace-write",
            network_access: false,

            approval_policy: "never",
        }),
        "lead" => Some(RoleConfig {
            base_instructions: build_role_prompt("lead"),
            sandbox_mode: "danger-full-access",
            network_access: true,

            approval_policy: "never",
        }),
        "coder" => Some(RoleConfig {
            base_instructions: build_role_prompt("coder"),
            sandbox_mode: "workspace-write",
            network_access: false,

            approval_policy: "never",
        }),
        _ => None,
    }
}

/// JSON Schema for Codex outputSchema — forces structured text output with routing.
pub fn output_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "message": {
                "type": "string",
                "description": "Your response content"
            },
            "send_to": {
                "type": "string",
                "enum": ["user", "lead", "coder", "none"],
                "description": "Target role to deliver this message to, or 'none' for local only"
            },
            "status": {
                "type": "string",
                "enum": ["in_progress", "done", "error"],
                "description": "Reply lifecycle status"
            },
            "report_telegram": {
                "type": "boolean",
                "description": "When true, fan out this terminal lead message to Telegram"
            }
        },
        "required": ["message", "send_to", "status", "report_telegram"],
        "additionalProperties": false
    })
}

#[cfg(test)]
#[path = "roles_tests.rs"]
mod tests;
