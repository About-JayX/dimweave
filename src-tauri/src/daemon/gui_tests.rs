use super::{should_auto_finish_idle_claude_thinking, ClaudeStreamPayload};

#[test]
fn thinking_started_does_not_auto_finish_idle_claude_thinking() {
    assert!(!should_auto_finish_idle_claude_thinking(
        &ClaudeStreamPayload::ThinkingStarted
    ));
}

#[test]
fn preview_does_not_auto_finish_idle_claude_thinking() {
    assert!(!should_auto_finish_idle_claude_thinking(
        &ClaudeStreamPayload::Preview {
            text: "preview".into(),
        }
    ));
}
