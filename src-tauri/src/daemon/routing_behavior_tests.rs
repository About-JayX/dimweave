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
        // Claude online via SDK WS
        let epoch = s.begin_claude_sdk_launch("nonce-fanout".into());
        s.attach_claude_sdk_ws(epoch, "nonce-fanout", claude_tx);
        s.claude_role = "lead".into();
        s.codex_role = "coder".into();
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

// ── reply-target redirect tests ──────────────────────────────────

#[tokio::test]
async fn reply_target_redirects_role_reply_to_delegating_agent() {
    clear_reply_targets();
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, mut claude_rx) = tokio::sync::mpsc::channel(8);
    let (codex_tx, mut codex_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        let epoch = s.begin_claude_sdk_launch("nonce-rt".into());
        s.attach_claude_sdk_ws(epoch, "nonce-rt", claude_tx);
        s.claude_role = "lead".into();
        s.codex_role = "coder".into();
        s.codex_inject_tx = Some(codex_tx);
    }
    // Step 1: Lead (Claude) sends agent-targeted message to "codex"
    let delegate = BridgeMessage {
        id: "delegate-rt-1".into(),
        from: "lead".into(),
        display_source: Some("claude".into()),
        to: "codex".into(),
        content: "implement this".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::InProgress),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("claude".into()),
        attachments: None,
    };
    let result = route_message_inner(&state, delegate).await;
    assert!(matches!(result, RouteResult::Delivered));
    assert!(codex_rx.try_recv().is_ok(), "codex receives delegation");
    // Step 2: Coder (Codex) replies to role "lead" → redirected to "claude"
    let reply = BridgeMessage {
        id: "reply-rt-1".into(),
        from: "coder".into(),
        display_source: Some("codex".into()),
        to: "lead".into(),
        content: "done".into(),
        timestamp: 2,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("codex".into()),
        attachments: None,
    };
    let result = route_message_inner(&state, reply).await;
    assert!(
        matches!(result, RouteResult::Delivered),
        "reply must be redirected to delegating agent"
    );
    assert!(
        claude_rx.try_recv().is_ok(),
        "Claude (delegator) should receive the redirected reply"
    );
}

/// Regression: a redirected reply must NOT create a reciprocal mapping
/// that turns subsequent non-reply lead messages into sticky redirects.
#[tokio::test]
async fn reply_target_no_reciprocal_mapping_from_redirected_reply() {
    clear_reply_targets();
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, mut claude_rx) = tokio::sync::mpsc::channel(8);
    let (codex_tx, mut codex_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        let epoch = s.begin_claude_sdk_launch("nonce-recip".into());
        s.attach_claude_sdk_ws(epoch, "nonce-recip", claude_tx);
        s.claude_role = "lead".into();
        s.codex_role = "coder".into();
        s.codex_inject_tx = Some(codex_tx);
    }
    // Step 1: Lead delegates to coder by agent_id
    let delegate = BridgeMessage {
        id: "del-recip-1".into(),
        from: "lead".into(),
        display_source: Some("claude".into()),
        to: "codex".into(),
        content: "implement".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::InProgress),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("claude".into()),
        attachments: None,
    };
    assert!(matches!(route_message_inner(&state, delegate).await, RouteResult::Delivered));
    assert!(codex_rx.try_recv().is_ok());
    // Step 2: Coder replies to "lead" → redirected to claude
    let reply = BridgeMessage {
        id: "reply-recip-1".into(),
        from: "coder".into(),
        display_source: Some("codex".into()),
        to: "lead".into(),
        content: "done".into(),
        timestamp: 2,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("codex".into()),
        attachments: None,
    };
    assert!(matches!(route_message_inner(&state, reply).await, RouteResult::Delivered));
    assert!(claude_rx.try_recv().is_ok());
    // Step 3: Lead sends a NEW message to role "coder" — must NOT be
    // redirected by a reciprocal mapping; should reach codex via normal
    // role resolution.
    let new_msg = BridgeMessage {
        id: "new-lead-msg-1".into(),
        from: "lead".into(),
        display_source: Some("claude".into()),
        to: "coder".into(),
        content: "next task".into(),
        timestamp: 3,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::InProgress),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("claude".into()),
        attachments: None,
    };
    let result = route_message_inner(&state, new_msg).await;
    assert!(matches!(result, RouteResult::Delivered));
    assert!(
        codex_rx.try_recv().is_ok(),
        "codex receives via normal role resolution"
    );
    // Verify the message was NOT also redirected to claude (self-delivery)
    assert!(
        claude_rx.try_recv().is_err(),
        "no reciprocal redirect — claude must NOT receive its own message"
    );
}

