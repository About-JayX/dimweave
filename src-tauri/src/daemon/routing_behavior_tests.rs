use super::*;
use crate::daemon::{state::DaemonState, types::BridgeMessage};
use crate::daemon::routing_display::{is_renderable_message, should_emit_claude_thinking};
use std::sync::Arc;
use tokio::sync::RwLock;

// ── is_valid_agent_role tests ─────────────────────────────────────

#[test]
fn valid_roles_accepted() {
    for role in &["lead", "coder", "reviewer"] {
        assert!(crate::daemon::is_valid_agent_role(role), "{role} should be valid");
    }
}

#[test]
fn user_role_rejected() {
    assert!(!crate::daemon::is_valid_agent_role("user"));
}

#[test]
fn unknown_role_rejected() {
    assert!(!crate::daemon::is_valid_agent_role("admin"));
    assert!(!crate::daemon::is_valid_agent_role(""));
}

// ── fan-out behavior tests (route_message_inner level) ────────────

#[tokio::test]
async fn auto_fanout_delivers_to_both_agents() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, mut claude_rx) = tokio::sync::mpsc::channel(8);
    let (codex_tx, mut codex_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        s.attached_agents.insert(
            "claude".into(),
            crate::daemon::state::AgentSender::new(claude_tx, 0),
        );
        s.codex_inject_tx = Some(codex_tx);
    }
    let targets = {
        let s = state.read().await;
        resolve_user_targets(&s, "auto")
    };
    assert_eq!(targets.len(), 2);
    for role in &targets {
        let msg = BridgeMessage {
            id: format!("test_{role}"),
            from: "user".into(),
            to: role.clone(),
            content: "hello".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status: None,
        };
        let result = route_message_inner(&state, msg).await;
        assert!(matches!(result, RouteResult::Delivered));
    }
    assert!(claude_rx.try_recv().is_ok());
    assert!(codex_rx.try_recv().is_ok());
}

#[tokio::test]
async fn explicit_user_target_routes_to_gui() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let msg = BridgeMessage {
        id: "u1".into(),
        from: "user".into(),
        to: "user".into(),
        content: "hello".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::ToGui));
    assert!(state.read().await.buffered_messages.is_empty());
}

#[tokio::test]
async fn invalid_target_is_dropped_not_buffered() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let msg = BridgeMessage {
        id: "bad-1".into(),
        from: "user".into(),
        to: "admin".into(),
        content: "hello".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Dropped));
    assert!(state.read().await.buffered_messages.is_empty());
}

#[tokio::test]
async fn valid_role_offline_is_buffered() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let msg = BridgeMessage {
        id: "buf-1".into(),
        from: "user".into(),
        to: "reviewer".into(),
        content: "review this".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Buffered));
    assert_eq!(state.read().await.buffered_messages.len(), 1);
}

#[test]
fn visible_messages_require_non_whitespace_content() {
    let visible = BridgeMessage {
        id: "msg-visible".into(),
        from: "coder".into(),
        to: "user".into(),
        content: "hello".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
    };
    let empty = BridgeMessage {
        id: "msg-empty".into(),
        from: "coder".into(),
        to: "user".into(),
        content: "   \n\t".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
    };
    assert!(is_renderable_message(&visible));
    assert!(!is_renderable_message(&empty));
}

#[test]
fn claude_thinking_starts_only_for_delivered_non_claude_messages() {
    let msg = BridgeMessage {
        id: "msg-claude".into(),
        from: "user".into(),
        to: "lead".into(),
        content: "please help".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
    };
    assert!(should_emit_claude_thinking(
        &msg,
        &RouteResult::Delivered,
        "lead",
    ));
    assert!(!should_emit_claude_thinking(
        &msg,
        &RouteResult::Buffered,
        "lead",
    ));
    assert!(!should_emit_claude_thinking(
        &BridgeMessage {
            from: "lead".into(),
            ..msg.clone()
        },
        &RouteResult::Delivered,
        "lead",
    ));
}
