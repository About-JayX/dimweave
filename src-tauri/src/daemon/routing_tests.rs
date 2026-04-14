use super::*;
use crate::daemon::{state::DaemonState, types::BridgeMessage};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn route_to_offline_agent_buffers() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let msg = BridgeMessage::system("hello", "lead");
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Buffered));
    assert_eq!(state.read().await.buffered_messages.len(), 1);
}

#[tokio::test]
async fn route_to_user_returns_to_gui() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let msg = BridgeMessage::system("hello", "user");
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::ToGui));
}

#[tokio::test]
async fn route_to_claude_from_unknown_sender_drops() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    {
        let mut s = state.write().await;
        // Claude must be online for sender gating to apply
        let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
        let epoch = s.begin_claude_sdk_launch("nonce-drop".into());
        s.attach_claude_sdk_ws(epoch, "nonce-drop", tx);
        s.claude_role = "lead".into();
    }
    let msg = BridgeMessage {
        id: "msg-1".into(),
        from: "intruder".into(),
        display_source: None,
        to: "lead".into(),
        content: "hello".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Dropped));
}

#[tokio::test]
async fn format_ndjson_user_message_wraps_channel_payload() {
    let msg = BridgeMessage {
        id: "msg-1".into(),
        from: "coder".into(),
        display_source: None,
        to: "lead".into(),
        content: "finished".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,    };

    let ndjson = format_ndjson_user_message(&msg).await;
    let parsed: serde_json::Value = serde_json::from_str(ndjson.trim()).unwrap();

    assert_eq!(
        parsed["message"]["content"][0]["text"],
        "<channel source=\"agentnexus\" from=\"coder\">finished</channel>"
    );
}