// ── task_runtime_routing: AC1/AC2 task-first provider resolution ────

#[tokio::test]
async fn task_runtime_routing_delivers_via_task_local_codex_channel() {
    use crate::daemon::task_graph::types::{CreateSessionParams, Provider, SessionRole};
    use crate::daemon::task_runtime::{CodexTaskSlot, TaskRuntime};
    use crate::daemon::types::ProviderConnectionMode;

    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (task_codex_tx, mut task_codex_rx) = tokio::sync::mpsc::channel(8);
    let (global_codex_tx, mut global_codex_rx) = tokio::sync::mpsc::channel(8);
    let task_id;
    {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/ws", "Task");
        task_id = task.task_id.clone();
        s.active_task_id = Some(task.task_id.clone());
        let sess = s.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Codex,
            role: SessionRole::Coder,
            cwd: "/ws",
            title: "Coder",
            agent_id: None,
        });
        s.task_graph
            .set_coder_session(&task.task_id, &sess.session_id);
        s.task_graph
            .set_external_session_id(&sess.session_id, "thread_1");
        // Wire task-local Codex slot
        s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
        let rt = s.task_runtimes.get_mut(&task.task_id).unwrap();
        let mut slot = CodexTaskSlot::new(4500);
        slot.inject_tx = Some(task_codex_tx);
        slot.connection = Some(crate::daemon::types::ProviderConnectionState {
            provider: Provider::Codex,
            external_session_id: "thread_1".into(),
            cwd: "/ws".into(),
            connection_mode: ProviderConnectionMode::New,
        });
        rt.codex_slot = Some(slot);
        // Also wire global (should NOT be used for task-scoped messages)
        s.codex_inject_tx = Some(global_codex_tx);
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
    let msg = BridgeMessage {
        id: "task-local-codex-1".into(),
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
        "task-scoped message must be delivered"
    );
    assert!(
        task_codex_rx.try_recv().is_ok(),
        "must use task-local Codex channel"
    );
    assert!(
        global_codex_rx.try_recv().is_err(),
        "must NOT use global Codex channel"
    );
}

#[tokio::test]
async fn task_runtime_routing_delivers_via_task_local_claude_channel() {
    use crate::daemon::task_graph::types::{CreateSessionParams, Provider, SessionRole};
    use crate::daemon::task_runtime::{ClaudeTaskSlot, TaskRuntime};

    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (task_claude_tx, mut task_claude_rx) = tokio::sync::mpsc::channel(8);
    let (global_claude_tx, mut global_claude_rx) = tokio::sync::mpsc::channel(8);
    let task_id;
    {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/ws", "Task");
        task_id = task.task_id.clone();
        s.active_task_id = Some(task.task_id.clone());
        let sess = s.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            cwd: "/ws",
            title: "Lead",
            agent_id: None,
        });
        s.task_graph
            .set_lead_session(&task.task_id, &sess.session_id);
        s.task_graph
            .set_external_session_id(&sess.session_id, "sess_1");
        // Wire task-local Claude slot
        s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
        let rt = s.task_runtimes.get_mut(&task.task_id).unwrap();
        let mut slot = ClaudeTaskSlot::new();
        slot.ws_tx = Some(task_claude_tx);
        rt.claude_slot = Some(slot);
        // Also wire global (should NOT be used for task-scoped messages)
        s.claude_sdk_ws_tx = Some(global_claude_tx);
        s.claude_role = "lead".into();
        s.set_provider_connection(
            "claude",
            crate::daemon::types::ProviderConnectionState {
                provider: Provider::Claude,
                external_session_id: "sess_1".into(),
                cwd: "/ws".into(),
                connection_mode: crate::daemon::types::ProviderConnectionMode::New,
            },
        );
    }
    let msg = BridgeMessage {
        id: "task-local-claude-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "lead".into(),
        content: "plan the task".into(),
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
        "task-scoped message must be delivered"
    );
    assert!(
        task_claude_rx.try_recv().is_ok(),
        "must use task-local Claude channel"
    );
    assert!(
        global_claude_rx.try_recv().is_err(),
        "must NOT use global Claude channel"
    );
}

#[tokio::test]
async fn task_runtime_routing_falls_back_to_global_without_task_id() {
    // Without task_id, routing should use global channels
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (global_codex_tx, mut global_codex_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        s.codex_inject_tx = Some(global_codex_tx);
        s.codex_role = "coder".into();
    }
    let msg = BridgeMessage {
        id: "global-fallback-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "coder".into(),
        content: "implement".into(),
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
    assert!(global_codex_rx.try_recv().is_ok());
}

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
            agent_id: None,
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
