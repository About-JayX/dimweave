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
        attachments: None,    };
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
        attachments: None,    };
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
        attachments: None,    };
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
            agent_id: None,
        });
        s.task_graph
            .set_lead_session(&task.task_id, &lead.session_id);
        s.task_graph
            .set_external_session_id(&lead.session_id, "claude_current");
        s.claude_role = "lead".into();
        // Claude online via SDK WS (is_agent_online checks SDK WS, not attached_agents)
        let epoch = s.begin_claude_sdk_launch("nonce-stale".into());
        s.attach_claude_sdk_ws(epoch, "nonce-stale", claude_tx);
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
        attachments: None,    };

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
    tokio::sync::mpsc::Receiver<String>,
    tokio::sync::mpsc::Receiver<(Vec<serde_json::Value>, bool)>,
) {
    use crate::daemon::task_graph::types::Provider;
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_tx, claude_rx) = tokio::sync::mpsc::channel::<String>(8);
    let (codex_tx, codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(8);
    let ids = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task_with_config(
            "/repo-b", "repo-b", Provider::Codex, Provider::Claude,
        );
        s.active_task_id = Some(task.task_id.clone());
        // Register task_agents for authoritative routing
        let codex_agent = s.task_graph.add_task_agent(&task.task_id, Provider::Codex, "lead");
        let claude_agent = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");

        let lead = s.task_graph.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Codex,
            role: SessionRole::Lead,
            cwd: "/repo-b",
            title: "Lead",
            agent_id: None,
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
            agent_id: None,
        });
        s.task_graph.set_coder_session(&task.task_id, &coder.session_id);
        s.task_graph
            .set_external_session_id(&coder.session_id, "claude_coder_current");

        s.claude_role = "coder".into();
        s.codex_role = "lead".into();
        // Per-agent-id slots (authoritative mode)
        s.init_task_runtime(&task.task_id, "/repo-b".into());
        s.task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot(&claude_agent.agent_id)
            .ws_tx = Some(claude_tx);
        s.task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_codex_slot(&codex_agent.agent_id, 4500)
            .inject_tx = Some(codex_tx);
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
        attachments: None,    };

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
        attachments: None,    };

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
            agent_id: None,
        });
        s.task_graph
            .set_lead_session(&task.task_id, &lead.session_id);
        s.task_graph
            .set_external_session_id(&lead.session_id, "claude_current");
        s.claude_role = "lead".into();
        // Claude online via SDK WS (is_agent_online checks SDK WS, not attached_agents)
        let epoch = s.begin_claude_sdk_launch("nonce-stale-reason".into());
        s.attach_claude_sdk_ws(epoch, "nonce-stale-reason", claude_tx);
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
        attachments: None,    };

    let outcome = route_message_inner_with_meta(&state, msg).await;
    assert!(matches!(outcome.result, RouteResult::Buffered));
    assert_eq!(outcome.buffer_reason, Some("task_session_mismatch"));
}

// ── agent_id routing: broadcast delivery ──────────────────────

/// When a task has two agents with the same role ("coder") backed by different
/// providers (Claude + Codex), a message to "coder" must broadcast to BOTH.
#[tokio::test]
async fn agent_id_routing_broadcast_delivers_to_both_providers_for_same_role() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (claude_sdk_tx, mut claude_sdk_rx) = tokio::sync::mpsc::channel::<String>(8);
    let (codex_tx, mut codex_rx) =
        tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(8);
    let task_id = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        // Both agents have role "coder"
        let claude_agent = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "coder");
        let codex_agent = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Codex, "coder");
        // Per-agent-id slots (authoritative mode)
        s.init_task_runtime(&task.task_id, "/ws".into());
        s.task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot(&claude_agent.agent_id)
            .ws_tx = Some(claude_sdk_tx);
        s.task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_codex_slot(&codex_agent.agent_id, 4500)
            .inject_tx = Some(codex_tx);
        // Compat singletons
        s.claude_role = "coder".into();
        s.codex_role = "coder".into();
        task.task_id
    };
    let msg = BridgeMessage {
        id: "broadcast-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "coder".into(),
        content: "implement feature X".into(),
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
        "broadcast should deliver to at least one target"
    );
    assert!(
        claude_sdk_rx.try_recv().is_ok(),
        "Claude SDK should receive the broadcast"
    );
    assert!(
        codex_rx.try_recv().is_ok(),
        "Codex should also receive the broadcast"
    );
}

