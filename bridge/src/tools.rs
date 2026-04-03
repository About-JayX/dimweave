use crate::types::{BridgeMessage, MessageStatus};
use std::sync::atomic::{AtomicU64, Ordering};

static MSG_SEQ: AtomicU64 = AtomicU64::new(0);

const VALID_REPLY_TARGETS: &[&str] = &["user", "lead", "coder", "reviewer"];
const VALID_REPLY_STATUSES: &[&str] = &["in_progress", "done", "error"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallError {
    InvalidStatus(String),
}

impl std::fmt::Display for ToolCallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidStatus(value) => write!(
                f,
                "Invalid status: \"{value}\". Expected \"in_progress\", \"done\", or \"error\"."
            ),
        }
    }
}

pub fn reply_tool_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "reply",
        "description": "Send a message to another agent role in Dimweave. The system routes it automatically.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "to": {
                    "type": "string",
                    "enum": VALID_REPLY_TARGETS,
                    "description": "Target role: user, lead, coder, or reviewer"
                },
                "text": {
                    "type": "string",
                    "description": "Message content"
                },
                "status": {
                    "type": "string",
                    "enum": VALID_REPLY_STATUSES,
                    "description": "Message lifecycle status"
                }
            },
            "required": ["to", "text", "status"]
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
    from: &str,
) -> Result<Option<BridgeMessage>, ToolCallError> {
    let Some(name) = params.get("name").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    if name != "reply" {
        return Ok(None);
    }
    let Some(args) = params.get("arguments") else {
        return Ok(None);
    };
    let Some(to) = args.get("to").and_then(|value| value.as_str()) else {
        return Ok(None);
    };
    if !VALID_REPLY_TARGETS.contains(&to) {
        return Ok(None);
    }
    let Some(text) = args.get("text").and_then(|value| value.as_str()) else {
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
    let seq = MSG_SEQ.fetch_add(1, Ordering::Relaxed);
    Ok(Some(BridgeMessage {
        id: format!("claude_{}_{seq}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        display_source: None,
        to: to.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(status),
        sender_agent_id: None,
        attachments: None,
    }))
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tests;
