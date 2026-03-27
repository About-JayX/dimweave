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
    let msg = BridgeMessage {
        id: "msg-1".into(),
        from: "intruder".into(),
        to: "lead".into(),
        content: "hello".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Dropped));
}

// ── resolve_user_targets tests ──────────────────────────────────────

#[test]
fn explicit_target_returns_single_role() {
    let s = DaemonState::new();
    assert_eq!(resolve_user_targets(&s, "coder"), vec!["coder"]);
}

#[test]
fn auto_with_no_agents_returns_empty() {
    let s = DaemonState::new();
    assert!(resolve_user_targets(&s, "auto").is_empty());
}

#[test]
fn auto_with_claude_only() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    s.attached_agents
        .insert("claude".into(), crate::daemon::state::AgentSender::new(tx, 0));
    let targets = resolve_user_targets(&s, "auto");
    assert_eq!(targets, vec!["lead"]);
}

#[test]
fn auto_with_codex_only() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    s.codex_inject_tx = Some(tx);
    let targets = resolve_user_targets(&s, "auto");
    assert_eq!(targets, vec!["coder"]);
}

#[test]
fn auto_with_both_agents_returns_two_roles() {
    let mut s = DaemonState::new();
    let (claude_tx, _) = tokio::sync::mpsc::channel(1);
    let (codex_tx, _) = tokio::sync::mpsc::channel(1);
    s.attached_agents
        .insert("claude".into(), crate::daemon::state::AgentSender::new(claude_tx, 0));
    s.codex_inject_tx = Some(codex_tx);
    let targets = resolve_user_targets(&s, "auto");
    assert_eq!(targets, vec!["lead", "coder"]);
}

#[test]
fn auto_dedupes_when_same_role() {
    let mut s = DaemonState::new();
    s.claude_role = "coder".into();
    s.codex_role = "coder".into();
    let (claude_tx, _) = tokio::sync::mpsc::channel(1);
    let (codex_tx, _) = tokio::sync::mpsc::channel(1);
    s.attached_agents
        .insert("claude".into(), crate::daemon::state::AgentSender::new(claude_tx, 0));
    s.codex_inject_tx = Some(codex_tx);
    let targets = resolve_user_targets(&s, "auto");
    // Same role — should deduplicate to one entry
    assert_eq!(targets, vec!["coder"]);
}

#[test]
fn auto_excludes_user_role() {
    let mut s = DaemonState::new();
    s.claude_role = "user".into();
    let (tx, _) = tokio::sync::mpsc::channel(1);
    s.attached_agents
        .insert("claude".into(), crate::daemon::state::AgentSender::new(tx, 0));
    // "user" role should be filtered out from auto targets
    let targets = resolve_user_targets(&s, "auto");
    assert!(targets.is_empty());
}