/// When a task has two Claude agents with the same role ("coder"), each with its
/// own per-agent-id slot, a message to "coder" must deliver to BOTH slots.
/// This proves that routing resolves per-agent-id, not per-provider.
#[tokio::test]
async fn agent_id_routing_two_same_provider_agents_both_receive_delivery() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (tx_a, mut rx_a) = tokio::sync::mpsc::channel::<String>(8);
    let (tx_b, mut rx_b) = tokio::sync::mpsc::channel::<String>(8);
    let task_id = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        let agent_a = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");
        let agent_b = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");
        s.claude_role = "coder".into();
        // Init task runtime and create per-agent-id slots
        s.init_task_runtime(&task.task_id, "/ws".into());
        let slot_a = s.task_runtimes.get_mut(&task.task_id).unwrap()
            .get_or_create_claude_slot(&agent_a.agent_id);
        slot_a.ws_tx = Some(tx_a);
        let slot_b = s.task_runtimes.get_mut(&task.task_id).unwrap()
            .get_or_create_claude_slot(&agent_b.agent_id);
        slot_b.ws_tx = Some(tx_b);
        task.task_id
    };
    let msg = BridgeMessage {
        id: "two-claude-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "coder".into(),
        content: "implement feature X".into(),
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
        "two same-provider agents should deliver"
    );
    assert!(
        rx_a.try_recv().is_ok(),
        "Agent A should receive the message via its own slot"
    );
    assert!(
        rx_b.try_recv().is_ok(),
        "Agent B should receive the message via its own slot"
    );
}

/// When a task has per-agent records and matched agent A has no per-agent
/// channel, routing must NOT fall back to a provider/task channel that
/// belongs to another agent B. Agent A's delivery is simply skipped.
#[tokio::test]
async fn agent_id_routing_no_fallback_to_provider_channel_in_per_agent_mode() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let (tx_a, mut rx_a) = tokio::sync::mpsc::channel::<String>(8);
    let task_id = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        let agent_a = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "coder");
        let _agent_b = s
            .task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "coder");
        s.claude_role = "coder".into();
        s.init_task_runtime(&task.task_id, "/ws".into());
        // Agent A gets a per-agent slot with a channel
        let slot_a = s
            .task_runtimes
            .get_mut(&task.task_id)
            .unwrap()
            .get_or_create_claude_slot(&agent_a.agent_id);
        slot_a.ws_tx = Some(tx_a);
        // Agent B gets a per-agent slot with NO channel (offline)
        // — no slot created means no channel exists
        task.task_id
    };
    let msg = BridgeMessage {
        id: "no-fallback-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "coder".into(),
        content: "implement feature".into(),
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
        "should deliver to agent A which has a channel"
    );
    assert!(
        rx_a.try_recv().is_ok(),
        "Agent A should receive the message via its own per-agent slot"
    );
    // Agent A's channel should have received exactly one message, not two
    assert!(
        rx_a.try_recv().is_err(),
        "Agent A's channel must not receive a second copy from provider fallback"
    );
}

/// When a task has agents but the target role has NO matching agent,
/// the message should be buffered, not silently rerouted.
#[tokio::test]
async fn agent_id_routing_missing_role_buffers_clearly() {
    let state = Arc::new(RwLock::new(DaemonState::new()));
    let task_id = {
        let mut s = state.write().await;
        let task = s.task_graph.create_task("/ws", "T");
        s.active_task_id = Some(task.task_id.clone());
        s.task_graph
            .add_task_agent(&task.task_id, Provider::Claude, "lead");
        let epoch = s.begin_claude_sdk_launch("nonce-mr".into());
        let (tx, _) = tokio::sync::mpsc::channel::<String>(1);
        s.attach_claude_sdk_ws(epoch, "nonce-mr", tx);
        s.claude_role = "lead".into();
        task.task_id
    };
    let msg = BridgeMessage {
        id: "missing-role-1".into(),
        from: "user".into(),
        display_source: Some("user".into()),
        to: "reviewer".into(),
        content: "review please".into(),
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
        matches!(result, RouteResult::Buffered),
        "missing role should buffer, not silently reroute"
    );
}
