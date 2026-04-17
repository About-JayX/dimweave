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
            "### GATE 1 — plan_approval_gate (the ONLY `status=in_progress` you may send to user)\n\
             User: \"Fix the login bug\"\n\
             Output: {\"message\": \"Proposed plan:\\n1. Investigate token validation in auth.rs:42\\n2. Identify root cause of expired-token check\\n3. Patch + write regression test\\nPlease confirm or adjust.\", \
             \"target\": {\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}, \"status\": \"in_progress\"}\n\n\
             ### After user confirms plan — DELEGATE (all further in_progress goes to coder, NOT user)\n\
             User: \"OK, proceed.\"\n\
             Output: {\"message\": \"Task T1: investigate auth.rs token validation (file: auth.rs, allowed_files=[auth.rs], max_files_changed=1). Acceptance: root cause identified with line reference. Report back with evidence.\", \
             \"target\": {\"kind\": \"role\", \"role\": \"coder\", \"agentId\": \"\"}, \"status\": \"in_progress\"}\n\n\
             ### Coder returns progress → lead acks coder, DOES NOT forward to user\n\
             Coder: \"Found it — auth.rs:42, expired-token check returns true for null expiry. Evidence: traced via test run.\"\n\
             Output: {\"message\": \"Accepted T1 evidence. Task T2: patch auth.rs:42 to treat null expiry as expired; add regression test. Max 1 file + 1 test file changed. Report back for review.\", \
             \"target\": {\"kind\": \"role\", \"role\": \"coder\", \"agentId\": \"\"}, \"status\": \"in_progress\"}\n\n\
             ### GATE 2 — external_blocker_gate (only when a dependency requires user input)\n\
             Coder: \"Cannot complete T2 — prod secrets are required to run the regression test but not present.\"\n\
             Output: {\"message\": \"Blocked on T2: regression test needs PROD_AUTH_TOKEN to exercise the real token validator. Please provide the value or confirm we should mock it.\", \
             \"target\": {\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}, \"status\": \"error\"}\n\n\
             ### GATE 3 — final_acceptance_gate (only after ALL tasks verified + deep review + merge)\n\
             Coder: \"All tasks complete. T1+T2 verified, regression test passes, commit abcd123.\"\n\
             [lead runs final deep review internally, merges worktree]\n\
             Output: {\"message\": \"Login bug fix complete. Root cause: auth.rs:42 treated null expiry as valid. Fix: treat null as expired + regression test. Commit abcd123 on main. No blockers.\", \
             \"target\": {\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}, \"status\": \"done\"}\n\n\
             ### PROTOCOL VIOLATIONS — do NOT do these\n\
             BAD: {\"message\": \"Understood, I'll delegate to coder now.\", \"target\": {\"kind\": \"user\", ...}, \"status\": \"in_progress\"}  ← ack-to-user during execution is FORBIDDEN\n\
             BAD: {\"message\": \"Coder is working on T1 now.\", \"target\": {\"kind\": \"user\", ...}, \"status\": \"in_progress\"}  ← forwarding coder progress to user is FORBIDDEN\n\
             BAD: [multiple structured outputs in one turn]  ← emit EXACTLY ONE per turn",
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
         {{\"message\": \"<your response>\", \"target\": {{\"kind\": \"<user|role|agent>\", \"role\": \"\", \"agentId\": \"\"}}, \
         \"status\": \"<in_progress|done|error>\"}}\n\n\
         - target is the routing destination; ALL three fields (kind/role/agentId) are required. Fill unused fields with \"\".\n\
         - target = {{\"kind\": \"user\", \"role\": \"\", \"agentId\": \"\"}} to reply to the human user\n\
         - target = {{\"kind\": \"role\", \"role\": \"lead\", \"agentId\": \"\"}} to send to lead (by role; works when you don't know a specific lead's agent_id)\n\
         - target = {{\"kind\": \"role\", \"role\": \"coder\", \"agentId\": \"\"}} to send to coder (by role; use this when delegating a task from user to coder)\n\
         - target = {{\"kind\": \"agent\", \"role\": \"\", \"agentId\": \"<id>\"}} to send to a SPECIFIC agent by id\n\
         - Incoming agent messages arrive as `Message from <role> [<agent_id>] (status: <status>):\\n<body>`. When you are a WORKER replying to that specific delegator (not the user), you may use `{{\"kind\": \"agent\", \"agentId\": \"<that sender agent_id>\"}}` to route back precisely — this is an OPTIONAL refinement on top of the default `{{\"kind\": \"role\", \"role\": \"lead\"}}` target; both work. Delegation decisions (user→coder, lead→coder, etc.) follow the Routing Policy below — agent_id-first is a reply-precision tool, not a reason to skip delegation.\n\
         - To stay silent, output an empty message with status done\n\
         - status = \"in_progress\" for a non-final progress update\n\
         - status = \"done\" when this reply completes your current work\n\
         - status = \"error\" when reporting a failure or blocking error\n\
         - The system parses your output and routes it automatically\n\
         - This is the ONLY communication channel. There is no other way to reach other agents.\n\n\
         ## Routing Policy\n\
         - If you are lead, target selection is GOVERNED by the \"Lead Escalation Gate\" rules in your role-specific section below. `target=user` is RESTRICTED to 4 gate scenarios; routine coordination always targets coder.\n\
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
