use crate::daemon::{
    types::{BridgeMessage, MessageStatus},
    SharedState,
};

pub(super) async fn begin_sdk_direct_text_turn_if_allowed(state: &SharedState) -> bool {
    state.write().await.begin_claude_sdk_direct_text_turn()
}

pub(super) async fn claim_sdk_terminal_delivery(state: &SharedState) -> bool {
    state.write().await.claim_claude_sdk_terminal_delivery()
}

pub(super) async fn finish_sdk_direct_text_turn(state: &SharedState) {
    state.write().await.finish_claude_sdk_direct_text_turn();
}

pub(super) fn build_direct_sdk_gui_message(
    role: &str,
    text: &str,
    status: MessageStatus,
    agent_id: &str,
) -> Option<BridgeMessage> {
    // Direct SDK fallback only renders terminal text. UI already exposes a
    // single Claude thinking indicator, so surfacing partial assistant chunks
    // here would reintroduce the duplicate/preview noise we removed.
    if !status.is_terminal() || text.is_empty() {
        return None;
    }
    let prefix = match status {
        MessageStatus::Done => "claude_sdk_result",
        MessageStatus::Error => "claude_sdk_error",
        MessageStatus::InProgress => "claude_sdk",
    };
    Some(BridgeMessage {
        id: format!("{prefix}_{}", chrono::Utc::now().timestamp_millis()),
        from: role.to_string(),
        display_source: Some("claude".to_string()),
        to: "user".to_string(),
        content: text.to_string(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: Some(status),
        task_id: None,
        session_id: None,
        sender_agent_id: Some(agent_id.to_string()),
        attachments: None,
    })
}
