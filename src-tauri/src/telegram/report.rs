use crate::daemon::types::{BridgeMessage, MessageStatus};

const TELEGRAM_MAX_LENGTH: usize = 4096;

/// Lead messages with `report_telegram=true` trigger Telegram reports.
/// Runtime gates (enabled flag, outbound tx, paired chat) live in `routing_dispatch.rs`.
pub fn should_send_telegram_report(msg: &BridgeMessage) -> bool {
    msg.from == "lead" && msg.report_telegram == Some(true)
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
    let body = escape_html(msg.content.trim());
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
    fn no_flag_does_not_trigger() {
        // lead + terminal status but no flag → no trigger
        assert!(!should_send_telegram_report(&test_message(
            "lead",
            "user",
            Some(MessageStatus::Done)
        )));
    }

    #[test]
    fn lead_with_flag_triggers_regardless_of_status() {
        // done
        let mut msg = test_message("lead", "coder", Some(MessageStatus::Done));
        msg.report_telegram = Some(true);
        assert!(should_send_telegram_report(&msg));

        // error
        let mut msg = test_message("lead", "user", Some(MessageStatus::Error));
        msg.report_telegram = Some(true);
        assert!(should_send_telegram_report(&msg));

        // in_progress — now triggers too
        let mut msg = test_message("lead", "user", Some(MessageStatus::InProgress));
        msg.report_telegram = Some(true);
        assert!(should_send_telegram_report(&msg));

        // no status — also triggers
        let mut msg = test_message("lead", "user", None);
        msg.report_telegram = Some(true);
        assert!(should_send_telegram_report(&msg));
    }

    #[test]
    fn flag_missing_or_false_does_not_trigger() {
        let mut msg = test_message("lead", "coder", Some(MessageStatus::Done));

        // None → no trigger
        msg.report_telegram = None;
        assert!(!should_send_telegram_report(&msg));

        // false → no trigger
        msg.report_telegram = Some(false);
        assert!(!should_send_telegram_report(&msg));
    }

    #[test]
    fn non_lead_does_not_trigger_even_with_flag() {
        let mut msg = test_message("coder", "lead", Some(MessageStatus::Done));
        msg.report_telegram = Some(true);
        assert!(!should_send_telegram_report(&msg));

        let mut msg = test_message("user", "lead", None);
        msg.report_telegram = Some(true);
        assert!(!should_send_telegram_report(&msg));
    }

    #[test]
    fn html_formatter_escapes_dynamic_text() {
        let formatted = escape_html(r#"<tag> & "quote""#);
        assert_eq!(formatted, "&lt;tag&gt; &amp; &quot;quote&quot;");
    }

    #[test]
    fn html_report_includes_task_id_and_status() {
        let mut msg = test_message("lead", "user", Some(MessageStatus::Done));
        msg.report_telegram = Some(true);
        msg.task_id = Some("task_42".into());
        let report = build_telegram_report(Some("Fix login bug"), &msg);
        assert!(report.contains("task_42"));
        assert!(report.contains("Fix login bug"));
        assert!(report.contains("done"));
    }

    #[test]
    fn html_report_escapes_content() {
        let mut msg = test_message("lead", "user", Some(MessageStatus::Done));
        msg.report_telegram = Some(true);
        msg.content = "<script>alert('xss')</script>".into();
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
