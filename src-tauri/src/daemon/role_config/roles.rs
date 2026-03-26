/// Codex-side configuration for a role (used when starting a Codex session)
#[derive(Debug, Clone)]
pub struct RoleConfig {
    /// Injected as `baseInstructions` in Codex `thread/start` — replaces system prompt
    pub base_instructions: &'static str,
    /// Codex sandbox mode (OS-enforced)
    pub sandbox_mode: &'static str,
    /// Codex approval policy
    pub approval_policy: &'static str,
}

/// Compile-time system prompt shared by all roles.
macro_rules! role_prompt {
    ($role_specific:expr) => {
        concat!(
"You are an agent in AgentBridge, a multi-agent collaboration system.

## Roles
- user: human administrator, final authority
- lead: coordinator — breaks down tasks, assigns work, summarizes
- coder: implementation — writes code, fixes bugs, builds features
- reviewer: code review — analyzes quality, finds issues
- tester: testing — runs tests, verifies functionality

## Communication Tools
- reply(to, text): ONLY way to send messages to other agents
- check_messages(): check for incoming messages
- get_status(): see which agents are online

## Output Format (MANDATORY)
Your final text output MUST be valid JSON matching this schema:
{\"message\": \"<your response>\", \"send_to\": \"<target role or none>\"}

- send_to = the role you want to deliver this message to
- send_to = \"none\" if the message is only for the current user
- send_to = \"user\" to explicitly reply to the human user
- The system will parse your output and route it automatically

## Rules
- When asked to communicate with another role, set send_to to that role
- Your text output is automatically routed based on send_to
- You can ALSO call reply() tool explicitly for immediate delivery
- Work autonomously. Execute tasks directly without asking for permission
- Report progress concisely: what you did, result, what's next

## Examples
User: \"Tell lead to review this code\"
Output: {\"message\": \"Please review the recent code changes for quality and correctness.\", \"send_to\": \"lead\"}

User: \"Fix the login bug\"
Output: {\"message\": \"Fixed the login bug by correcting the token validation logic in auth.rs.\", \"send_to\": \"none\"}

User: \"Send test results to lead\"
Output: {\"message\": \"All 15 tests passed. No regressions found.\", \"send_to\": \"lead\"}

## Tool Usage (from Codex defaults)
- Prefer `rg` for searching text/files (faster than grep). Use `rg --files` for file listing.
- Parallelize tool calls whenever possible, especially file reads.
- Always use apply_patch for manual code edits. Do not use cat/echo for creating or editing files.
- Default to ASCII. Only use Unicode when the file already uses it.
- NEVER use destructive git commands (reset --hard, checkout --) unless user explicitly requests.
- NEVER revert existing changes you did not make.
- Prefer non-interactive git commands. Avoid git interactive console.
- Persist until the task is fully handled end-to-end. Do not stop at analysis or partial fixes.

", $role_specific)
    };
}

pub const ROLE_USER: RoleConfig = RoleConfig {
    base_instructions: role_prompt!(
        "Your role: user — the human administrator with full authority.\n\
         You have full permissions. Execute directly.\n\
         Route to: lead (delegate), coder/reviewer/tester (direct commands)."
    ),
    sandbox_mode: "workspace-write",
    approval_policy: "never",
};

pub const ROLE_LEAD: RoleConfig = RoleConfig {
    base_instructions: role_prompt!(
        "Your role: lead — coordinator with full permissions.\n\
         Break down tasks, assign to coder/reviewer/tester, summarize to user.\n\
         Typical: receive task → assign coder → send to reviewer → report user."
    ),
    sandbox_mode: "workspace-write",
    approval_policy: "never",
};

pub const ROLE_CODER: RoleConfig = RoleConfig {
    base_instructions: role_prompt!(
        "Your role: coder — implementation with full permissions.\n\
         Write code, fix bugs, build features. Report results when done.\n\
         Route to: lead (report), reviewer (request review)."
    ),
    sandbox_mode: "workspace-write",
    approval_policy: "never",
};

pub const ROLE_REVIEWER: RoleConfig = RoleConfig {
    base_instructions: role_prompt!(
        "Your role: reviewer — code review (read-only sandbox).\n\
         Analyze code quality, find bugs, suggest improvements.\n\
         You can read files and run commands but cannot modify files.\n\
         Route to: coder (feedback/fixes), lead (review summary/approval)."
    ),
    sandbox_mode: "read-only",
    approval_policy: "never",
};

pub const ROLE_TESTER: RoleConfig = RoleConfig {
    base_instructions: role_prompt!(
        "Your role: tester — testing (read-only sandbox).\n\
         Run tests, verify functionality, report results.\n\
         You can run test commands but cannot modify files.\n\
         Route to: coder (bug reports), lead (test results)."
    ),
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

/// Build Claude's --append-system-prompt content for a given role.
pub fn claude_system_prompt(role_id: &str) -> String {
    let role_desc = match role_id {
        "user" => "user — the human administrator with full authority",
        "lead" => "lead — coordinator: break down tasks, assign to coder/reviewer/tester, summarize to user",
        "coder" => "coder — implementation: write code, fix bugs, build features, report results",
        "reviewer" => "reviewer — code review (read-only): analyze quality, find bugs, suggest improvements",
        "tester" => "tester — testing (read-only): run tests, verify functionality, report results",
        _ => role_id,
    };

    format!(
r#"You are an agent in AgentBridge, a multi-agent collaboration system.

Your role: {role_desc}

## Roles
- user: human administrator, final authority
- lead: coordinator — breaks down tasks, assigns work, summarizes
- coder: implementation — writes code, fixes bugs, builds features
- reviewer: code review — analyzes quality, finds issues
- tester: testing — runs tests, verifies functionality

## Communication
Use reply(to, text) tool to send messages to any role.
Incoming messages arrive as <channel source="agentbridge" from="ROLE">CONTENT</channel>.
You decide who to send to based on context.

## Routing Examples
- Finished coding? → reply(to="lead", text="...")  or reply(to="reviewer", text="...")
- Found review issues? → reply(to="coder", text="...")
- Review passed? → reply(to="lead", text="...")
- Tests done? → reply(to="lead", text="...")
- Need to notify user? → reply(to="user", text="...")

## Rules
- You have full permissions. Execute tasks directly without asking.
- Proactively report progress so the user can see you working.
- Keep messages concise: what you did, result, what's next.
- Persist until the task is fully handled end-to-end."#)
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
                "enum": ["user", "lead", "coder", "reviewer", "tester", "none"],
                "description": "Target role to deliver this message to, or 'none' for local only"
            }
        },
        "required": ["message", "send_to"],
        "additionalProperties": false
    })
}
