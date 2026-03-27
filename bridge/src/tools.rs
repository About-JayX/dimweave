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
        "description": "Send a message to another agent role in AgentNexus. The system routes it automatically.",
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
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reply_schema_uses_to_field() {
        let schema = reply_tool_schema();
        assert!(schema["inputSchema"]["properties"]["to"].is_object());
        assert_eq!(
            schema["inputSchema"]["required"],
            serde_json::json!(["to", "text", "status"])
        );
        // chat_id no longer exists
        assert!(schema["inputSchema"]["properties"]["chat_id"].is_null());
    }

    #[test]
    fn handle_reply_tool() {
        let params = serde_json::json!({
            "name": "reply",
            "arguments": { "to": "lead", "text": "hello", "status": "done" }
        });
        let msg = handle_tool_call(&params, "coder").unwrap().unwrap();
        assert_eq!(msg.to, "lead");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.from, "coder");
        assert_eq!(msg.status.unwrap().as_str(), "done");
    }

    #[test]
    fn handle_reply_defaults_missing_status_to_done() {
        let params = serde_json::json!({
            "name": "reply",
            "arguments": { "to": "lead", "text": "hello" }
        });
        let msg = handle_tool_call(&params, "coder").unwrap().unwrap();
        assert_eq!(msg.status.unwrap().as_str(), "done");
    }

    #[test]
    fn invalid_status_returns_explicit_error() {
        let params = serde_json::json!({
            "name": "reply",
            "arguments": { "to": "lead", "text": "hello", "status": "waiting" }
        });
        let err = handle_tool_call(&params, "coder").unwrap_err();
        assert!(
            err.to_string().contains("Invalid status: \"waiting\""),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn unknown_tool_returns_none() {
        let params = serde_json::json!({ "name": "unknown", "arguments": {} });
        assert!(handle_tool_call(&params, "claude").unwrap().is_none());
    }

    #[test]
    fn invalid_target_rejected() {
        let params = serde_json::json!({
            "name": "reply",
            "arguments": { "to": "admin", "text": "hello", "status": "done" }
        });
        assert!(handle_tool_call(&params, "coder").unwrap().is_none());
    }

    #[test]
    fn empty_reply_text_rejected() {
        let params = serde_json::json!({
            "name": "reply",
            "arguments": { "to": "lead", "text": "", "status": "done" }
        });
        assert!(handle_tool_call(&params, "coder").unwrap().is_none());
    }

    #[test]
    fn whitespace_only_reply_text_rejected() {
        let params = serde_json::json!({
            "name": "reply",
            "arguments": { "to": "lead", "text": " \n\t ", "status": "done" }
        });
        assert!(handle_tool_call(&params, "coder").unwrap().is_none());
    }

    #[test]
    fn reply_schema_has_enum_constraint() {
        let schema = reply_tool_schema();
        let to_enum = &schema["inputSchema"]["properties"]["to"]["enum"];
        assert!(to_enum.is_array());
        let targets: Vec<&str> = to_enum.as_array().unwrap()
            .iter().map(|v| v.as_str().unwrap()).collect();
        assert!(targets.contains(&"user"));
        assert!(targets.contains(&"lead"));
        assert!(targets.contains(&"reviewer"));
        assert!(!targets.contains(&"tester"));
        assert!(!targets.contains(&"admin"));
    }
}
