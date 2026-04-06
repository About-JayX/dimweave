use super::*;
use crate::daemon::{
    state::DaemonState,
    task_graph::types::{CreateSessionParams, Provider, SessionRole},
    types::{
        BridgeMessage, MessageStatus, ProviderConnectionMode, ProviderConnectionState, ToAgent,
    },
};
use std::sync::Arc;
use tokio::sync::RwLock;

// ── shared-role live routing tests ──────────────────────────────
//
// These tests verify that when Claude and Codex share the same role,
// routing delivers to whichever agent is online rather than buffering
// because the first-checked agent happens to be offline.

/// When Claude is offline but holds role "lead" (cached), and Codex is online
/// with the same "lead" role, a message to "lead" must be delivered to the
/// online Codex — not buffered because of Claude's offline status.
#[tokio::test]
async fn route_to_live_codex_when_offline_claude_shares_role() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (codex_tx, mut codex_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        s.claude_role = "lead".into();
        s.codex_role = "lead".into();
        // Claude is NOT in attached_agents (offline)
        // Codex IS online
        s.codex_inject_tx = Some(codex_tx);
    }
    let msg = BridgeMessage {
        id: "shared-role-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "lead".into(),
        content: "please summarize".into(),
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
        "expected Delivered to online Codex, got Buffered/Dropped"
    );
    assert!(
        codex_rx.try_recv().is_ok(),
        "Codex should have received the message"
    );
    assert!(
        state.read().await.buffered_messages.is_empty(),
        "nothing should be buffered when an online candidate exists"
    );
}

/// When neither Claude nor Codex is online for the target role, it must buffer.
#[tokio::test]
async fn shared_role_both_offline_still_buffers() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    {
        let mut s = state.write().await;
        s.claude_role = "lead".into();
        s.codex_role = "lead".into();
        // Both offline: no attached_agents, no codex_inject_tx
    }
    let msg = BridgeMessage {
        id: "both-off-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "lead".into(),
        content: "hello".into(),
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
        matches!(result, RouteResult::Buffered),
        "both agents offline => must buffer"
    );
    assert_eq!(state.read().await.buffered_messages.len(), 1);
}

/// Mirror case: Claude is online, Codex is offline, same role => Delivered to Claude.
#[tokio::test]
async fn route_to_live_claude_when_offline_codex_shares_role() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, mut claude_rx) = tokio::sync::mpsc::channel(8);
    {
        let mut s = state.write().await;
        s.claude_role = "lead".into();
        s.codex_role = "lead".into();
        s.attached_agents.insert(
            "claude".into(),
            crate::daemon::state::AgentSender::new(claude_tx, 0),
        );
        // Codex offline: no codex_inject_tx
    }
    let msg = BridgeMessage {
        id: "mirror-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "lead".into(),
        content: "hello".into(),
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
        "online Claude should receive the message"
    );
    assert!(claude_rx.try_recv().is_ok());
    assert!(state.read().await.buffered_messages.is_empty());
}

#[tokio::test]
async fn stale_online_agent_for_same_role_is_buffered_when_task_session_does_not_match() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, mut claude_rx) = tokio::sync::mpsc::channel(8);
    let (task_id, lead_session_id) = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/repo-b", "repo-b");
        s.active_task_id = Some(task.task_id.clone());
        let lead = s.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            cwd: "/repo-b",
            title: "Lead",
        });
        s.task_graph
            .set_lead_session(&task.task_id, &lead.session_id);
        s.task_graph
            .set_external_session_id(&lead.session_id, "claude_current");
        s.claude_role = "lead".into();
        s.attached_agents.insert(
            "claude".into(),
            crate::daemon::state::AgentSender::new(claude_tx, 0),
        );
        s.set_provider_connection(
            "claude",
            ProviderConnectionState {
                provider: Provider::Claude,
                external_session_id: "claude_stale".into(),
                cwd: "/repo-a".into(),
                connection_mode: ProviderConnectionMode::Resumed,
            },
        );
        (task.task_id, lead.session_id)
    };

    let msg = BridgeMessage {
        id: "stale-session-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "lead".into(),
        content: "route only to the current task".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: Some(task_id),
        session_id: Some(lead_session_id),
        sender_agent_id: None,
        attachments: None,
    };

    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Buffered));
    assert!(claude_rx.try_recv().is_err());
    assert_eq!(state.read().await.buffered_messages.len(), 1);
}

