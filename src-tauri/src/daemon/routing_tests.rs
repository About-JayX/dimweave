use super::*;
use crate::daemon::{
    state::DaemonState,
    task_graph::types::Provider,
    types::{BridgeMessage, MessageSource, MessageTarget},
};
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
        source: MessageSource::Agent {
            agent_id: "intruder".into(),
            role: "intruder".into(),
            provider: Provider::Claude,
            display_source: None,
        },
        target: MessageTarget::Role { role: "lead".into() },
        reply_target: None,
        message: "hello".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        attachments: None,
    };
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
        source: MessageSource::User,
        target: MessageTarget::Agent { agent_id: "claude".into() },
        reply_target: None,
        message: "direct to claude".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
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
        source: MessageSource::User,
        target: MessageTarget::Agent { agent_id: "codex".into() },
        reply_target: None,
        message: "direct to codex".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
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
        source: MessageSource::User,
        target: MessageTarget::Agent { agent_id: "claude".into() },
        reply_target: None,
        message: "buffered".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        attachments: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Buffered));
    assert_eq!(state.read().await.buffered_messages.len(), 1);
    assert_eq!(state.read().await.buffered_messages[0].target_str(), "claude");
}

#[tokio::test]
async fn format_ndjson_user_message_wraps_channel_payload() {
    let msg = BridgeMessage {
        id: "msg-1".into(),
        source: MessageSource::Agent {
            agent_id: "codex".into(),
            role: "coder".into(),
            provider: Provider::Codex,
            display_source: None,
        },
        target: MessageTarget::Role { role: "lead".into() },
        reply_target: None,
        message: "finished".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        attachments: None,
    };

    let ndjson = format_ndjson_user_message(&msg).await;
    let parsed: serde_json::Value = serde_json::from_str(ndjson.trim()).unwrap();

    // After Step 7: channel now includes `sender_agent_id` when source is an
    // Agent variant. task_id is absent here because the fixture sets task_id=None.
    assert_eq!(
        parsed["message"]["content"][0]["text"],
        "<channel source=\"agentnexus\" from=\"coder\" sender_agent_id=\"codex\">finished</channel>"
    );
}

#[tokio::test]
async fn format_ndjson_user_message_includes_task_id_when_present() {
    use crate::daemon::task_graph::types::Provider;
    let msg = BridgeMessage {
        id: "m".into(),
        source: MessageSource::Agent {
            agent_id: "codex".into(),
            role: "coder".into(),
            provider: Provider::Codex,
            display_source: None,
        },
        target: MessageTarget::Role { role: "lead".into() },
        reply_target: None,
        message: "done".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: Some("task_42".into()),
        session_id: None,
        attachments: None,
    };
    let ndjson = format_ndjson_user_message(&msg).await;
    let parsed: serde_json::Value = serde_json::from_str(ndjson.trim()).unwrap();
    let channel = parsed["message"]["content"][0]["text"]
        .as_str()
        .expect("channel text");
    assert!(
        channel.contains("sender_agent_id=\"codex\""),
        "missing sender_agent_id in: {channel}"
    );
    assert!(
        channel.contains("task_id=\"task_42\""),
        "missing task_id in: {channel}"
    );
}

#[tokio::test]
async fn format_ndjson_user_message_omits_sender_agent_id_for_user_source() {
    let msg = BridgeMessage {
        id: "m".into(),
        source: MessageSource::User,
        target: MessageTarget::Role { role: "coder".into() },
        reply_target: None,
        message: "do this".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        attachments: None,
    };
    let ndjson = format_ndjson_user_message(&msg).await;
    let parsed: serde_json::Value = serde_json::from_str(ndjson.trim()).unwrap();
    let channel = parsed["message"]["content"][0]["text"]
        .as_str()
        .expect("channel text");
    assert!(
        !channel.contains("sender_agent_id"),
        "user-source channel must not carry sender_agent_id: {channel}"
    );
}
