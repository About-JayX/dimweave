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

// ── agent-targeted routing tests ─────────────────────────────────

#[tokio::test]
async fn agent_targeted_to_claude_delivers_when_online() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, mut claude_rx) = tokio::sync::mpsc::channel::<String>(8);
    {
        let mut s = state.write().await;
        let epoch = s.begin_claude_sdk_launch("nonce-agent-target".into());
        s.attach_claude_sdk_ws(epoch, "nonce-agent-target", claude_tx);
        s.claude_role = "lead".into();
    }
    // Target "claude" by agent_id, not by role "lead"
    let msg = BridgeMessage {
        id: "agent-target-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "claude".into(),
        content: "direct to claude".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Delivered));
    assert!(claude_rx.try_recv().is_ok());
}

#[tokio::test]
async fn agent_targeted_to_codex_delivers_when_online() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (codex_tx, mut codex_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        s.codex_role = "coder".into();
        s.codex_inject_tx = Some(codex_tx);
    }
    let msg = BridgeMessage {
        id: "agent-target-2".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "codex".into(),
        content: "direct to codex".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Delivered));
    assert!(codex_rx.try_recv().is_ok());
}

#[tokio::test]
async fn agent_targeted_to_offline_agent_buffers() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    {
        let mut s = state.write().await;
        s.claude_role = "lead".into();
        // Claude NOT online
    }
    let msg = BridgeMessage {
        id: "agent-target-3".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "claude".into(),
        content: "buffered".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Buffered));
    assert_eq!(state.read().await.buffered_messages.len(), 1);
    assert_eq!(state.read().await.buffered_messages[0].to, "claude");
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
