use crate::daemon::types::{BridgeMessage, MessageStatus};

const TELEGRAM_MAX_LENGTH: usize = 4096;

/// Only terminal `lead -> user` messages trigger Telegram reports.
pub fn should_send_lead_report(msg: &BridgeMessage) -> bool {
    msg.from == "lead"
        && msg.to == "user"
        && matches!(
            msg.status,
            Some(MessageStatus::Done) | Some(MessageStatus::Error)
        )
}

/// Build a plain-text lead report.
pub fn build_lead_report(task_title: Option<&str>, msg: &BridgeMessage) -> String {
    let status_str = msg.status.as_ref().map(|s| s.as_str()).unwrap_or("done");
    format!(
        "Dimweave update\nTask: {}\nStatus: {}\n\n{}",
        task_title.unwrap_or("No active task"),
        status_str,
        msg.content.trim(),
    )
}

/// Chunk text to fit Telegram's message length limit.
pub fn chunk_report(text: &str) -> Vec<String> {
    if text.len() <= TELEGRAM_MAX_LENGTH {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let end = remaining
            .char_indices()
            .take_while(|(i, _)| *i < TELEGRAM_MAX_LENGTH)
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(remaining.len());
        chunks.push(remaining[..end].to_string());
        remaining = &remaining[end..];
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_message(from: &str, to: &str, status: Option<MessageStatus>) -> BridgeMessage {
        BridgeMessage {
            id: "test".into(),
            from: from.into(),
            display_source: None,
            to: to.into(),
            content: "result text".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status,
            task_id: None,
            session_id: None,
            sender_agent_id: None,
            attachments: None, report_telegram: None,
        }
    }

    #[test]
    fn only_terminal_lead_to_user_triggers() {
        assert!(should_send_lead_report(&test_message(
            "lead",
            "user",
            Some(MessageStatus::Done)
        )));
        assert!(should_send_lead_report(&test_message(
            "lead",
            "user",
            Some(MessageStatus::Error)
        )));
        assert!(!should_send_lead_report(&test_message(
            "lead",
            "user",
            Some(MessageStatus::InProgress)
        )));
        assert!(!should_send_lead_report(&test_message(
            "coder",
            "user",
            Some(MessageStatus::Done)
        )));
        assert!(!should_send_lead_report(&test_message(
            "lead",
            "coder",
            Some(MessageStatus::Done)
        )));
    }

    #[test]
    fn chunk_short_text_returns_single() {
        let parts = chunk_report("hello");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], "hello");
    }

    #[test]
    fn chunk_long_text_splits_correctly() {
        let long = "x".repeat(5000);
        let parts = chunk_report(&long);
        assert!(parts.len() >= 2);
        assert!(parts.iter().all(|p| p.len() <= TELEGRAM_MAX_LENGTH));
        let rejoined: String = parts.into_iter().collect();
        assert_eq!(rejoined, long);
    }
}