async fn seeded_task_with_codex_lead_and_claude_coder() -> (
    SharedState,
    String,
    String,
    String,
    tokio::sync::mpsc::Receiver<ToAgent>,
    tokio::sync::mpsc::Receiver<(Vec<serde_json::Value>, bool)>,
) {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(8);
    let (codex_tx, codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(8);
    let ids = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/repo-b", "repo-b");
        s.active_task_id = Some(task.task_id.clone());

        let lead = s.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Codex,
            role: SessionRole::Lead,
            cwd: "/repo-b",
            title: "Lead",
        });
        s.task_graph.set_lead_session(&task.task_id, &lead.session_id);
        s.task_graph
            .set_external_session_id(&lead.session_id, "codex_lead_current");

        let coder = s.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: Some(&lead.session_id),
            provider: Provider::Claude,
            role: SessionRole::Coder,
            cwd: "/repo-b",
            title: "Coder",
        });
        s.task_graph.set_coder_session(&task.task_id, &coder.session_id);
        s.task_graph
            .set_external_session_id(&coder.session_id, "claude_coder_current");

        s.claude_role = "coder".into();
        s.codex_role = "lead".into();
        s.attached_agents.insert(
            "claude".into(),
            crate::daemon::state::AgentSender::new(claude_tx, 0),
        );
        s.codex_inject_tx = Some(codex_tx);
        s.set_provider_connection(
            "claude",
            ProviderConnectionState {
                provider: Provider::Claude,
                external_session_id: "claude_coder_current".into(),
                cwd: "/repo-b".into(),
                connection_mode: ProviderConnectionMode::Resumed,
            },
        );
        s.set_provider_connection(
            "codex",
            ProviderConnectionState {
                provider: Provider::Codex,
                external_session_id: "codex_lead_current".into(),
                cwd: "/repo-b".into(),
                connection_mode: ProviderConnectionMode::Resumed,
            },
        );
        (task.task_id, lead.session_id, coder.session_id)
    };
    (state, ids.0, ids.1, ids.2, claude_rx, codex_rx)
}

#[tokio::test]
async fn lead_to_coder_uses_target_coder_session_not_sender_lead_session() {
    let (state, task_id, lead_session_id, _coder_session_id, _claude_rx, _codex_rx) =
        seeded_task_with_codex_lead_and_claude_coder().await;
    let msg = BridgeMessage {
        id: "lead-to-coder-1".into(),
        from: "lead".into(),
        display_source: Some("codex".into()),
        to: "coder".into(),
        content: "implement task 1".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::Done),
        task_id: Some(task_id),
        session_id: Some(lead_session_id),
        sender_agent_id: Some("codex".into()),
        attachments: None,
    };

    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Delivered));
}

#[tokio::test]
async fn coder_to_lead_uses_target_lead_session_not_sender_coder_session() {
    let (state, task_id, _lead_session_id, coder_session_id, _claude_rx, _codex_rx) =
        seeded_task_with_codex_lead_and_claude_coder().await;
    let msg = BridgeMessage {
        id: "coder-to-lead-1".into(),
        from: "coder".into(),
        display_source: Some("claude".into()),
        to: "lead".into(),
        content: "task 1 complete".into(),
        timestamp: 2,
        reply_to: None,
        priority: None,
        status: Some(MessageStatus::Done),
        task_id: Some(task_id),
        session_id: Some(coder_session_id),
        sender_agent_id: Some("claude".into()),
        attachments: None,
    };

    let result = route_message_inner(&state, msg).await;
    assert!(matches!(result, RouteResult::Delivered));
}

#[tokio::test]
async fn stale_online_agent_reports_task_session_mismatch_reason() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel(8);
    let (task_id, lead_session_id) = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/repo-b", "repo-b");
        s.active_task_id = Some(task.task_id.clone());
        let lead = s.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            cwd: "/repo-b",
            title: "Lead",
        });
        s.task_graph
            .set_lead_session(&task.task_id, &lead.session_id);
        s.task_graph
            .set_external_session_id(&lead.session_id, "claude_current");
        s.claude_role = "lead".into();
        s.attached_agents.insert(
            "claude".into(),
            crate::daemon::state::AgentSender::new(claude_tx, 0),
        );
        s.set_provider_connection(
            "claude",
            ProviderConnectionState {
                provider: Provider::Claude,
                external_session_id: "claude_stale".into(),
                cwd: "/repo-a".into(),
                connection_mode: ProviderConnectionMode::Resumed,
            },
        );
        (task.task_id, lead.session_id)
    };

    let msg = BridgeMessage {
        id: "stale-session-reason-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "lead".into(),
        content: "route only to the current task".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: None,
        task_id: Some(task_id),
        session_id: Some(lead_session_id),
        sender_agent_id: None,
        attachments: None,
    };

    let outcome = route_message_inner_with_meta(&state, msg).await;
    assert!(matches!(outcome.result, RouteResult::Buffered));
    assert_eq!(outcome.buffer_reason, Some("task_session_mismatch"));
}
