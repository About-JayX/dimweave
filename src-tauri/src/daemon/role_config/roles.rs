use super::{codex_base_prompt, role_protocol};

/// Codex-side configuration for a role (used when starting a Codex session)
#[derive(Debug, Clone)]
pub struct RoleConfig {
    /// Injected as `baseInstructions` in Codex `thread/start` —
    /// merged from original Codex GPT-5.4 system prompt + Dimweave role instructions
    pub base_instructions: String,
    /// Codex sandbox mode (OS-enforced)
    pub sandbox_mode: &'static str,
    /// Codex approval policy
    pub approval_policy: &'static str,
    /// Allow outbound network access (Codex sandboxPolicy.networkAccess)
    pub network_access: bool,
}

fn role_examples(role_id: &str) -> &'static str {
    match role_id {
        "lead" =>
            "User: \"Fix the login bug\"\n\
             Output: {\"message\": \"Delegating login bug fix to coder. Task: investigate token validation in auth.rs.\", \
             \"target\": {\"kind\": \"role\", \"role\": \"coder\", \"agentId\": \"\"}, \"status\": \"in_progress\"}\n\n\
             Coder: \"Fixed the login bug by correcting the token validation logic in auth.rs.\"\n\
             Output: {\"message\": \"Login bug fix verified and committed. The issue was an expired token check in auth.rs line 42.\", \
             \"target\": {\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}, \"status\": \"done\"}\n\n\
             User: \"Lead, what is the project status?\"\n\
             Output: {\"message\": \"Project status: all tasks complete, no blockers.\", \
             \"target\": {\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}, \"status\": \"done\"}",
        "coder" =>
            "Lead: \"Fix the login bug in auth.rs\"\n\
             Output: {\"message\": \"Fixed the login bug by correcting the token validation logic in auth.rs.\", \
             \"target\": {\"kind\": \"role\", \"role\": \"lead\", \"agentId\": \"\"}, \"status\": \"done\"}\n\n\
             User: \"Coder, reply to me directly after you fix the login bug\"\n\
             Output: {\"message\": \"Fixed the login bug by correcting the token validation logic in auth.rs.\", \
             \"target\": {\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}, \"status\": \"done\"}",
        _ =>
            "User: \"Do something\"\n\
             Output: {\"message\": \"Done.\", \
             \"target\": {\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}, \"status\": \"done\"}",
    }
}

fn build_role_prompt(role_id: &str) -> String {
    format!(
        "{identity}\n\n\
         # Dimweave Multi-Agent System\n\n\
         You are also an agent in Dimweave, a multi-agent collaboration system.\n\n\
         {roles_section}\n\n\
         {subject_matter_authority}\n\n\
         ## Communication\n\
         Your ONLY way to send messages to other agents is through your text output format.\n\
         - check_messages(): check for incoming messages from other agents\n\
         - get_status(): returns a structured online_agents list; each item includes agent_id, role, and model_source \
         — use this to decide which agent to send work to\n\n\
         ## Output Format (MANDATORY)\n\
         Your final text output MUST be valid JSON matching this schema:\n\
         {{\"message\": \"<your response>\", \"target\": {{\"kind\": \"<user|role|agent>\", ...}}, \
         \"status\": \"<in_progress|done|error>\"}}\n\n\
         - target = the routing destination for this message (all three fields kind/role/agentId are required)\n\
         - target = {{\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}} to reply to the human user\n\
         - target = {{\"kind\": \"role\", \"role\": \"lead\", \"agentId\": \"\"}} to send to lead\n\
         - target = {{\"kind\": \"role\", \"role\": \"coder\", \"agentId\": \"\"}} to send to coder\n\
         - target = {{\"kind\": \"agent\", \"role\": \"\", \"agentId\": \"<id>\"}} to send to a specific agent instance\n\
         - To stay silent, output an empty message with status done\n\
         - status = \"in_progress\" for a non-final progress update\n\
         - status = \"done\" when this reply completes your current work\n\
         - status = \"error\" when reporting a failure or blocking error\n\
         - The system parses your output and routes it automatically\n\
         - This is the ONLY communication channel. There is no other way to reach other agents.\n\n\
         ## Routing Policy\n\
         - If you are lead, you may target the user or any worker role when appropriate.\n\
         - If you are NOT lead, target = {{\"kind\": \"role\", \"role\": \"lead\", \"agentId\": \"\"}} is the default.\n\
         - For messages from user, you may target the user only when the user explicitly names your role \
         or explicitly asks your role to answer.\n\
         - If that explicit role mention is absent and you are not lead, \
         target lead for updates, results, blockers, and questions.\n\
         - Route directly to another non-lead role only when the current instruction explicitly names that target role. \
         Otherwise target lead.\n\n\
         ## Response Rules\n\
         - Work autonomously. Execute tasks directly without asking for permission.\n\
         - Every status/update message MUST include: (1) what you did, (2) result/verification, \
         (3) blocker or \"no blockers\", (4) next action.\n\
         - If an external dependency blocks completion (missing token, unreachable endpoint, pending admin action), \
         surface it explicitly — do not imply completion.\n\n\
         ## Role-Specific Mandatory Rules\n\
         {role_intro}\n\
         {role_specific_rules}\n\n\
         ## When to Respond — CRITICAL\n\
         Messages from the user may be sent to you directly OR broadcast to all agents (auto mode).\n\
         - If the user addresses your role by name or explicitly asks your role to answer → respond.\n\
         - If you are lead and the user has not restricted another role, \
         you may treat general user requests as routed to lead.\n\
         - If you are not lead and the user did not explicitly ask for your role, \
         follow the routing policy instead of replying directly to user.\n\
         - If the user explicitly says \"only X role respond\" or \"X回答我\" and X is NOT your role \
         → you MUST output {{\"message\": \"\", \"target\": {{\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}}, \"status\": \"done\"}}. \
         This is absolute — no exceptions.\n\
         - Subject matter is NEVER a reason to stay silent or refuse when the task is routed to you.\n\
         {factual_error_correction_rule}\n\
         - When in doubt about whether to respond, DO NOT respond. \
         Silence is always safer than an unwanted reply.\n\n\
         ## Examples\n\
         {role_examples}\n\n\
         {coding_capabilities}\n\n\
         {communication_style}\n\n\
         {security_research_policy}",
        identity = codex_base_prompt::identity_and_personality(),
        roles_section = role_protocol::roles_section(),
        subject_matter_authority = role_protocol::subject_matter_authority_section(),
        role_examples = role_examples(role_id),
        role_intro = role_protocol::codex_role_intro(role_id),
        role_specific_rules = role_protocol::role_specific_rules(role_id),
        factual_error_correction_rule = role_protocol::factual_error_correction_rule(),
        coding_capabilities = codex_base_prompt::coding_capabilities(),
        communication_style = codex_base_prompt::communication_style(),
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
            "target": {
                "type": "object",
                "description": "Routing target (all fields required).",
                "properties": {
                    "kind": {
                        "type": "string",
                        "enum": ["user", "role", "agent"],
                        "description": "Target type"
                    },
                    "role": {
                        "type": "string",
                        "description": "Required when kind is role"
                    },
                    "agentId": {
                        "type": "string",
                        "description": "Required when kind is agent"
                    }
                },
                "required": ["kind", "role", "agentId"],
                "additionalProperties": false
            },
            "status": {
                "type": "string",
                "enum": ["in_progress", "done", "error"],
                "description": "Reply lifecycle status"
            }
        },
        "required": ["message", "target", "status"],
        "additionalProperties": false
    })
}

#[cfg(test)]
#[path = "roles_tests.rs"]
mod tests;
