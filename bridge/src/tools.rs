use crate::types::{MessageStatus, MessageTarget, ParsedReply};

const VALID_REPLY_STATUSES: &[&str] = &["in_progress", "done", "error"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallError {
    InvalidStatus(String),
    InvalidTarget(String),
}

impl std::fmt::Display for ToolCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStatus(value) => write!(
                f,
                "Invalid status: \"{value}\". Expected \"in_progress\", \"done\", or \"error\"."
            ),
            Self::InvalidTarget(msg) => write!(f, "Invalid target: {msg}"),
        }
    }
}

pub fn reply_tool_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "reply",
        "description": "Send a message to another role or agent in Dimweave. The system routes it automatically.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "target": {
                    "type": "object",
                    "description": "Message target (flat 3-field form — all keys required, unused keys filled with empty strings).",
                    "additionalProperties": false,
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["user", "role", "agent"],
                            "description": "Target type"
                        },
                        "role": {
                            "type": "string",
                            "description": "Role name when kind='role'; empty string otherwise"
                        },
                        "agentId": {
                            "type": "string",
                            "description": "Agent id when kind='agent'; empty string otherwise"
                        }
                    },
                    "required": ["kind", "role", "agentId"]
                },
                "message": {
                    "type": "string",
                    "description": "Message content"
                },
                "status": {
                    "type": "string",
                    "enum": VALID_REPLY_STATUSES,
                    "description": "Message lifecycle status"
                }
            },
            "required": ["target", "message", "status"]
        }
    })
}

pub fn get_online_agents_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "get_online_agents",
        "description": "Query which agents are currently online in Dimweave and their roles.",
        "inputSchema": { "type": "object", "properties": {} }
    })
}

pub fn tool_list() -> Vec<serde_json::Value> {
    vec![reply_tool_schema(), get_online_agents_schema()]
}

pub fn is_get_online_agents(params: &serde_json::Value) -> bool {
    params.get("name").and_then(|v| v.as_str()) == Some("get_online_agents")
}

pub fn handle_tool_call(
    params: &serde_json::Value,
) -> Result<Option<ParsedReply>, ToolCallError> {
    let Some(name) = params.get("name").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    if name != "reply" {
        return Ok(None);
    }
    let Some(args) = params.get("arguments") else {
        return Ok(None);
    };
    let target = parse_target(args)?;
    let Some(text) = args.get("message").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    if text.trim().is_empty() {
        return Ok(None);
    }
    let status = match args.get("status") {
        Some(value) => {
            let raw = value.as_str().unwrap_or_default();
            MessageStatus::parse(raw).ok_or_else(|| {
                ToolCallError::InvalidStatus(if raw.is_empty() {
                    value.to_string()
                } else {
                    raw.to_string()
                })
            })?
        }
        None => MessageStatus::Done,
    };
    Ok(Some(ParsedReply {
        target,
        message: text.to_string(),
        status,
    }))
}

fn parse_target(args: &serde_json::Value) -> Result<MessageTarget, ToolCallError> {
    // Legacy `{to: ..., text: ...}` → help the model self-correct with a
    // precise error rather than a generic "missing target.kind".
    if args.get("to").is_some() && args.get("target").is_none() {
        return Err(ToolCallError::InvalidTarget(
            "legacy 'to' field detected; reply tool now requires structured \
             `target: {kind, role, agentId}` object (see tool schema)".into(),
        ));
    }
    let obj = args
        .get("target")
        .ok_or_else(|| ToolCallError::InvalidTarget("missing target".into()))?;
    let kind = obj
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolCallError::InvalidTarget("missing target.kind".into()))?;
    match kind {
        "user" => Ok(MessageTarget::User),
        "role" => {
            let role = obj.get("role").and_then(|v| v.as_str()).ok_or_else(|| {
                ToolCallError::InvalidTarget("kind=role requires role field".into())
            })?;
            if role.trim().is_empty() {
                return Err(ToolCallError::InvalidTarget("role must not be empty".into()));
            }
            Ok(MessageTarget::Role { role: role.into() })
        }
        "agent" => {
            let id = obj
                .get("agentId")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ToolCallError::InvalidTarget("kind=agent requires agentId field".into())
                })?;
            if id.trim().is_empty() {
                return Err(ToolCallError::InvalidTarget(
                    "agentId must not be empty".into(),
                ));
            }
            Ok(MessageTarget::Agent {
                agent_id: id.into(),
            })
        }
        other => Err(ToolCallError::InvalidTarget(format!(
            "unknown kind: \"{other}\""
        ))),
    }
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
