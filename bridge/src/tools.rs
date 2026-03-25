use crate::types::BridgeMessage;

pub fn reply_tool_schema() -> serde_json::Value {
    serde_json::json!({
        "name": "reply",
        "description": "Send a message to another agent or the user.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "to": {
                    "type": "string",
                    "description": "Target role: lead/coder/reviewer/tester/user"
                },
                "text": {
                    "type": "string",
                    "description": "Message content"
                }
            },
            "required": ["to", "text"]
        }
    })
}

pub fn handle_tool_call(params: &serde_json::Value, from: &str) -> Option<BridgeMessage> {
    let name = params.get("name")?.as_str()?;
    if name != "reply" {
        return None;
    }
    let args = params.get("arguments")?;
    let to = args.get("to")?.as_str()?;
    let text = args.get("text")?.as_str()?;
    Some(BridgeMessage {
        id: format!("claude_{}", chrono::Utc::now().timestamp_millis()),
        from: from.to_string(),
        to: to.to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_reply_tool() {
        let params = serde_json::json!({
            "name": "reply",
            "arguments": { "to": "lead", "text": "hello" }
        });
        let msg = handle_tool_call(&params, "coder").unwrap();
        assert_eq!(msg.to, "lead");
        assert_eq!(msg.content, "hello");
        assert_eq!(msg.from, "coder");
    }

    #[test]
    fn unknown_tool_returns_none() {
        let params = serde_json::json!({ "name": "unknown", "arguments": {} });
        assert!(handle_tool_call(&params, "claude").is_none());
    }
}
