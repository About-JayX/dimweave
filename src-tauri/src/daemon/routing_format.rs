use crate::daemon::types::{Attachment, BridgeMessage};

/// Append non-image file paths as text context.
fn append_file_attachment_context(base: &mut String, atts: &[Attachment]) {
    let files: Vec<&Attachment> = atts.iter().filter(|a| !a.is_image).collect();
    if files.is_empty() {
        return;
    }
    base.push_str("\n\n[Attached files:");
    for att in files {
        base.push_str(&format!("\n- {}", att.file_path));
    }
    base.push(']');
}

/// Build base text content for Codex (handles agent vs user formatting).
///
/// Sender agent_id is inlined as `[agent_id]` after the role so a worker
/// can reply with `{kind:"agent", agentId:<sender>}` to the specific
/// delegator, matching the Claude `<channel sender_agent_id=...>` semantics.
///
/// `task_id` is **not** injected into user-input text: each Codex session
/// is already bound to exactly one task at `thread/start`, so prefixing
/// user messages with `[task: <uuid>]` was redundant AND empirically caused
/// the model to emit multiple consecutive `agentMessage` items in a single
/// turn (Codex treated the prefix as an additional directive). For
/// agent-source messages we keep a task label only when the agent comes
/// from a different task context than the receiver's session — in practice
/// that doesn't happen today (sessions are task-scoped), so we drop it.
fn build_codex_text(msg: &BridgeMessage) -> String {
    if msg.is_from_user() {
        return msg.message.clone();
    }
    let sender_label = match msg.source_agent_id() {
        Some(aid) => format!("{} [{}]", msg.source_role(), aid),
        None => msg.source_role().to_string(),
    };
    match msg.status {
        Some(status) => format!(
            "Message from {} (status: {}):\n{}",
            sender_label, status.as_str(), msg.message
        ),
        None => format!("Message from {}:\n{}", sender_label, msg.message),
    }
}

/// Build structured input items for Codex turn/start.
/// Images → `{"type":"localImage","path":"..."}`, files → text inline paths.
pub fn build_codex_input_items(msg: &BridgeMessage) -> Vec<serde_json::Value> {
    let mut text = build_codex_text(msg);
    if let Some(atts) = &msg.attachments {
        append_file_attachment_context(&mut text, atts);
    }
    let mut items = vec![serde_json::json!({"type": "text", "text": text})];
    if let Some(atts) = &msg.attachments {
        for att in atts.iter().filter(|a| a.is_image) {
            items.push(serde_json::json!({"type": "localImage", "path": att.file_path}));
        }
    }
    items
}

/// Format NDJSON user message for Claude SDK, with image compression.
/// Images are base64 encoded after resize; non-image files are text paths.
pub async fn format_ndjson_user_message(msg: &BridgeMessage) -> String {
    let mut text = msg.message.clone();
    if let Some(atts) = &msg.attachments {
        append_file_attachment_context(&mut text, atts);
    }
    let wrapped = crate::daemon::claude_sdk::protocol::wrap_channel_content(
        msg.source_role(),
        &text,
        msg.source_agent_id(),
        msg.task_id.as_deref(),
    );
    let mut blocks = vec![serde_json::json!({"type": "text", "text": wrapped})];
    if let Some(atts) = &msg.attachments {
        for att in atts.iter().filter(|a| a.is_image) {
            match crate::daemon::image_compress::compress_for_claude(&att.file_path).await {
                Ok(img) => {
                    blocks.push(serde_json::json!({
                        "type": "image",
                        "source": {
                            "type": "base64",
                            "media_type": img.media_type,
                            "data": img.base64,
                        }
                    }));
                }
                Err(e) => eprintln!("[Route] image compress failed for {}: {e}", att.file_path),
            }
        }
    }
    crate::daemon::claude_sdk::protocol::format_user_message_with_content(&blocks)
}

/// Legacy sync version for non-image messages (used by text-only callers).
pub fn format_codex_input(msg: &BridgeMessage) -> String {
    build_codex_text(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::types::MessageSource;

    fn file_att() -> Attachment {
        Attachment {
            file_path: "/tmp/foo.rs".into(),
            file_name: "foo.rs".into(),
            is_image: false,
            media_type: None,
        }
    }

    fn image_att() -> Attachment {
        Attachment {
            file_path: "/tmp/pic.png".into(),
            file_name: "pic.png".into(),
            is_image: true,
            media_type: Some("image/png".into()),
        }
    }

    #[test]
    fn codex_items_text_only() {
        let mut msg = BridgeMessage::system("hello", "coder");
        msg.source = MessageSource::User;
        let items = build_codex_input_items(&msg);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["type"], "text");
    }

    #[test]
    fn codex_items_with_image_and_file() {
        let mut msg = BridgeMessage::system("analyze", "coder");
        msg.source = MessageSource::User;
        msg.attachments = Some(vec![file_att(), image_att()]);
        let items = build_codex_input_items(&msg);
        assert_eq!(items.len(), 2); // 1 text (with file path) + 1 localImage
        assert_eq!(items[0]["type"], "text");
        assert!(items[0]["text"].as_str().unwrap().contains("/tmp/foo.rs"));
        assert_eq!(items[1]["type"], "localImage");
        assert_eq!(items[1]["path"], "/tmp/pic.png");
    }

    #[test]
    fn codex_items_image_not_in_text() {
        let mut msg = BridgeMessage::system("look", "coder");
        msg.source = MessageSource::User;
        msg.attachments = Some(vec![image_att()]);
        let items = build_codex_input_items(&msg);
        // Image path should NOT appear in text (it's a localImage item)
        assert!(!items[0]["text"].as_str().unwrap().contains("pic.png"));
        assert_eq!(items[1]["type"], "localImage");
    }

    #[test]
    fn no_attachments_unchanged() {
        let mut msg = BridgeMessage::system("hello", "coder");
        msg.source = MessageSource::User;
        let items = build_codex_input_items(&msg);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["text"], "hello");
    }
}
