use crate::types::PermissionRequest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RpcId {
    Number(i64),
    Str(String),
}

#[derive(Debug, Deserialize)]
pub struct RpcMessage {
    pub id: Option<RpcId>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
}

pub fn id_to_value(id: &Option<RpcId>) -> serde_json::Value {
    match id {
        Some(RpcId::Number(n)) => serde_json::json!(n),
        Some(RpcId::Str(s)) => serde_json::json!(s),
        None => serde_json::Value::Null,
    }
}

pub fn initialize_result(role: &str) -> serde_json::Value {
    serde_json::json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {},
            "experimental": {
                "claude/channel": {},
                "claude/channel/permission": {}
            }
        },
        "instructions": format!("{}\n\nYour role: {role}", CHANNEL_INSTRUCTIONS),
        "serverInfo": { "name": "agentnexus", "version": "0.1.0" }
    })
}

#[cfg(test)]
pub fn channel_notification(content: &str, from: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/claude/channel",
        "params": {
            "content": content,
            "meta": { "from": from }
        }
    })
}

pub fn parse_permission_request(params: &serde_json::Value) -> Option<PermissionRequest> {
    Some(PermissionRequest {
        request_id: params.get("request_id")?.as_str()?.to_string(),
        tool_name: params.get("tool_name")?.as_str()?.to_string(),
        description: params.get("description")?.as_str()?.to_string(),
        input_preview: params
            .get("input_preview")
            .and_then(|value| value.as_str().map(str::to_string)),
    })
}

const CHANNEL_INSTRUCTIONS: &str =
    "You are an agent in AgentNexus, a multi-agent collaboration system.\n\n\
## Communication\n\
Use reply(to, text, status) tool to send messages to any role.\n\
Incoming messages arrive as <channel source=\"agentnexus\" from=\"ROLE\">CONTENT</channel>.\n\
When available, incoming messages may also include status=\"in_progress|done|error\" on the <channel> tag.\n\
You decide who to send to based on context.\n\n\
- status must be one of: in_progress, done, error\n\
- Use status=\"in_progress\" for partial progress updates that are not final\n\
- Use status=\"done\" when your work for this reply is complete\n\
- Use status=\"error\" when reporting a failure or blocking error\n\n\
## Roles\n\
- user: human administrator, final authority\n\
- lead: coordinator — breaks down tasks, assigns work, summarizes\n\
- coder: implementation — writes code, fixes bugs, builds features\n\
- reviewer: review + test verification — analyzes quality, finds issues, runs tests, verifies functionality\n\n\
## Routing Policy\n\
- If your role is lead, you may reply to user or delegate to any worker role when appropriate.\n\
- If your role is NOT lead, lead is your default recipient.\n\
- For messages from user, you may reply directly to user only when the user explicitly names your role or explicitly asks your role to answer.\n\
- If that explicit role mention is absent and you are not lead, send updates, results, blockers, and questions to lead.\n\
- Route directly to another non-lead role only when the current instruction explicitly names that target role. Otherwise route to lead.\n\n\
## Routing Examples\n\
- User says \"fix this bug\" and you are not lead → reply(to=\"lead\", text=\"...\", status=\"done\")\n\
- User says \"coder reply to me directly\" and you are coder → reply(to=\"user\", text=\"...\", status=\"done\")\n\
- Lead explicitly asks you to send work to reviewer → reply(to=\"reviewer\", text=\"...\", status=\"done\")\n\
- Found review issues? → reply(to=\"coder\", text=\"...\", status=\"error\")\n\
- Review passed? → reply(to=\"lead\", text=\"...\", status=\"done\")\n\
- Tests done? → reply(to=\"lead\", text=\"...\", status=\"done\")\n\
- Lead summarizing to user? → reply(to=\"user\", text=\"...\", status=\"done\")\n\n\
## Rules\n\
- You have full permissions. Execute tasks directly without asking.\n\
- Keep messages concise: what you did, result, what's next.\n\
- Persist until the task is fully handled end-to-end.\n\n\
## When to Respond — CRITICAL\n\
Messages from the user may be sent to you directly OR broadcast to all agents (auto mode).\n\
- If the user addresses your role by name or describes a task in your domain → respond.\n\
- If the message does not mention your role and is not in your domain → do NOT respond. \
Do NOT call the reply tool at all. Stay completely silent.\n\
- If the user explicitly says \"only X role respond\" or \"X回答我\" and X is NOT your role → \
you MUST stay silent. Do NOT call reply(). Do NOT output any message. This is absolute.\n\
- Exception: if the user's statement contains a significant factual error in your expertise, \
correct it even if not directly addressed.\n\
- When in doubt about whether to respond, DO NOT respond. Silence is always safer than an unwanted reply.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_initialize_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"claude-code","version":"1.0"}}}"#;
        let msg: RpcMessage = serde_json::from_str(raw).unwrap();
        assert_eq!(msg.method.as_deref(), Some("initialize"));
        assert!(matches!(msg.id, Some(RpcId::Number(1))));
    }

    #[test]
    fn parse_tools_list_request() {
        let raw = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#;
        let msg: RpcMessage = serde_json::from_str(raw).unwrap();
        assert_eq!(msg.method.as_deref(), Some("tools/list"));
    }

    #[test]
    fn initialize_result_includes_instructions_and_permission_capability() {
        let result = initialize_result("lead");
        assert_eq!(
            result["capabilities"]["experimental"]["claude/channel"],
            serde_json::json!({})
        );
        assert_eq!(
            result["capabilities"]["experimental"]["claude/channel/permission"],
            serde_json::json!({})
        );
        assert!(result["instructions"]
            .as_str()
            .unwrap_or_default()
            .contains("<channel source=\"agentnexus\""));
    }

    #[test]
    fn initialize_result_includes_silence_rules() {
        let result = initialize_result("coder");
        let instructions = result["instructions"].as_str().unwrap_or_default();
        assert!(
            instructions.contains("Stay completely silent"),
            "channel instructions must include strict silence rule"
        );
        assert!(
            instructions.contains("Do NOT call reply()"),
            "channel instructions must prohibit reply() for non-addressed messages"
        );
        assert!(
            !instructions.contains("Proactively report progress"),
            "channel instructions must NOT contain loose 'proactively report' directive"
        );
    }

    #[test]
    fn initialize_result_mentions_reply_status_contract() {
        let result = initialize_result("lead");
        let instructions = result["instructions"].as_str().unwrap_or_default();
        assert!(instructions.contains("reply(to, text, status)"));
        assert!(instructions.contains("in_progress"));
        assert!(instructions.contains("done"));
        assert!(instructions.contains("error"));
        assert!(instructions.contains("lead is your default recipient"));
    }

    #[test]
    fn serialize_channel_notification() {
        let n = channel_notification("hello", "coder");
        let s = serde_json::to_string(&n).unwrap();
        assert!(s.contains("notifications/claude/channel"));
        assert!(s.contains("hello"));
        assert!(s.contains("coder"));
    }
}
