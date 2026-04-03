use crate::daemon::types::BridgeMessage;

fn append_attachment_context(base: &mut String, msg: &BridgeMessage) {
    if let Some(atts) = &msg.attachments {
        if !atts.is_empty() {
            base.push_str("\n\n[Attached files:");
            for att in atts {
                base.push_str(&format!("\n- {}", att.file_path));
            }
            base.push(']');
        }
    }
}

/// Format a BridgeMessage as NDJSON user message for Claude SDK WS delivery.
/// Uses the verified stream-json protocol format.
/// Wraps content in `<channel>` tags to match agent prompt instructions.
pub fn format_ndjson_user_message(msg: &BridgeMessage) -> String {
    let mut content = msg.content.clone();
    append_attachment_context(&mut content, msg);
    crate::daemon::claude_sdk::protocol::format_channel_user_message(&msg.from, &content)
}

/// Format a BridgeMessage as plain text for Codex turn/start injection.
pub fn format_codex_input(msg: &BridgeMessage) -> String {
    let mut base = if msg.from == "user" {
        msg.content.clone()
    } else {
        let sender_label = match &msg.sender_agent_id {
            Some(aid) => format!("{} [{}]", msg.from, aid),
            None => msg.from.clone(),
        };
        match msg.status {
            Some(status) => format!(
                "Message from {} (status: {}):\n{}",
                sender_label,
                status.as_str(),
                msg.content
            ),
            None => format!("Message from {}:\n{}", sender_label, msg.content),
        }
    };
    append_attachment_context(&mut base, msg);
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::types::{Attachment, MessageStatus};

    #[test]
    fn codex_input_appends_attachment_paths() {
        let msg = BridgeMessage::system("hello", "coder");
        let mut msg = msg;
        msg.from = "user".into();
        msg.attachments = Some(vec![Attachment {
            file_path: "/tmp/foo.rs".into(),
            file_name: "foo.rs".into(),
        }]);
        let out = format_codex_input(&msg);
        assert!(out.contains("[Attached files:"));
        assert!(out.contains("/tmp/foo.rs"));
    }

    #[test]
    fn codex_input_no_attachments_unchanged() {
        let mut msg = BridgeMessage::system("hello", "coder");
        msg.from = "user".into();
        assert_eq!(format_codex_input(&msg), "hello");
    }

    #[test]
    fn ndjson_includes_attachment_paths() {
        let mut msg = BridgeMessage::system("hi", "lead");
        msg.from = "user".into();
        msg.attachments = Some(vec![Attachment {
            file_path: "/tmp/bar.png".into(),
            file_name: "bar.png".into(),
        }]);
        let out = format_ndjson_user_message(&msg);
        assert!(out.contains("/tmp/bar.png"));
    }
}
