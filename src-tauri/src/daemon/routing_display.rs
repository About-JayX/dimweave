use crate::daemon::{
    gui::{self, ClaudeStreamPayload},
    routing::RouteResult,
    types::BridgeMessage,
};
use tauri::AppHandle;

pub(crate) fn emit_route_side_effects(
    app: &AppHandle,
    msg: &BridgeMessage,
    result: &RouteResult,
    buffer_reason: Option<&'static str>,
    emit_claude_thinking: bool,
    display_in_gui: bool,
) {
    if emit_claude_thinking {
        gui::emit_claude_stream(app, ClaudeStreamPayload::ThinkingStarted);
    }
    if display_in_gui && !matches!(result, RouteResult::Dropped) && is_renderable_message(msg) {
        gui::emit_agent_message(app, msg);
    }

    let tag = match result {
        RouteResult::Delivered => "delivered",
        RouteResult::Buffered => "buffered",
        RouteResult::Dropped => "dropped",
        RouteResult::ToGui => "gui",
    };
    eprintln!("[Route] {} → {} {tag}", msg.from, msg.to);

    match result {
        RouteResult::Delivered => {
            gui::emit_system_log(
                app,
                "info",
                &format!("[Route] {} → {} delivered", msg.from, msg.to),
            );
        }
        RouteResult::Buffered => {
            gui::emit_system_log(
                app,
                "warn",
                &buffered_route_message(&msg.to, buffer_reason),
            );
        }
        RouteResult::Dropped => {
            let reason = if !crate::daemon::is_valid_agent_role(&msg.to) && msg.to != "user" {
                format!("[Route] dropped invalid target '{}'", msg.to)
            } else {
                format!(
                    "[Route] dropped unauthorized sender '{}' → '{}'",
                    msg.from, msg.to
                )
            };
            gui::emit_system_log(app, "warn", &reason);
        }
        RouteResult::ToGui => {}
    }
}

pub(crate) fn buffered_route_message(to: &str, buffer_reason: Option<&'static str>) -> String {
    match buffer_reason {
        Some("target_session_missing") => {
            format!("[Route] {to} has no bound session in the active task, buffered")
        }
        Some("task_session_mismatch") => {
            format!("[Route] {to} does not match the active task session, buffered")
        }
        _ => format!("[Route] {to} offline, buffered"),
    }
}

pub(crate) fn is_renderable_message(msg: &BridgeMessage) -> bool {
    !msg.content.trim().is_empty() || msg.attachments.as_ref().is_some_and(|atts| !atts.is_empty())
}

/// Pre-route check: is this message targeting Claude and eligible for thinking indicator?
pub(crate) fn should_emit_claude_thinking_pre(msg: &BridgeMessage, claude_role: &str) -> bool {
    msg.to == claude_role && msg.from != claude_role && is_renderable_message(msg)
}

#[cfg(test)]
pub(crate) fn should_emit_claude_thinking(
    msg: &BridgeMessage,
    result: &RouteResult,
    claude_role: &str,
) -> bool {
    matches!(result, RouteResult::Delivered) && should_emit_claude_thinking_pre(msg, claude_role)
}

#[cfg(test)]
mod tests {
    use super::buffered_route_message;

    #[test]
    fn buffered_route_message_no_longer_mentions_review_gate() {
        let msg = buffered_route_message("coder", Some("review_gate"));
        assert_eq!(msg, "[Route] coder offline, buffered");
    }
}
