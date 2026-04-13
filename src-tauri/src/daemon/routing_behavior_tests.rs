use super::*;
use crate::daemon::routing_display::{
    buffered_route_message, is_renderable_message, should_emit_claude_thinking,
};
use crate::daemon::{
    state::DaemonState,
    types::{Attachment, BridgeMessage},
};
use std::sync::Arc;
use tokio::sync::RwLock;

fn file_attachment() -> Attachment {
    Attachment {
        file_path: "/tmp/spec.md".into(),
        file_name: "spec.md".into(),
        is_image: false,
        media_type: None,
    }
}

// ── is_valid_agent_role tests ─────────────────────────────────────

#[test]
fn valid_roles_accepted() {
    for role in &["lead", "coder"] {
        assert!(
            crate::daemon::is_valid_agent_role(role),
            "{role} should be valid"
        );
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
    assert!(!crate::daemon::is_valid_agent_role("tester"));
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
            display_source: Some("user".into()),
            to: role.clone(),
            content: "hello".into(),
            timestamp: 1,
            reply_to: None,
            priority: None,
            status: None,
            task_id: None,
            session_id: None,
            sender_agent_id: None,
            attachments: None,        };
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
        display_source: Some("user".into()),
        to: "user".into(),
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
    assert!(matches!(result, RouteResult::ToGui));
    assert!(state.read().await.buffered_messages.is_empty());
}

#[tokio::test]
async fn invalid_target_is_dropped_not_buffered() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let msg = BridgeMessage {
        id: "bad-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "admin".into(),
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
    assert!(state.read().await.buffered_messages.is_empty());
}

#[tokio::test]
async fn valid_role_offline_is_buffered() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let msg = BridgeMessage {
        id: "buf-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "coder".into(),
        content: "implement this".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,    };
    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Buffered));
    assert_eq!(state.read().await.buffered_messages.len(), 1);
}

#[tokio::test]
async fn removed_role_target_is_dropped_not_buffered() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let msg = BridgeMessage {
        id: "bad-tester-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "tester".into(),
        content: "test this".into(),
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
    assert!(state.read().await.buffered_messages.is_empty());
}

#[test]
fn visible_messages_require_content_or_attachments() {
    let visible = BridgeMessage {
        id: "msg-visible".into(),
        from: "coder".into(),
        display_source: Some("codex".into()),
        to: "user".into(),
        content: "hello".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,    };
    let attachment_only = BridgeMessage {
        id: "msg-attachment".into(),
        from: "coder".into(),
        display_source: Some("codex".into()),
        to: "user".into(),
        content: "   \n\t".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: Some(vec![file_attachment()]),
    };
    let empty = BridgeMessage {
        id: "msg-empty".into(),
        from: "coder".into(),
        display_source: Some("codex".into()),
        to: "user".into(),
        content: "   \n\t".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,    };
    assert!(is_renderable_message(&visible));
    assert!(is_renderable_message(&attachment_only));
    assert!(!is_renderable_message(&empty));
}

#[test]
fn claude_thinking_starts_only_for_delivered_non_claude_messages() {
    let msg = BridgeMessage {
        id: "msg-claude".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "lead".into(),
        content: "please help".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,    };
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
            display_source: Some("claude".into()),
            ..msg.clone()
        },
        &RouteResult::Delivered,
        "lead",
    ));
}

#[test]
fn task_session_mismatch_buffer_message_is_not_reported_as_offline() {
    let text = buffered_route_message("coder", Some("task_session_mismatch"));
    assert!(text.contains("task session"));
    assert!(!text.contains("offline"));
}

// ── task_runtime_routing: AC1/AC2 task-first provider resolution ────

#[tokio::test]
async fn task_first_routing_resolves_codex_coder_via_task_provider() {
    use crate::daemon::task_graph::types::{CreateSessionParams, Provider, SessionRole};
    use crate::daemon::types::ProviderConnectionMode;

    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (codex_tx, mut codex_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/ws", "Task");
        s.active_task_id = Some(task.task_id.clone());
        // Create a Codex coder session for the task
        let sess = s.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Codex,
            role: SessionRole::Coder,
            cwd: "/ws",
            title: "Coder",
        });
        s.task_graph
            .set_coder_session(&task.task_id, &sess.session_id);
        s.task_graph
            .set_external_session_id(&sess.session_id, "thread_1");
        // Wire Codex online
        s.codex_inject_tx = Some(codex_tx);
        s.codex_role = "coder".into();
        s.set_provider_connection(
            "codex",
            crate::daemon::types::ProviderConnectionState {
                provider: Provider::Codex,
                external_session_id: "thread_1".into(),
                cwd: "/ws".into(),
                connection_mode: ProviderConnectionMode::New,
            },
        );
    }

    // Message with task_id targets "coder" — should route to Codex via task provider
    let task_id = state.read().await.active_task_id.clone().unwrap();
    let msg = BridgeMessage {
        id: "task-route-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "coder".into(),
        content: "implement this".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: Some(task_id),
        session_id: None,
        sender_agent_id: None,
        attachments: None,
    };
    let result = route_message_inner(&state, msg).await;
    assert!(
        matches!(result, RouteResult::Delivered),
        "task-scoped coder message must reach Codex"
    );
    assert!(codex_rx.try_recv().is_ok());
}

#[tokio::test]
async fn task_first_routing_uses_global_role_without_task_id() {
    // Without task_id, routing falls back to global claude_role/codex_role
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (codex_tx, mut codex_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        s.codex_inject_tx = Some(codex_tx);
        s.codex_role = "coder".into();
    }
    let msg = BridgeMessage {
        id: "global-route-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "coder".into(),
        content: "implement this".into(),
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
    assert!(
        matches!(result, RouteResult::Delivered),
        "global role fallback must still work"
    );
    assert!(codex_rx.try_recv().is_ok());
}
