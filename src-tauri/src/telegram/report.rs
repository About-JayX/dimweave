use crate::daemon::types::{BridgeMessage, MessageStatus};
#[cfg(test)]
use crate::daemon::types::{MessageSource, MessageTarget};

const TELEGRAM_MAX_LENGTH: usize = 4096;

/// All lead messages trigger Telegram reports.
/// Runtime gates (enabled flag, outbound tx, paired chat) live in `routing_dispatch.rs`.
pub fn should_send_telegram_report(msg: &BridgeMessage) -> bool {
    msg.source_role() == "lead"
}

/// Escape dynamic text for safe embedding in Telegram HTML messages.
pub fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn event_emoji(status: &MessageStatus) -> &'static str {
    match status {
        MessageStatus::Done => "\u{2705}",
        MessageStatus::Error => "\u{1f6a8}",
        MessageStatus::InProgress => "\u{23f3}",
    }
}

/// Build an HTML-formatted Telegram report card.
pub fn build_telegram_report(task_title: Option<&str>, msg: &BridgeMessage) -> String {
    let status = msg.status.as_ref().copied().unwrap_or(MessageStatus::Done);
    let emoji = event_emoji(&status);
    let task_id = msg.task_id.as_deref().unwrap_or("—");
    let title = task_title.unwrap_or("No active task");
    let body = escape_html(msg.message.trim());
    format!(
        "{emoji} <b>Dimweave</b> · {status}\n\
         \u{1f4cb} <b>{title_escaped}</b>\n\
         <code>{task_id}</code>\n\n\
         {body}",
        status = escape_html(status.as_str()),
        title_escaped = escape_html(title),
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
        let source = match from {
            "user" => MessageSource::User,
            "system" => MessageSource::System,
            _ => MessageSource::Agent {
                agent_id: from.into(),
                role: from.into(),
                provider: crate::daemon::task_graph::types::Provider::Claude,
                display_source: None,
            },
        };
        let target = if to == "user" {
            MessageTarget::User
        } else {
            MessageTarget::Role { role: to.into() }
        };
        BridgeMessage {
            id: "test".into(),
            source,
            target,
            reply_target: None,
            message: "result text".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status,
            task_id: None,
            session_id: None,
            attachments: None,
        }
    }

    #[test]
    fn lead_message_triggers_regardless_of_status() {
        assert!(should_send_telegram_report(&test_message(
            "lead", "coder", Some(MessageStatus::Done)
        )));
        assert!(should_send_telegram_report(&test_message(
            "lead", "user", Some(MessageStatus::Error)
        )));
        assert!(should_send_telegram_report(&test_message(
            "lead", "user", Some(MessageStatus::InProgress)
        )));
        assert!(should_send_telegram_report(&test_message(
            "lead", "user", None
        )));
    }

    #[test]
    fn non_lead_does_not_trigger() {
        assert!(!should_send_telegram_report(&test_message(
            "coder", "lead", Some(MessageStatus::Done)
        )));
        assert!(!should_send_telegram_report(&test_message(
            "user", "lead", None
        )));
    }

    #[test]
    fn html_formatter_escapes_dynamic_text() {
        let formatted = escape_html(r#"<tag> & "quote""#);
        assert_eq!(formatted, "&lt;tag&gt; &amp; &quot;quote&quot;");
    }

    #[test]
    fn html_report_includes_task_id_and_status() {
        let mut msg = test_message("lead", "user", Some(MessageStatus::Done));
        msg.task_id = Some("task_42".into());
        let report = build_telegram_report(Some("Fix login bug"), &msg);
        assert!(report.contains("task_42"));
        assert!(report.contains("Fix login bug"));
        assert!(report.contains("done"));
    }

    #[test]
    fn html_report_escapes_content() {
        let mut msg = test_message("lead", "user", Some(MessageStatus::Done));
        msg.message = "<script>alert('xss')</script>".into();
        let report = build_telegram_report(None, &msg);
        assert!(!report.contains("<script>"));
        assert!(report.contains("&lt;script&gt;"));
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
