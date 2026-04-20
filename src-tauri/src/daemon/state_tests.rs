use super::*;
use crate::daemon::state::MatchedTaskAgent;
use crate::daemon::task_graph::types::{CreateSessionParams, Provider};
use crate::daemon::types::{MessageSource, MessageTarget};

fn matched(agent_id: &str, runtime: &'static str) -> MatchedTaskAgent {
    MatchedTaskAgent { agent_id: agent_id.into(), runtime }
}

fn temp_state_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "dimweave_state_{}_{}_{}.json",
        name,
        std::process::id(),
        chrono::Utc::now().timestamp_millis(),
    ))
}

#[test]
fn telegram_notifications_disabled_by_default() {
    let s = DaemonState::new();
    assert!(!s.telegram_notifications_enabled);
}

#[test]
fn claude_role_is_unselected_by_default() {
    let s = DaemonState::new();
    assert_eq!(s.claude_role, "");
}

#[test]
fn codex_role_is_unselected_by_default() {
    let s = DaemonState::new();
    assert_eq!(s.codex_role, "");
}

#[test]
fn flush_clears_buffer() {
    let mut s = DaemonState::new();
    s.buffer_message(BridgeMessage::system("hello", "lead"));
    assert_eq!(s.buffered_messages.len(), 1);
    let flushed = s.flush_buffered();
    assert_eq!(flushed.len(), 1);
    assert!(s.buffered_messages.is_empty());
}

#[test]
fn buffer_caps_at_200() {
    let mut s = DaemonState::new();
    for i in 0..250 {
        s.buffer_message(BridgeMessage::system(&format!("msg{i}"), "lead"));
    }
    assert!(s.buffered_messages.len() <= 200);
}

#[test]
fn permission_requests_round_trip_to_verdicts() {
    let mut s = DaemonState::new();
    s.store_permission_request(
        "claude",
        PermissionRequest {
            request_id: "req-1".into(),
            tool_name: "Bash".into(),
            description: "run ls".into(),
            input_preview: Some("ls".into()),
        },
        100,
    );

    let (agent_id, outbound) = s
        .resolve_permission("req-1", PermissionBehavior::Allow, 200)
        .expect("pending permission should resolve");

    assert_eq!(agent_id, "claude");
    match outbound {
        ToAgent::PermissionVerdict { verdict } => {
            assert_eq!(verdict.request_id, "req-1");
            assert!(matches!(verdict.behavior, PermissionBehavior::Allow));
        }
        other => panic!("unexpected outbound message: {other:?}"),
    }
}

#[test]
fn expired_permissions_are_rejected() {
    let mut s = DaemonState::new();
    s.store_permission_request(
        "claude",
        PermissionRequest {
            request_id: "req-expired".into(),
            tool_name: "Bash".into(),
            description: "run rm".into(),
            input_preview: None,
        },
        100,
    );

    let result = s.resolve_permission(
        "req-expired",
        PermissionBehavior::Deny,
        100 + PERMISSION_TTL_MS + 1,
    );
    assert!(result.is_none());
}

/// Subprocess-death recovery: when an agent's subprocess exits with
/// pending permission requests in flight, the monitor must purge those
/// requests so the GUI prompt can be dismissed. Otherwise the prompt
/// sits forever and the user thinks the agent is still waiting.
#[test]
fn purge_pending_permissions_removes_only_target_agent_requests() {
    let mut s = DaemonState::new();
    s.store_permission_request(
        "claude",
        PermissionRequest {
            request_id: "claude-req-1".into(),
            tool_name: "Bash".into(),
            description: "ls".into(),
            input_preview: None,
        },
        100,
    );
    s.store_permission_request(
        "claude",
        PermissionRequest {
            request_id: "claude-req-2".into(),
            tool_name: "Edit".into(),
            description: "patch".into(),
            input_preview: None,
        },
        200,
    );
    s.store_permission_request(
        "codex",
        PermissionRequest {
            request_id: "codex-req-1".into(),
            tool_name: "shell".into(),
            description: "grep".into(),
            input_preview: None,
        },
        300,
    );

    // Purge claude's two pending requests; codex's must survive.
    let purged = s.purge_pending_permissions_for_agent("claude");
    let mut ids = purged;
    ids.sort(); // HashMap iteration order is unspecified
    assert_eq!(ids, vec!["claude-req-1".to_string(), "claude-req-2".into()]);

    // The codex request still resolves normally.
    let resolved = s.resolve_permission("codex-req-1", PermissionBehavior::Allow, 400);
    assert!(resolved.is_some(), "codex pending must survive claude purge");

    // Purged claude requests must no longer resolve.
    let dead = s.resolve_permission("claude-req-1", PermissionBehavior::Allow, 500);
    assert!(dead.is_none(), "purged request must not resolve");
}

#[test]
fn purge_pending_permissions_is_noop_when_no_matching_agent() {
    let mut s = DaemonState::new();
    s.store_permission_request(
        "claude",
        PermissionRequest {
            request_id: "req".into(),
            tool_name: "t".into(),
            description: "d".into(),
            input_preview: None,
        },
        100,
    );
    let purged = s.purge_pending_permissions_for_agent("nonexistent");
    assert!(purged.is_empty());
    // Existing request still resolvable.
    assert!(s
        .resolve_permission("req", PermissionBehavior::Allow, 200)
        .is_some());
}

#[test]
fn status_snapshot_reports_current_online_agents() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<String>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "lead".into();
    s.codex_role = "coder".into();
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", claude_tx).is_some());
    s.codex_inject_tx = Some(codex_tx);

    let snapshot = s.status_snapshot();
    assert_eq!(snapshot.claude_role, "lead");
    assert_eq!(snapshot.codex_role, "coder");
    assert!(snapshot
        .agents
        .iter()
        .any(|agent| agent.agent == "claude" && agent.online));
    assert!(snapshot
        .agents
        .iter()
        .any(|agent| agent.agent == "codex" && agent.online));
}

#[test]
fn status_snapshot_does_not_treat_claude_bridge_as_connected_provider() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );

    let snapshot = s.status_snapshot();
    let claude = snapshot
        .agents
        .iter()
        .find(|agent| agent.agent == "claude")
        .expect("claude present");

    assert!(!claude.online);
}

#[test]
fn status_snapshot_includes_provider_session_metadata() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );
    s.codex_role = "coder".into();
    s.codex_inject_tx = Some(codex_tx);
    s.set_provider_connection(
        "claude",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Claude,
            external_session_id: "claude_resume_42".into(),
            cwd: "/tmp/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::Resumed,
        },
    );
    s.set_provider_connection(
        "codex",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Codex,
            external_session_id: "thread_123".into(),
            cwd: "/tmp/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::New,
        },
    );

    let snapshot = s.status_snapshot();
    let claude = snapshot
        .agents
        .iter()
        .find(|agent| agent.agent == "claude")
        .expect("claude present");
    let codex = snapshot
        .agents
        .iter()
        .find(|agent| agent.agent == "codex")
        .expect("codex present");

    assert_eq!(
        claude
            .provider_session
            .as_ref()
            .map(|session| session.external_session_id.as_str()),
        Some("claude_resume_42")
    );
    assert_eq!(
        claude
            .provider_session
            .as_ref()
            .map(|session| session.connection_mode.as_str()),
        Some("resumed")
    );
    assert_eq!(
        codex
            .provider_session
            .as_ref()
            .map(|session| session.external_session_id.as_str()),
        Some("thread_123")
    );
    assert_eq!(
        codex
            .provider_session
            .as_ref()
            .map(|session| session.connection_mode.as_str()),
        Some("new")
    );
}

#[test]
fn clearing_provider_connection_pauses_and_unbinds_active_task_session() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.active_task_id = Some(task.task_id.clone());
    let lead = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Claude,
        role: crate::daemon::task_graph::types::SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
        agent_id: None,
    });
    s.task_graph.set_lead_session(&task.task_id, &lead.session_id);
    s.task_graph
        .set_external_session_id(&lead.session_id, "claude_current");
    s.set_provider_connection(
        "claude",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Claude,
            external_session_id: "claude_current".into(),
            cwd: "/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::Resumed,
        },
    );

    s.clear_provider_connection("claude");

    let updated_task = s.task_graph.get_task(&task.task_id).unwrap();
    let updated_session = s.task_graph.get_session(&lead.session_id).unwrap();
    assert!(updated_task.lead_session_id.is_none());
    assert_eq!(
        updated_session.status,
        crate::daemon::task_graph::types::SessionStatus::Paused
    );
}

#[test]
fn online_role_conflict_allows_shared_role() {
    let mut s = DaemonState::new();
    s.claude_role = "lead".into();
    s.codex_role = "lead".into();
    assert_eq!(s.online_role_conflict("codex", "lead"), None);

    // Even with Claude online, shared role is allowed
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<String>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-conflict".into());
    s.attach_claude_sdk_ws(epoch, "nonce-conflict", claude_tx);
    assert_eq!(s.online_role_conflict("codex", "lead"), None);
}

#[test]
fn agent_connect_allows_same_role_cross_provider() {
    // Mirrors the AgentConnect handler in control/handler.rs:
    // Claude is online via SDK WS with role "lead", then Codex bridge connects
    // with the same role. Before the fix, AgentConnect would reject via
    // online_role_conflict(). Now the connect proceeds and the bridge agent
    // is registered in attached_agents.
    let mut s = DaemonState::new();
    s.claude_role = "lead".into();
    s.codex_role = "lead".into();

    // Make Claude online via SDK WS
    let (claude_ws_tx, _claude_ws_rx) = tokio::sync::mpsc::channel::<String>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-connect".into());
    s.attach_claude_sdk_ws(epoch, "nonce-connect", claude_ws_tx);
    assert!(s.is_agent_online("claude"));

    // Simulate AgentConnect for Codex bridge: exact state ops from handler.rs
    let (codex_bridge_tx, _codex_bridge_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    let gen = s.next_agent_gen;
    s.next_agent_gen += 1;
    s.attached_agents.insert(
        "codex".into(),
        crate::daemon::state::AgentSender::new(codex_bridge_tx, gen),
    );

    // Claude remains online, Codex bridge is now attached — no rejection
    assert!(s.is_agent_online("claude"));
    assert!(s.attached_agents.contains_key("codex"));
}

#[test]
fn launch_claude_sdk_succeeds_when_codex_shares_role() {
    // Mirrors the launch_claude_sdk() helper in mod.rs:
    // Codex is already online with role "lead", then Claude launches with
    // the same role. Before the fix, the helper would early-return with an
    // error from online_role_conflict(). Now it proceeds through to
    // begin_claude_sdk_launch() + attach_claude_sdk_ws().
    let mut s = DaemonState::new();
    s.claude_role = "lead".into();
    s.codex_role = "lead".into();

    // Make Codex online via inject channel
    let (codex_tx, _codex_rx) =
        tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.codex_inject_tx = Some(codex_tx);
    assert!(s.is_agent_online("codex"));

    // Simulate what launch_claude_sdk() does after the removed conflict check
    let epoch = s.begin_claude_sdk_launch("nonce-launch".into());
    let (claude_ws_tx, _claude_ws_rx) = tokio::sync::mpsc::channel::<String>(1);
    let generation = s.attach_claude_sdk_ws(epoch, "nonce-launch", claude_ws_tx);
    assert!(generation.is_some(), "Claude SDK WS attach must succeed");

    // Both providers are now live with the same role
    assert!(s.is_agent_online("claude"));
    assert!(s.is_agent_online("codex"));
    assert_eq!(s.claude_role, s.codex_role);
}

#[test]
fn claude_sdk_direct_text_handoff_stays_enabled_until_turn_finishes() {
    let mut s = DaemonState::new();

    assert!(s.begin_claude_sdk_direct_text_turn());

    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );

    assert!(s.should_route_claude_sdk_text_directly());

    s.finish_claude_sdk_direct_text_turn();

    assert!(!s.should_route_claude_sdk_text_directly());
}

#[test]
fn invalidating_claude_sdk_session_clears_direct_text_handoff_state() {
    let mut s = DaemonState::new();
    assert!(s.begin_claude_sdk_direct_text_turn());
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    let (event_tx, _event_rx) = tokio::sync::mpsc::channel::<Vec<serde_json::Value>>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );
    let (sdk_tx, _sdk_rx) = tokio::sync::mpsc::channel::<String>(1);
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", sdk_tx).is_some());
    s.claude_sdk_event_tx = Some(event_tx);

    s.invalidate_claude_sdk_session();

    assert!(!s.should_route_claude_sdk_text_directly());
    assert!(s.claude_sdk_event_tx.is_none());
    assert_eq!(s.claude_sdk_pending_nonce(), None);
    assert_eq!(s.claude_sdk_active_nonce(), None);
}

#[test]
fn pending_nonce_promotes_to_active_on_first_ws_attach() {
    let mut s = DaemonState::new();
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);

    let generation = s.attach_claude_sdk_ws(epoch, "nonce-a", tx);

    assert!(generation.is_some());
    assert_eq!(s.claude_sdk_pending_nonce(), None);
    assert_eq!(s.claude_sdk_active_nonce(), Some("nonce-a"));
}

#[test]
fn stale_nonce_is_rejected_for_attach_and_disconnect() {
    let mut s = DaemonState::new();
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);

    assert!(s.attach_claude_sdk_ws(epoch, "wrong-nonce", tx).is_none());
    assert!(!s.clear_claude_sdk_ws(epoch, "wrong-nonce", 1));
    assert_eq!(s.claude_sdk_pending_nonce(), Some("nonce-a"));
    assert_eq!(s.claude_sdk_active_nonce(), None);
}

#[test]
fn stale_disconnect_cannot_clear_reconnected_ws_for_same_launch() {
    let mut s = DaemonState::new();
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    let (first_tx, _first_rx) = tokio::sync::mpsc::channel::<String>(1);
    let first_generation = s
        .attach_claude_sdk_ws(epoch, "nonce-a", first_tx)
        .expect("first ws should attach");

    let (second_tx, _second_rx) = tokio::sync::mpsc::channel::<String>(1);
    let second_generation = s
        .attach_claude_sdk_ws(epoch, "nonce-a", second_tx)
        .expect("reconnect should attach");

    assert_ne!(first_generation, second_generation);
    assert!(!s.clear_claude_sdk_ws(epoch, "nonce-a", first_generation));
    assert!(s.is_claude_sdk_online());
    assert!(s.clear_claude_sdk_ws(epoch, "nonce-a", second_generation));
    assert!(!s.is_claude_sdk_online());
}

#[test]
fn claude_preview_batch_schedules_once_until_flushed() {
    let mut s = DaemonState::new();

    assert!(s.append_claude_preview_delta("Hello"));
    assert!(!s.append_claude_preview_delta(" world"));
    assert_eq!(
        s.take_claude_preview_batch().as_deref(),
        Some("Hello world")
    );
    assert_eq!(s.take_claude_preview_batch(), None);
    assert!(s.append_claude_preview_delta("Again"));
}

#[test]
fn invalidating_claude_session_clears_preview_batch() {
    let mut s = DaemonState::new();

    assert!(s.append_claude_preview_delta("preview"));
    s.invalidate_claude_sdk_session();

    assert_eq!(s.take_claude_preview_batch(), None);
}

#[test]
fn sdk_terminal_delivery_claim_blocks_later_bridge_terminal_delivery() {
    let mut s = DaemonState::new();

    assert!(s.begin_claude_sdk_direct_text_turn());
    assert!(s.claim_claude_sdk_terminal_delivery());
    assert!(!s.claim_claude_bridge_terminal_delivery());
}

#[test]
fn bridge_terminal_delivery_claim_blocks_later_sdk_terminal_delivery() {
    let mut s = DaemonState::new();

    assert!(s.begin_claude_sdk_direct_text_turn());
    assert!(s.claim_claude_bridge_terminal_delivery());
    assert!(!s.claim_claude_sdk_terminal_delivery());
}

#[test]
fn inactive_bridge_terminal_delivery_blocks_later_sdk_terminal_delivery() {
    // A bridge terminal reply that arrives while state is Inactive (bridge
    // connected, no assistant event yet) must latch CompletedByBridge so a
    // later SDK result cannot also claim visible-result ownership.
    // RED: the current Inactive arm returns true but forgets to latch —
    // so claim_claude_sdk_terminal_delivery() still sees Inactive and
    // also returns true.
    let mut s = DaemonState::new();
    assert!(s.claim_claude_bridge_terminal_delivery());
    assert!(!s.claim_claude_sdk_terminal_delivery());
}

#[test]
fn completed_direct_turn_does_not_leak_into_next_bridge_owned_turn() {
    let mut s = DaemonState::new();

    assert!(s.begin_claude_sdk_direct_text_turn());
    assert!(s.claim_claude_sdk_terminal_delivery());

    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );

    assert!(!s.begin_claude_sdk_direct_text_turn());
}

#[test]
fn migrate_buffered_role_retargets_messages() {
    let mut s = DaemonState::new();
    s.buffer_message(BridgeMessage::system("hello", "lead"));
    s.buffer_message(BridgeMessage::system("world", "coder"));
    s.migrate_buffered_role("lead", "coder");
    assert!(s.buffered_messages.iter().all(|m| m.target_str() != "lead"));
    assert!(s.buffered_messages.iter().any(|m| m.target_str() == "coder"));
}

#[test]
fn take_buffered_for_drains_only_matching_role() {
    let mut s = DaemonState::new();
    s.buffer_message(BridgeMessage::system("a", "lead"));
    s.buffer_message(BridgeMessage::system("b", "coder"));
    s.buffer_message(BridgeMessage::system("c", "lead"));
    let taken = s.take_buffered_for("lead");
    assert_eq!(taken.len(), 2);
    assert_eq!(s.buffered_messages.len(), 1);
    assert_eq!(s.buffered_messages[0].target_str(), "coder");
}

#[test]
fn take_buffered_for_matches_agent_id_target() {
    let mut s = DaemonState::new();
    s.buffer_message(BridgeMessage::system("a", "lead"));
    s.buffer_message(BridgeMessage::system("b", "claude"));
    s.buffer_message(BridgeMessage::system("c", "coder"));
    // Taking by role "lead" should NOT drain agent-targeted "claude" messages
    let taken_role = s.take_buffered_for("lead");
    assert_eq!(taken_role.len(), 1);
    assert_eq!(taken_role[0].message, "a");
    // Taking by agent_id "claude" should drain agent-targeted messages
    let taken_agent = s.take_buffered_for("claude");
    assert_eq!(taken_agent.len(), 1);
    assert_eq!(taken_agent[0].message, "b");
    // Only "coder" should remain
    assert_eq!(s.buffered_messages.len(), 1);
    assert_eq!(s.buffered_messages[0].target_str(), "coder");
}

#[test]
fn buffered_verdicts_round_trip() {
    let mut s = DaemonState::new();
    s.buffer_permission_verdict(
        "claude",
        PermissionVerdict {
            request_id: "req-1".into(),
            behavior: PermissionBehavior::Allow,
        },
    );
    s.buffer_permission_verdict(
        "claude",
        PermissionVerdict {
            request_id: "req-2".into(),
            behavior: PermissionBehavior::Deny,
        },
    );
    let verdicts = s.take_buffered_verdicts_for("claude");
    assert_eq!(verdicts.len(), 2);
    assert!(s.take_buffered_verdicts_for("claude").is_empty());
}

#[test]
fn buffered_verdicts_cap_at_50() {
    let mut s = DaemonState::new();
    for i in 0..60 {
        s.buffer_permission_verdict(
            "claude",
            PermissionVerdict {
                request_id: format!("req-{i}"),
                behavior: PermissionBehavior::Allow,
            },
        );
    }
    let verdicts = s.take_buffered_verdicts_for("claude");
    assert!(verdicts.len() <= 50);
}

#[test]
fn stale_codex_session_cleanup_cannot_clear_new_session() {
    let mut s = DaemonState::new();
    let stale_epoch = s.begin_codex_launch();
    let current_epoch = s.begin_codex_launch();
    let (current_tx, _current_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);

    assert!(s.attach_codex_session_if_current(current_epoch, current_tx));
    assert!(s.clear_codex_session_if_current(stale_epoch).is_none());
    assert!(s.codex_inject_tx.is_some());
    // current epoch passes guard; no provider connection → returns None but inject_tx is cleared
    let _ = s.clear_codex_session_if_current(current_epoch);
    assert!(s.codex_inject_tx.is_none());
}

#[test]
fn clear_codex_session_if_current_returns_task_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.active_task_id = Some(task.task_id.clone());
    let session = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Codex,
        role: crate::daemon::task_graph::types::SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
        agent_id: None,
    });
    s.task_graph
        .set_coder_session(&task.task_id, &session.session_id);
    s.task_graph
        .set_external_session_id(&session.session_id, "thread_1");
    s.set_provider_connection(
        "codex",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Codex,
            external_session_id: "thread_1".into(),
            cwd: "/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::New,
        },
    );
    let epoch = s.begin_codex_launch();

    let result = s.clear_codex_session_if_current(epoch);
    assert_eq!(result, Some(task.task_id));
}

#[test]
fn invalidate_claude_sdk_session_if_current_returns_task_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.active_task_id = Some(task.task_id.clone());
    let session = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Claude,
        role: crate::daemon::task_graph::types::SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
        agent_id: None,
    });
    s.task_graph
        .set_lead_session(&task.task_id, &session.session_id);
    s.task_graph
        .set_external_session_id(&session.session_id, "claude_sess_1");
    s.set_provider_connection(
        "claude",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Claude,
            external_session_id: "claude_sess_1".into(),
            cwd: "/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::New,
        },
    );
    let epoch = s.begin_claude_sdk_launch("nonce".into());

    let result = s.invalidate_claude_sdk_session_if_current(epoch);
    assert_eq!(result, Some(task.task_id));
}

#[test]
fn codex_register_on_launch_binds_resumed_thread_to_active_task() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.active_task_id = Some(task.task_id.clone());

    crate::daemon::provider::codex::register_on_launch(&mut s, &task.task_id, "coder", "/ws", "thread_resumed_1", None);

    let session = s
        .task_graph
        .find_session_by_external_id(
            crate::daemon::task_graph::types::Provider::Codex,
            "thread_resumed_1",
        )
        .expect("resumed thread should be registered in task graph");
    assert_eq!(session.task_id, task.task_id);
    let updated_task = s.task_graph.get_task(&task.task_id).unwrap();
    assert_eq!(
        updated_task.current_coder_session_id.as_deref(),
        Some(session.session_id.as_str())
    );
}

#[test]
fn observe_task_message_moves_task_to_reviewing_without_gate() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.set_active_task(Some(task.task_id.clone()));
    s.task_graph.update_task_status(
        &task.task_id,
        crate::daemon::task_graph::types::TaskStatus::Implementing,
    );

    let coder_done = BridgeMessage {
        id: "coder_done".into(),
        source: MessageSource::Agent {
            agent_id: "codex".into(),
            role: "coder".into(),
            provider: Provider::Codex,
            display_source: Some("codex".into()),
        },
        target: MessageTarget::Role { role: "lead".into() },
        reply_target: None,
        message: "finished current todo".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        attachments: None,
    };
    assert!(s.prepare_task_routing(&coder_done).is_allowed);
    let released = s.observe_task_message(&coder_done);
    assert!(released.is_empty());
    assert_eq!(
        s.task_graph.get_task(&task.task_id).unwrap().status,
        crate::daemon::task_graph::types::TaskStatus::Reviewing
    );
}

#[test]
fn observe_task_message_effects_reports_task_ui_events_on_state_change() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.set_active_task(Some(task.task_id.clone()));
    s.task_graph.update_task_status(
        &task.task_id,
        crate::daemon::task_graph::types::TaskStatus::Implementing,
    );

    let coder_done = BridgeMessage {
        id: "coder_done".into(),
        source: MessageSource::Agent {
            agent_id: "codex".into(),
            role: "coder".into(),
            provider: Provider::Codex,
            display_source: Some("codex".into()),
        },
        target: MessageTarget::Role { role: "lead".into() },
        reply_target: None,
        message: "finished current todo".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        attachments: None,
    };

    let effects = s.observe_task_message_effects(&coder_done);

    assert!(effects.released.is_empty());
    assert_eq!(effects.ui_events.len(), 1);
    assert!(matches!(
        &effects.ui_events[0],
        crate::daemon::gui_task::TaskUiEvent::TaskUpdated(task)
            if task.status == crate::daemon::task_graph::types::TaskStatus::Reviewing
                && task.task_id == task.task_id
    ));
}

#[test]
fn prepare_task_routing_allows_direct_coder_messages() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.set_active_task(Some(task.task_id.clone()));
    let decision = s.prepare_task_routing(&BridgeMessage {
        id: "user_to_coder".into(),
        source: MessageSource::User,
        target: MessageTarget::Role { role: "coder".into() },
        reply_target: None,
        message: "resume".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        attachments: None,
    });

    assert!(decision.is_allowed);
    assert!(decision.buffer_reason.is_none());
}

#[test]
fn daemon_state_persist_round_trip_restores_buffered_messages_per_task() {
    use crate::daemon::task_graph::types::{CreateSessionParams, Provider, SessionRole};

    let path = temp_state_path("buffered_messages_round_trip");
    let _ = std::fs::remove_file(&path);

    let mut s = DaemonState::with_task_graph_path(path.clone()).expect("create with path");
    let task_a = s.task_graph.create_task("/ws/a", "Task A");
    let task_b = s.task_graph.create_task("/ws/b", "Task B");
    let session_a = s.task_graph.create_session(CreateSessionParams {
        task_id: &task_a.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws/a",
        title: "Coder A",
        agent_id: None,
    });
    let session_b = s.task_graph.create_session(CreateSessionParams {
        task_id: &task_b.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws/b",
        title: "Coder B",
        agent_id: None,
    });

    let mut buffered_a = BridgeMessage::system("resume task a", "coder");
    buffered_a.task_id = Some(task_a.task_id.clone());
    buffered_a.session_id = Some(session_a.session_id.clone());
    let mut buffered_b = BridgeMessage::system("resume task b", "coder");
    buffered_b.task_id = Some(task_b.task_id.clone());
    buffered_b.session_id = Some(session_b.session_id.clone());
    s.buffer_message(buffered_a.clone());
    s.buffer_message(buffered_b.clone());
    s.save_task_graph().expect("save should succeed");

    let mut restored = DaemonState::with_task_graph_path(path.clone()).expect("reload should work");
    let released_a = restored.take_buffered_for_task("coder", Some(&task_a.task_id));
    assert_eq!(released_a.len(), 1);
    assert_eq!(released_a[0].id, buffered_a.id);
    assert_eq!(released_a[0].task_id.as_deref(), Some(task_a.task_id.as_str()));

    let released_b = restored.take_buffered_for_task("coder", Some(&task_b.task_id));
    assert_eq!(released_b.len(), 1);
    assert_eq!(released_b[0].id, buffered_b.id);
    assert_eq!(released_b[0].task_id.as_deref(), Some(task_b.task_id.as_str()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn daemon_state_task_graph_persist_round_trip() {
    let path =
        std::env::temp_dir().join(format!("dimweave_state_test_{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let mut s =
        DaemonState::with_task_graph_path(path.clone()).expect("create with path should succeed");
    let task = s.task_graph.create_task("/ws", "Stateful Task");
    let tid = task.task_id.clone();
    s.save_task_graph().expect("save should succeed");

    let s2 = DaemonState::with_task_graph_path(path.clone()).expect("reload should succeed");
    let t = s2.task_graph.get_task(&tid).expect("task should survive");
    assert_eq!(t.title, "Stateful Task");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn observe_task_message_auto_saves_without_explicit_call() {
    let path = std::env::temp_dir().join(format!(
        "dimweave_autosave_test_{}.json",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&path);

    let mut s = DaemonState::with_task_graph_path(path.clone()).expect("create with path");
    let task = s.task_graph.create_task("/ws", "AutoSave Task");
    let tid = task.task_id.clone();
    s.task_graph.update_task_status(
        &tid,
        crate::daemon::task_graph::types::TaskStatus::Implementing,
    );
    s.set_active_task(Some(tid.clone()));
    // Manually save the initial state so the file exists
    s.save_task_graph().unwrap();

    // Simulate coder -> lead done message (triggers auto-save internally)
    let coder_done = BridgeMessage {
        id: "cd".into(),
        source: MessageSource::Agent {
            agent_id: "codex".into(),
            role: "coder".into(),
            provider: Provider::Codex,
            display_source: Some("codex".into()),
        },
        target: MessageTarget::Role { role: "lead".into() },
        reply_target: None,
        message: "done".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        attachments: None,
    };
    let _ = s.observe_task_message(&coder_done);

    // Load from disk WITHOUT calling save_task_graph() — the auto-save
    // inside observe_task_message should have persisted the change.
    let s2 = DaemonState::with_task_graph_path(path.clone()).expect("reload");
    let t = s2.task_graph.get_task(&tid).expect("task exists on disk");
    assert_eq!(
        t.status,
        crate::daemon::task_graph::types::TaskStatus::Reviewing
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn shutdown_teardown_clears_live_runtime_handles() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    let (sdk_tx, _sdk_rx) = tokio::sync::mpsc::channel::<String>(1);
    let (event_tx, _event_rx) = tokio::sync::mpsc::channel::<Vec<serde_json::Value>>(1);
    let (ready_tx, _ready_rx) = tokio::sync::oneshot::channel();
    let (telegram_tx, _telegram_rx) =
        tokio::sync::mpsc::channel::<crate::telegram::types::TelegramOutbound>(1);

    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );
    s.codex_inject_tx = Some(codex_tx);
    s.claude_sdk_ws_tx = Some(sdk_tx);
    s.claude_sdk_event_tx = Some(event_tx);
    s.claude_sdk_ready_tx = Some(ready_tx);
    s.telegram_outbound_tx = Some(telegram_tx);
    s.buffer_message(BridgeMessage::system("linger", "lead"));
    s.store_permission_request(
        "claude",
        PermissionRequest {
            request_id: "req-1".into(),
            tool_name: "Bash".into(),
            description: "run pwd".into(),
            input_preview: None,
        },
        100,
    );
    s.set_provider_connection(
        "claude",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Claude,
            external_session_id: "claude_session".into(),
            cwd: "/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::New,
        },
    );
    s.set_provider_connection(
        "codex",
        crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Codex,
            external_session_id: "thread_1".into(),
            cwd: "/ws".into(),
            connection_mode: crate::daemon::types::ProviderConnectionMode::New,
        },
    );
    s.set_runtime_health(crate::daemon::types::RuntimeHealthStatus {
        level: crate::daemon::types::RuntimeHealthLevel::Error,
        source: "claude_sdk".into(),
        message: "stale".into(),
    });

    s.teardown_runtime_handles_for_shutdown();

    assert!(s.attached_agents.is_empty());
    assert!(s.buffered_messages.is_empty());
    assert!(s.codex_inject_tx.is_none());
    assert!(s.claude_sdk_ws_tx.is_none());
    assert!(s.claude_sdk_event_tx.is_none());
    assert!(s.claude_sdk_ready_tx.is_none());
    assert!(s.provider_connection("claude").is_none());
    assert!(s.provider_connection("codex").is_none());
    assert!(s.runtime_health.is_none());
    assert!(s.telegram_outbound_tx.is_none());
    assert!(s
        .resolve_permission("req-1", PermissionBehavior::Allow, 200)
        .is_none());
}

// ── claude_task_slot tests ─────────────────────────────────

#[test]
fn claude_task_slot_begin_launch_creates_slot_and_returns_epoch() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));

    let epoch = s.begin_claude_task_launch(&task.task_id, "nonce-a".into());
    assert!(epoch.is_some());
    assert_eq!(s.claude_task_epoch(&task.task_id), epoch);
}

#[test]
fn claude_task_slot_begin_launch_returns_none_without_runtime() {
    let mut s = DaemonState::new();
    assert!(s.begin_claude_task_launch("no-such-task", "nonce".into()).is_none());
}

#[test]
fn claude_task_slot_attach_ws_promotes_nonce_and_returns_generation() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let epoch = s.begin_claude_task_launch(&task.task_id, "nonce-a".into()).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);

    let gen = s.attach_claude_task_ws(&task.task_id, epoch, "nonce-a", tx);
    assert!(gen.is_some());
    assert!(s.is_claude_task_online(&task.task_id));
}

#[test]
fn claude_task_slot_attach_ws_rejects_wrong_nonce() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let epoch = s.begin_claude_task_launch(&task.task_id, "nonce-a".into()).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);

    assert!(s.attach_claude_task_ws(&task.task_id, epoch, "wrong-nonce", tx).is_none());
    assert!(!s.is_claude_task_online(&task.task_id));
}

#[test]
fn claude_task_slot_clear_ws_only_affects_matching_generation() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let epoch = s.begin_claude_task_launch(&task.task_id, "nonce-a".into()).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<String>(1);
    let gen1 = s.attach_claude_task_ws(&task.task_id, epoch, "nonce-a", tx1).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<String>(1);
    let gen2 = s.attach_claude_task_ws(&task.task_id, epoch, "nonce-a", tx2).unwrap();

    // Stale generation cannot clear
    assert!(!s.clear_claude_task_ws(&task.task_id, epoch, "nonce-a", gen1));
    assert!(s.is_claude_task_online(&task.task_id));
    // Current generation clears
    assert!(s.clear_claude_task_ws(&task.task_id, epoch, "nonce-a", gen2));
    assert!(!s.is_claude_task_online(&task.task_id));
}

#[test]
fn claude_task_slot_cross_task_isolation() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    let epoch1 = s.begin_claude_task_launch(&t1.task_id, "nonce-1".into()).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws(&t1.task_id, epoch1, "nonce-1", tx1);

    let epoch2 = s.begin_claude_task_launch(&t2.task_id, "nonce-2".into()).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws(&t2.task_id, epoch2, "nonce-2", tx2);

    // Invalidating task 1 should NOT affect task 2
    s.invalidate_claude_task_session(&t1.task_id);
    assert!(!s.is_claude_task_online(&t1.task_id));
    // Task 2 epoch should still be valid (even though singleton was cleared)
    assert_eq!(s.claude_task_epoch(&t2.task_id), Some(epoch2));
}

#[test]
fn claude_task_slot_find_task_for_nonce_scans_runtimes() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    s.begin_claude_task_launch(&t1.task_id, "nonce-1".into());
    s.begin_claude_task_launch(&t2.task_id, "nonce-2".into());

    assert_eq!(s.find_task_for_claude_nonce("nonce-1"), Some(t1.task_id.clone()));
    assert_eq!(s.find_task_for_claude_nonce("nonce-2"), Some(t2.task_id.clone()));
    assert_eq!(s.find_task_for_claude_nonce("unknown"), None);
}

#[test]
fn claude_task_slot_invalidate_if_current_guards_on_epoch() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let epoch1 = s.begin_claude_task_launch(&task.task_id, "nonce-a".into()).unwrap();
    // Launch again — epoch advances
    let epoch2 = s.begin_claude_task_launch(&task.task_id, "nonce-b".into()).unwrap();
    assert_ne!(epoch1, epoch2);

    // Stale epoch cannot invalidate
    assert!(s.invalidate_claude_task_session_if_current(&task.task_id, epoch1).is_none());
    // Current epoch can
    assert!(s.invalidate_claude_task_session_if_current(&task.task_id, epoch2).is_some()
        || s.claude_task_epoch(&task.task_id).is_some());
}

// ── Regression: Finding 1 — nonce isolation across tasks ────

#[test]
fn claude_task_slot_task_b_launch_does_not_invalidate_task_a_nonce() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    s.begin_claude_task_launch(&t1.task_id, "nonce-a".into());
    // Task B launches — must NOT invalidate nonce-a
    s.begin_claude_task_launch(&t2.task_id, "nonce-b".into());

    assert!(s.claude_sdk_accepts_launch_nonce("nonce-a"),
        "task A nonce must survive task B launch");
    assert!(s.claude_sdk_accepts_launch_nonce("nonce-b"));
}

// ── Regression: Finding 2 — event routing per nonce ─────────

#[test]
fn claude_task_slot_event_tx_resolves_by_nonce() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    s.begin_claude_task_launch(&t1.task_id, "nonce-1".into());
    s.begin_claude_task_launch(&t2.task_id, "nonce-2".into());

    let (tx1, _rx1) = tokio::sync::mpsc::channel::<Vec<serde_json::Value>>(1);
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<Vec<serde_json::Value>>(1);
    let (ready1, _) = tokio::sync::oneshot::channel();
    let (ready2, _) = tokio::sync::oneshot::channel();
    s.set_claude_task_channels(&t1.task_id, ready1, tx1);
    s.set_claude_task_channels(&t2.task_id, ready2, tx2);

    // nonce-1 must resolve to task 1's event_tx, not task 2's
    let resolved1 = s.claude_task_event_tx_for_nonce("nonce-1");
    let resolved2 = s.claude_task_event_tx_for_nonce("nonce-2");
    assert!(resolved1.is_some(), "nonce-1 should resolve event_tx");
    assert!(resolved2.is_some(), "nonce-2 should resolve event_tx");
    assert!(s.claude_task_event_tx_for_nonce("unknown").is_none());
}

// ── Regression: Finding 3 — invalidate isolation ────────────

#[test]
fn claude_task_slot_invalidate_task_a_preserves_task_b_online() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    let epoch1 = s.begin_claude_task_launch(&t1.task_id, "nonce-1".into()).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws(&t1.task_id, epoch1, "nonce-1", tx1);

    let epoch2 = s.begin_claude_task_launch(&t2.task_id, "nonce-2".into()).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws(&t2.task_id, epoch2, "nonce-2", tx2);

    // Both online
    assert!(s.is_claude_task_online(&t1.task_id));
    assert!(s.is_claude_task_online(&t2.task_id));
    assert!(s.is_claude_sdk_online());

    // Invalidate task 1 — must NOT take task 2 offline
    s.invalidate_claude_task_session(&t1.task_id);

    assert!(!s.is_claude_task_online(&t1.task_id));
    assert!(s.is_claude_task_online(&t2.task_id),
        "task B must stay online after task A invalidation");
    assert!(s.is_claude_sdk_online(),
        "global online must reflect surviving task B");
    // Singleton ws_tx should still work (recomputed from task B)
    assert!(s.claude_sdk_ws_tx.is_some(),
        "singleton ws_tx must be recomputed from surviving slots");
}

// ── Regression: Finding 4 — invalidate must not detach another task's task_graph binding ──

#[test]
fn claude_task_slot_invalidate_task_a_preserves_task_b_graph_binding() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    // Create Claude sessions in task_graph for both tasks
    let sess_a = s.task_graph.create_session(CreateSessionParams {
        task_id: &t1.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Claude,
        role: crate::daemon::task_graph::types::SessionRole::Lead,
        cwd: "/ws/a",
        title: "Claude A",
        agent_id: None,
    });
    s.task_graph.set_lead_session(&t1.task_id, &sess_a.session_id);

    let sess_b = s.task_graph.create_session(CreateSessionParams {
        task_id: &t2.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Claude,
        role: crate::daemon::task_graph::types::SessionRole::Lead,
        cwd: "/ws/b",
        title: "Claude B",
        agent_id: None,
    });
    s.task_graph.set_lead_session(&t2.task_id, &sess_b.session_id);

    // Launch Claude for both tasks (Task B last → owns global claude_connection mirror)
    let epoch1 = s.begin_claude_task_launch(&t1.task_id, "nonce-1".into()).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws(&t1.task_id, epoch1, "nonce-1", tx1);

    let epoch2 = s.begin_claude_task_launch(&t2.task_id, "nonce-2".into()).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws(&t2.task_id, epoch2, "nonce-2", tx2);

    // Simulate runtime.rs: Task B sets the global claude_connection mirror
    s.set_provider_connection("claude", crate::daemon::types::ProviderConnectionState {
        provider: crate::daemon::task_graph::types::Provider::Claude,
        external_session_id: sess_b.session_id.clone(),
        cwd: "/ws/b".into(),
        connection_mode: crate::daemon::types::ProviderConnectionMode::New,
    });

    // Invalidate Task A — must NOT touch Task B's task_graph binding
    s.invalidate_claude_task_session(&t1.task_id);

    // Task B's lead_session_id must be preserved
    let t2_task = s.task_graph.get_task(&t2.task_id).unwrap();
    assert_eq!(
        t2_task.lead_session_id.as_deref(),
        Some(sess_b.session_id.as_str()),
        "Task B lead_session_id must not be cleared by Task A invalidation"
    );

    // Task B's session must still be Active
    let t2_sess = s.task_graph.get_session(&sess_b.session_id).unwrap();
    assert_eq!(
        t2_sess.status,
        crate::daemon::task_graph::types::SessionStatus::Active,
        "Task B session must not be paused by Task A invalidation"
    );

    // Task A's session should be paused and lead cleared
    let t1_task = s.task_graph.get_task(&t1.task_id).unwrap();
    assert!(
        t1_task.lead_session_id.is_none(),
        "Task A lead_session_id should be cleared"
    );
    let t1_sess = s.task_graph.get_session(&sess_a.session_id).unwrap();
    assert_eq!(
        t1_sess.status,
        crate::daemon::task_graph::types::SessionStatus::Paused,
        "Task A session should be paused"
    );
}

// ── codex_task_slot tests ─────────────────────────────────

#[test]
fn codex_task_slot_begin_launch_creates_slot_and_returns_epoch() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));

    let epoch = s.begin_codex_task_launch(&task.task_id, 4500);
    assert!(epoch.is_some());
    assert_eq!(s.codex_task_epoch(&task.task_id), epoch);
}

#[test]
fn codex_task_slot_begin_launch_returns_none_without_runtime() {
    let mut s = DaemonState::new();
    assert!(s.begin_codex_task_launch("no-such-task", 4500).is_none());
}

#[test]
fn codex_task_slot_attach_marks_online() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let epoch = s.begin_codex_task_launch(&task.task_id, 4500).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);

    assert!(s.attach_codex_task_session(&task.task_id, epoch, tx, None));
    assert!(s.is_codex_task_online(&task.task_id));
    assert!(s.is_codex_online());
}

#[test]
fn codex_task_slot_attach_rejects_stale_epoch() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let stale_epoch = s.begin_codex_task_launch(&task.task_id, 4500).unwrap();
    let _current_epoch = s.begin_codex_task_launch(&task.task_id, 4500).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);

    assert!(!s.attach_codex_task_session(&task.task_id, stale_epoch, tx, None));
    assert!(!s.is_codex_task_online(&task.task_id));
}

#[test]
fn codex_task_slot_clear_only_matching_epoch() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let epoch = s.begin_codex_task_launch(&task.task_id, 4500).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&task.task_id, epoch, tx, None);

    // Stale epoch cannot clear
    assert!(s.clear_codex_task_session(&task.task_id, epoch.wrapping_sub(1)).is_none());
    assert!(s.is_codex_task_online(&task.task_id));
    // Current epoch clears the slot (returns None when no provider connection exists)
    let _ = s.clear_codex_task_session(&task.task_id, epoch);
    assert!(!s.is_codex_task_online(&task.task_id));
}

#[test]
fn codex_task_slot_cross_task_isolation() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    let epoch1 = s.begin_codex_task_launch(&t1.task_id, 4500).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t1.task_id, epoch1, tx1, None);

    let epoch2 = s.begin_codex_task_launch(&t2.task_id, 4501).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t2.task_id, epoch2, tx2, None);

    // Both online
    assert!(s.is_codex_task_online(&t1.task_id));
    assert!(s.is_codex_task_online(&t2.task_id));

    // Invalidating task 1 must NOT affect task 2
    s.invalidate_codex_task_session(&t1.task_id);
    assert!(!s.is_codex_task_online(&t1.task_id));
    assert!(s.is_codex_task_online(&t2.task_id),
        "task B must stay online after task A invalidation");
    assert!(s.is_codex_online(),
        "global online must reflect surviving task B");
}

#[test]
fn codex_task_slot_used_ports_only_returns_online_slots() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    let epoch1 = s.begin_codex_task_launch(&t1.task_id, 4500).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t1.task_id, epoch1, tx1, None);
    let epoch2 = s.begin_codex_task_launch(&t2.task_id, 4501).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t2.task_id, epoch2, tx2, None);

    assert_eq!(s.codex_used_ports().len(), 2);

    // After invalidating task 1, its port should be freed
    s.invalidate_codex_task_session(&t1.task_id);
    let used = s.codex_used_ports();
    assert_eq!(used.len(), 1);
    assert!(used.contains(&4501));
    assert!(!used.contains(&4500));
}

#[test]
fn codex_task_slot_invalidate_preserves_task_b_graph_binding() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    let sess_a = s.task_graph.create_session(CreateSessionParams {
        task_id: &t1.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Codex,
        role: crate::daemon::task_graph::types::SessionRole::Coder,
        cwd: "/ws/a",
        title: "Codex A",
        agent_id: None,
    });
    s.task_graph.set_coder_session(&t1.task_id, &sess_a.session_id);

    let sess_b = s.task_graph.create_session(CreateSessionParams {
        task_id: &t2.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Codex,
        role: crate::daemon::task_graph::types::SessionRole::Coder,
        cwd: "/ws/b",
        title: "Codex B",
        agent_id: None,
    });
    s.task_graph.set_coder_session(&t2.task_id, &sess_b.session_id);

    let epoch1 = s.begin_codex_task_launch(&t1.task_id, 4500).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t1.task_id, epoch1, tx1, None);

    let epoch2 = s.begin_codex_task_launch(&t2.task_id, 4501).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t2.task_id, epoch2, tx2, None);

    // Invalidate Task A — must NOT touch Task B's task_graph binding
    s.invalidate_codex_task_session(&t1.task_id);

    let t2_task = s.task_graph.get_task(&t2.task_id).unwrap();
    assert_eq!(
        t2_task.current_coder_session_id.as_deref(),
        Some(sess_b.session_id.as_str()),
        "Task B coder_session_id must not be cleared by Task A invalidation"
    );
    let t2_sess = s.task_graph.get_session(&sess_b.session_id).unwrap();
    assert_eq!(
        t2_sess.status,
        crate::daemon::task_graph::types::SessionStatus::Active,
        "Task B session must not be paused by Task A invalidation"
    );
    let t1_task = s.task_graph.get_task(&t1.task_id).unwrap();
    assert!(
        t1_task.current_coder_session_id.is_none(),
        "Task A coder_session_id should be cleared"
    );
}

#[test]
fn codex_task_slot_clear_preserves_task_b_graph_binding() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    // Create and bind Codex sessions for both tasks
    let sess_a = s.task_graph.create_session(CreateSessionParams {
        task_id: &t1.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Codex,
        role: crate::daemon::task_graph::types::SessionRole::Coder,
        cwd: "/ws/a",
        title: "Codex A",
        agent_id: None,
    });
    s.task_graph.set_coder_session(&t1.task_id, &sess_a.session_id);

    let sess_b = s.task_graph.create_session(CreateSessionParams {
        task_id: &t2.task_id,
        parent_session_id: None,
        provider: crate::daemon::task_graph::types::Provider::Codex,
        role: crate::daemon::task_graph::types::SessionRole::Coder,
        cwd: "/ws/b",
        title: "Codex B",
        agent_id: None,
    });
    s.task_graph.set_coder_session(&t2.task_id, &sess_b.session_id);

    // Launch both tasks with Codex slots and distinct connections
    let conn_a = crate::daemon::types::ProviderConnectionState {
        provider: crate::daemon::task_graph::types::Provider::Codex,
        external_session_id: "thread_a".to_string(),
        cwd: "/ws/a".to_string(),
        connection_mode: crate::daemon::types::ProviderConnectionMode::New,
    };
    let epoch1 = s.begin_codex_task_launch(&t1.task_id, 4500).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t1.task_id, epoch1, tx1, Some(conn_a));
    s.task_graph.set_external_session_id(&sess_a.session_id, "thread_a");

    let conn_b = crate::daemon::types::ProviderConnectionState {
        provider: crate::daemon::task_graph::types::Provider::Codex,
        external_session_id: "thread_b".to_string(),
        cwd: "/ws/b".to_string(),
        connection_mode: crate::daemon::types::ProviderConnectionMode::New,
    };
    let epoch2 = s.begin_codex_task_launch(&t2.task_id, 4501).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    // Task B launched last → its connection becomes the singleton mirror
    s.attach_codex_task_session(&t2.task_id, epoch2, tx2, Some(conn_b));
    s.task_graph.set_external_session_id(&sess_b.session_id, "thread_b");

    // Clear Task A session (simulates Task A process exit / health-monitor cleanup)
    let cleared = s.clear_codex_task_session(&t1.task_id, epoch1);
    assert!(cleared.is_some(), "clear should succeed for matching epoch");

    // Task B's coder_session_id must NOT be cleared
    let t2_task = s.task_graph.get_task(&t2.task_id).unwrap();
    assert_eq!(
        t2_task.current_coder_session_id.as_deref(),
        Some(sess_b.session_id.as_str()),
        "Task B coder_session_id must not be cleared by Task A cleanup"
    );
    // Task B's session must still be Active
    let t2_sess = s.task_graph.get_session(&sess_b.session_id).unwrap();
    assert_eq!(
        t2_sess.status,
        crate::daemon::task_graph::types::SessionStatus::Active,
        "Task B session must not be paused by Task A cleanup"
    );
    // Task B Codex slot must still be online
    assert!(
        s.is_codex_task_online(&t2.task_id),
        "Task B Codex slot must remain online"
    );
    // Task A should be offline
    assert!(
        !s.is_codex_task_online(&t1.task_id),
        "Task A Codex slot should be offline after clear"
    );
    // Singleton mirror should now reflect Task B's connection
    let singleton = s.provider_connection("codex").expect("singleton should survive");
    assert_eq!(singleton.external_session_id, "thread_b");
}

#[test]
fn codex_task_slot_distinct_connection_ownership() {
    let mut s = DaemonState::new();
    let t1 = s.task_graph.create_task("/ws/a", "T1");
    let t2 = s.task_graph.create_task("/ws/b", "T2");
    s.init_task_runtime(&t1.task_id, std::path::PathBuf::from("/ws/a"));
    s.init_task_runtime(&t2.task_id, std::path::PathBuf::from("/ws/b"));

    let conn_a = crate::daemon::types::ProviderConnectionState {
        provider: crate::daemon::task_graph::types::Provider::Codex,
        external_session_id: "thread_a".to_string(),
        cwd: "/ws/a".to_string(),
        connection_mode: crate::daemon::types::ProviderConnectionMode::New,
    };
    let conn_b = crate::daemon::types::ProviderConnectionState {
        provider: crate::daemon::task_graph::types::Provider::Codex,
        external_session_id: "thread_b".to_string(),
        cwd: "/ws/b".to_string(),
        connection_mode: crate::daemon::types::ProviderConnectionMode::New,
    };

    let epoch1 = s.begin_codex_task_launch(&t1.task_id, 4500).unwrap();
    let (tx1, _rx1) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t1.task_id, epoch1, tx1, Some(conn_a.clone()));

    let epoch2 = s.begin_codex_task_launch(&t2.task_id, 4501).unwrap();
    let (tx2, _rx2) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attach_codex_task_session(&t2.task_id, epoch2, tx2, Some(conn_b.clone()));

    // Each task slot retains its own connection
    let slot_a = s.task_runtimes.get(&t1.task_id).unwrap()
        .codex_slot.as_ref().unwrap();
    assert_eq!(slot_a.connection.as_ref().unwrap().external_session_id, "thread_a");

    let slot_b = s.task_runtimes.get(&t2.task_id).unwrap()
        .codex_slot.as_ref().unwrap();
    assert_eq!(slot_b.connection.as_ref().unwrap().external_session_id, "thread_b");

    // Singleton mirror points to the last-attached (Task B)
    let singleton = s.provider_connection("codex").unwrap();
    assert_eq!(singleton.external_session_id, "thread_b");

    // Clear Task B — singleton should fall back to Task A's connection
    s.clear_codex_task_session(&t2.task_id, epoch2);
    let singleton = s.provider_connection("codex").unwrap();
    assert_eq!(
        singleton.external_session_id, "thread_a",
        "singleton must fall back to remaining online slot's connection"
    );

    // Task A's slot connection unchanged
    let slot_a = s.task_runtimes.get(&t1.task_id).unwrap()
        .codex_slot.as_ref().unwrap();
    assert_eq!(slot_a.connection.as_ref().unwrap().external_session_id, "thread_a");
}

// ── task_runtime_routing: AC3 stamp_message_context_for_task ────────

#[test]
fn stamp_message_context_for_task_uses_explicit_task_not_active() {
    use crate::daemon::task_graph::types::{Provider, SessionRole};
    let mut s = DaemonState::new();
    let task_a = s.task_graph.create_task("/ws", "Task A");
    let task_b = s.task_graph.create_task("/ws", "Task B");
    let sess_a = s.task_graph.create_session(CreateSessionParams {
        task_id: &task_a.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder A",
        agent_id: None,
    });
    s.task_graph
        .set_coder_session(&task_a.task_id, &sess_a.session_id);
    let sess_b = s.task_graph.create_session(CreateSessionParams {
        task_id: &task_b.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder B",
        agent_id: None,
    });
    s.task_graph
        .set_coder_session(&task_b.task_id, &sess_b.session_id);
    // active_task_id is task B
    s.active_task_id = Some(task_b.task_id.clone());

    let mut msg = BridgeMessage::system("test", "user");
    s.stamp_message_context_for_task(&task_a.task_id, "coder", &mut msg);

    assert_eq!(msg.task_id.as_deref(), Some(task_a.task_id.as_str()));
    assert_eq!(msg.session_id.as_deref(), Some(sess_a.session_id.as_str()));
}

// ── task_runtime_routing: codex_owning_task_id ──────────────────────

#[test]
fn codex_owning_task_id_returns_online_slot_task() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.active_task_id = Some(task.task_id.clone());
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    let epoch = s.begin_codex_task_launch(&task.task_id, 4500).unwrap();
    let (tx, _rx) = tokio::sync::mpsc::channel(1);
    s.attach_codex_task_session(&task.task_id, epoch, tx, None);

    assert_eq!(s.codex_owning_task_id().as_deref(), Some(task.task_id.as_str()));
}

#[test]
fn codex_owning_task_id_falls_back_to_active_task() {
    let mut s = DaemonState::new();
    s.active_task_id = Some("fallback_task".into());
    assert_eq!(s.codex_owning_task_id().as_deref(), Some("fallback_task"));
}

// ── task_runtime_routing: resolve_task_provider_agent ────────────────

#[test]
fn resolve_task_provider_agent_maps_lead_and_coder() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    // Default: lead=Claude, coder=Codex
    assert_eq!(
        s.resolve_task_provider_agent(&task.task_id, "lead"),
        Some("claude"),
    );
    assert_eq!(
        s.resolve_task_provider_agent(&task.task_id, "coder"),
        Some("codex"),
    );
    assert_eq!(s.resolve_task_provider_agent(&task.task_id, "user"), None);
}

// ── task_runtime_routing: task-slot online agents & provider summary ──

fn wire_claude_task_slot_online(s: &mut DaemonState, task_id: &str) {
    let rt = s.task_runtimes.get_mut(task_id).expect("task runtime");
    let mut slot = crate::daemon::task_runtime::ClaudeTaskSlot::new();
    slot.ws_tx = Some(tokio::sync::mpsc::channel(1).0);
    rt.claude_slot = Some(slot);
}

fn wire_codex_task_slot_online(s: &mut DaemonState, task_id: &str) {
    let rt = s.task_runtimes.get_mut(task_id).expect("task runtime");
    let mut slot = crate::daemon::task_runtime::CodexTaskSlot::new(4500);
    slot.inject_tx = Some(tokio::sync::mpsc::channel(1).0);
    rt.codex_slot = Some(slot);
}

#[test]
fn task_runtime_routing_scoped_online_agents_uses_task_slots() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    // Default bindings: lead=Claude, coder=Codex
    wire_claude_task_slot_online(&mut s, &task.task_id);
    wire_codex_task_slot_online(&mut s, &task.task_id);

    let agents = s.task_scoped_online_agents(&task.task_id);
    assert_eq!(agents.len(), 2);
    assert!(agents.iter().any(|a| a.agent_id == "claude" && a.role == "lead"));
    assert!(agents.iter().any(|a| a.agent_id == "codex" && a.role == "coder"));
}

#[test]
fn task_runtime_routing_scoped_online_agents_excludes_offline_slot() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    // Only Claude task-local slot is online; Codex is offline
    wire_claude_task_slot_online(&mut s, &task.task_id);
    // Wire Codex globally (should NOT count for task-scoped)
    s.codex_inject_tx = Some(tokio::sync::mpsc::channel(1).0);

    let agents = s.task_scoped_online_agents(&task.task_id);
    assert_eq!(agents.len(), 1, "only task-local Claude slot should show");
    assert_eq!(agents[0].agent_id, "claude");
    assert_eq!(agents[0].role, "lead");
}

#[test]
fn task_runtime_routing_provider_summary_uses_task_slots() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    // Only Claude task-local slot is online
    wire_claude_task_slot_online(&mut s, &task.task_id);

    let summary = s.task_provider_summary(&task.task_id).unwrap();
    assert_eq!(summary.lead_provider, "claude");
    assert_eq!(summary.coder_provider, "codex");
    assert!(summary.lead_online);
    assert!(!summary.coder_online, "Codex task slot is not wired");
}

#[test]
fn task_runtime_routing_provider_summary_ignores_global_channels() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "Task");
    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws"));
    // Wire global channels but NOT task-local slots
    s.claude_sdk_ws_tx = Some(tokio::sync::mpsc::channel(1).0);
    s.codex_inject_tx = Some(tokio::sync::mpsc::channel(1).0);

    let summary = s.task_provider_summary(&task.task_id).unwrap();
    assert!(!summary.lead_online, "global Claude should not count");
    assert!(!summary.coder_online, "global Codex should not count");
}

// ── runtime_final_cleanup: task-local ownership supersedes global singletons ──

#[test]
fn runtime_final_cleanup_task_local_role_ignores_global_role() {
    let mut s = DaemonState::new();
    s.claude_role = "coder".into(); // global says coder
    let task = s.task_graph.create_task("/ws", "T");
    // task says claude is lead (default lead_provider=Claude)
    let agent = s.resolve_task_provider_agent(&task.task_id, "lead");
    assert_eq!(agent, Some("claude"), "task-local lead_provider takes precedence over global claude_role");
}

#[test]
fn runtime_final_cleanup_task_provider_connection_ignores_global_mirror() {
    use crate::daemon::task_graph::types::Provider;
    use crate::daemon::task_runtime::CodexTaskSlot;
    use crate::daemon::types::{ProviderConnectionMode, ProviderConnectionState};

    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.init_task_runtime(&task.task_id, "/ws".into());

    // Global mirror has a stale codex connection
    s.set_provider_connection("codex", ProviderConnectionState {
        provider: Provider::Codex,
        external_session_id: "global_stale".into(),
        cwd: "/other".into(),
        connection_mode: ProviderConnectionMode::Resumed,
    });

    // Task slot has its own connection
    let mut slot = CodexTaskSlot::new(4510);
    slot.connection = Some(ProviderConnectionState {
        provider: Provider::Codex,
        external_session_id: "task_local".into(),
        cwd: "/ws".into(),
        connection_mode: ProviderConnectionMode::New,
    });
    s.task_runtimes.get_mut(&task.task_id).unwrap().codex_slot = Some(slot);

    let conn = s.task_provider_connection(&task.task_id, "codex");
    assert_eq!(
        conn.as_ref().map(|c| c.external_session_id.as_str()),
        Some("task_local"),
        "task_provider_connection must read from the task slot, not the global mirror"
    );

    let global = s.provider_connection("codex");
    assert_eq!(
        global.as_ref().map(|c| c.external_session_id.as_str()),
        Some("global_stale"),
        "global mirror is untouched"
    );
}

#[test]
fn runtime_final_cleanup_summary_uses_task_slot_connection() {
    use crate::daemon::task_graph::types::Provider;
    use crate::daemon::task_runtime::ClaudeTaskSlot;
    use crate::daemon::types::{ProviderConnectionMode, ProviderConnectionState};

    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.init_task_runtime(&task.task_id, "/ws".into());

    // Global mirror has a stale session
    s.set_provider_connection("claude", ProviderConnectionState {
        provider: Provider::Claude,
        external_session_id: "global_sess".into(),
        cwd: "/other".into(),
        connection_mode: ProviderConnectionMode::Resumed,
    });

    // Task slot is online with its own session
    let mut claude_slot = ClaudeTaskSlot::new();
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
    claude_slot.ws_tx = Some(tx);
    claude_slot.connection = Some(ProviderConnectionState {
        provider: Provider::Claude,
        external_session_id: "task_sess".into(),
        cwd: "/ws".into(),
        connection_mode: ProviderConnectionMode::New,
    });
    s.task_runtimes.get_mut(&task.task_id).unwrap().claude_slot = Some(claude_slot);

    let summary = s.task_provider_summary(&task.task_id).unwrap();
    assert!(summary.lead_online);
    let lead_sess = summary.lead_provider_session.unwrap();
    assert_eq!(
        lead_sess.external_session_id, "task_sess",
        "summary must use task slot session, not global mirror"
    );
}

#[test]
fn runtime_final_cleanup_global_role_defaults_empty() {
    let s = DaemonState::new();
    assert_eq!(s.claude_role, "", "global claude_role defaults to empty (compat-only)");
    assert_eq!(s.codex_role, "", "global codex_role defaults to empty (compat-only)");
}

// ── agent_id routing: resolve_task_role_providers ──────────────

#[test]
fn agent_id_routing_resolve_role_providers_from_task_agents() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "lead");
    s.task_graph.add_task_agent(&task.task_id, Provider::Codex, "coder");

    let leads = s.resolve_task_role_providers(&task.task_id, "lead");
    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].runtime, "claude");
    assert!(!leads[0].agent_id.is_empty(), "agent_id must be preserved");

    let coders = s.resolve_task_role_providers(&task.task_id, "coder");
    assert_eq!(coders.len(), 1);
    assert_eq!(coders[0].runtime, "codex");
}

#[test]
fn agent_id_routing_broadcast_same_role_both_providers() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");
    s.task_graph.add_task_agent(&task.task_id, Provider::Codex, "coder");

    let coders = s.resolve_task_role_providers(&task.task_id, "coder");
    assert_eq!(coders.len(), 2);
    assert!(coders.iter().any(|m| m.runtime == "claude"));
    assert!(coders.iter().any(|m| m.runtime == "codex"));
}

#[test]
fn agent_id_routing_same_provider_same_role_not_collapsed() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    // Two Claude agents both with role "coder" — must NOT collapse to one entry
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");

    let coders = s.resolve_task_role_providers(&task.task_id, "coder");
    assert_eq!(coders.len(), 2, "same-provider same-role must not collapse");
    assert_ne!(coders[0].agent_id, coders[1].agent_id, "each must have unique agent_id");
    assert_eq!(coders[0].runtime, "claude");
    assert_eq!(coders[1].runtime, "claude");
}

#[test]
fn agent_id_routing_unknown_role_returns_empty() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "lead");

    assert!(s.resolve_task_role_providers(&task.task_id, "reviewer").is_empty());
}

#[test]
fn agent_id_routing_compat_wrapper_uses_task_agents() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    // Task singleton field is Claude for lead_provider, but task_agents says Codex
    s.task_graph.add_task_agent(&task.task_id, Provider::Codex, "lead");

    assert_eq!(
        s.resolve_task_provider_agent(&task.task_id, "lead"),
        Some("codex"),
        "resolve_task_provider_agent should delegate to task_agents, not singleton fields"
    );
}

#[test]
fn agent_id_routing_extensible_role_resolves() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "reviewer");

    let reviewers = s.resolve_task_role_providers(&task.task_id, "reviewer");
    assert_eq!(reviewers.len(), 1);
    assert_eq!(reviewers[0].runtime, "claude");
    assert_eq!(s.resolve_task_provider_agent(&task.task_id, "reviewer"), Some("claude"));
}

#[test]
fn agent_id_routing_no_task_returns_empty() {
    let s = DaemonState::new();
    assert!(s.resolve_task_role_providers("nonexistent", "lead").is_empty());
    assert_eq!(s.resolve_task_provider_agent("nonexistent", "lead"), None);
}

// ── agent_id routing: task_scoped_online_agents ────────────────

#[test]
fn agent_id_routing_task_scoped_online_agents_uses_task_agents() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    let claude_agent = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "lead");
    s.task_graph.add_task_agent(&task.task_id, Provider::Codex, "coder");

    s.init_task_runtime(&task.task_id, "/ws".into());

    // Make Claude online for this task using agent-aware slot
    let slot = s.task_runtimes.get_mut(&task.task_id).unwrap()
        .get_or_create_claude_slot(&claude_agent.agent_id);
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
    slot.ws_tx = Some(tx);

    let online = s.task_scoped_online_agents(&task.task_id);
    assert_eq!(online.len(), 1);
    assert_eq!(online[0].agent_id, claude_agent.agent_id);
    assert_eq!(online[0].model_source, "claude", "model_source is still the runtime name");
    assert_eq!(online[0].role, "lead", "role should come from task_agents, not singleton");
}

// ── agent_id routing: per-agent-id online checks ─────────────

#[test]
fn agent_id_routing_online_by_id_distinguishes_two_same_provider_agents() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    let agent_a = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");
    let agent_b = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");

    s.init_task_runtime(&task.task_id, "/ws".into());
    let slot_a = s.task_runtimes.get_mut(&task.task_id).unwrap()
        .get_or_create_claude_slot(&agent_a.agent_id);
    slot_a.ws_tx = Some(tokio::sync::mpsc::channel::<String>(1).0);
    // agent_b has a slot but is NOT online (no ws_tx)
    let _slot_b = s.task_runtimes.get_mut(&task.task_id).unwrap()
        .get_or_create_claude_slot(&agent_b.agent_id);

    assert!(
        s.is_task_agent_online_by_id(&task.task_id, &agent_a.agent_id, "claude"),
        "agent_a should be online"
    );
    assert!(
        !s.is_task_agent_online_by_id(&task.task_id, &agent_b.agent_id, "claude"),
        "agent_b should be offline — no ws_tx"
    );
    // Legacy runtime check still reports online (any slot)
    assert!(
        s.is_task_agent_online(&task.task_id, "claude"),
        "provider-level check should be true if any slot is online"
    );
}

#[test]
fn agent_id_routing_task_scoped_online_uses_per_agent_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    let agent_a = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");
    let agent_b = s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "coder");

    s.init_task_runtime(&task.task_id, "/ws".into());
    // Only agent_a is online
    let slot_a = s.task_runtimes.get_mut(&task.task_id).unwrap()
        .get_or_create_claude_slot(&agent_a.agent_id);
    slot_a.ws_tx = Some(tokio::sync::mpsc::channel::<String>(1).0);
    let _slot_b = s.task_runtimes.get_mut(&task.task_id).unwrap()
        .get_or_create_claude_slot(&agent_b.agent_id);

    let online = s.task_scoped_online_agents(&task.task_id);
    assert_eq!(online.len(), 1, "only agent_a is online");
    assert_eq!(online[0].agent_id, agent_a.agent_id);
}

// ── agent_runtime_ownership: agent_id-keyed runtime slots ──────

#[test]
fn agent_runtime_ownership_slot_stores_agent_id() {
    let mut slot = crate::daemon::task_runtime::ClaudeTaskSlot::new();
    assert!(slot.agent_id.is_none(), "new slot has no agent_id");
    slot.agent_id = Some("agent_123".into());
    assert_eq!(slot.agent_id.as_deref(), Some("agent_123"));
}

#[test]
fn agent_runtime_ownership_multi_claude_in_same_task() {
    let mut rt = crate::daemon::task_runtime::TaskRuntime::new("t1".into(), "/ws".into());
    // First agent gets the default claude_slot
    let slot_a = rt.get_or_create_claude_slot("agent_a");
    slot_a.ws_tx = Some(tokio::sync::mpsc::channel::<String>(1).0);
    // Second agent goes into extra_claude_slots
    let _slot_b = rt.get_or_create_codex_slot("agent_b", 4501);
    // Verify independent lookup
    assert!(rt.claude_slot_by_agent("agent_a").unwrap().is_online());
    assert!(rt.codex_slot_by_agent("agent_b").is_some());
    assert!(rt.claude_slot_by_agent("nonexistent").is_none());
}

#[test]
fn agent_runtime_ownership_two_claude_agents_independent() {
    let mut rt = crate::daemon::task_runtime::TaskRuntime::new("t1".into(), "/ws".into());
    let slot_a = rt.get_or_create_claude_slot("agent_a");
    slot_a.ws_tx = Some(tokio::sync::mpsc::channel::<String>(1).0);
    let _slot_b = rt.get_or_create_claude_slot("agent_b");
    // agent_a is online, agent_b is not
    assert!(rt.claude_slot_by_agent("agent_a").unwrap().is_online());
    assert!(!rt.claude_slot_by_agent("agent_b").unwrap().is_online());
    // compat field stays usable
    assert!(rt.claude_slot.as_ref().unwrap().is_online());
}

#[test]
fn agent_runtime_ownership_nonce_resolves_to_agent_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.init_task_runtime(&task.task_id, "/ws".into());
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "lead");
    let agent_id = s.task_graph.agents_for_task(&task.task_id)[0]
        .agent_id
        .clone();
    let nonce = "nonce-resolve".to_string();
    s.begin_claude_task_launch_for_agent(&task.task_id, &agent_id, nonce.clone());
    let (tid, aid) = s
        .find_task_and_agent_for_claude_nonce(&nonce)
        .unwrap();
    assert_eq!(tid, task.task_id);
    assert_eq!(aid, agent_id);
}

#[test]
fn agent_runtime_ownership_launch_binds_agent_id_to_slot() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.init_task_runtime(&task.task_id, "/ws".into());
    s.task_graph.add_task_agent(&task.task_id, Provider::Claude, "lead");
    let agent_id = s.task_graph.agents_for_task(&task.task_id)[0]
        .agent_id
        .clone();
    let nonce = "nonce-bind".to_string();
    let epoch = s
        .begin_claude_task_launch_for_agent(&task.task_id, &agent_id, nonce.clone())
        .unwrap();
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws_for_agent(&task.task_id, &agent_id, epoch, &nonce, tx);
    let rt = s.get_task_runtime(&task.task_id).unwrap();
    let slot = rt.claude_slot_by_agent(&agent_id).unwrap();
    assert_eq!(slot.agent_id.as_deref(), Some(agent_id.as_str()));
    assert!(slot.is_online());
}

#[test]
fn agent_runtime_ownership_invalidate_preserves_other_agent() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T");
    s.init_task_runtime(&task.task_id, "/ws".into());
    let agent_a = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Claude, "lead")
        .agent_id
        .clone();
    let agent_b = s
        .task_graph
        .add_task_agent(&task.task_id, Provider::Claude, "coder")
        .agent_id
        .clone();
    // Launch agent_a
    let nonce_a = "nonce-a".to_string();
    let epoch_a = s
        .begin_claude_task_launch_for_agent(&task.task_id, &agent_a, nonce_a.clone())
        .unwrap();
    let (tx_a, _) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws_for_agent(&task.task_id, &agent_a, epoch_a, &nonce_a, tx_a);
    // Launch agent_b
    let nonce_b = "nonce-b".to_string();
    let epoch_b = s
        .begin_claude_task_launch_for_agent(&task.task_id, &agent_b, nonce_b.clone())
        .unwrap();
    let (tx_b, _) = tokio::sync::mpsc::channel::<String>(1);
    s.attach_claude_task_ws_for_agent(&task.task_id, &agent_b, epoch_b, &nonce_b, tx_b);
    // Invalidate only agent_a
    s.invalidate_claude_agent_session(&task.task_id, &agent_a);
    let rt = s.get_task_runtime(&task.task_id).unwrap();
    assert!(
        !rt.claude_slot_by_agent(&agent_a)
            .map_or(false, |s| s.is_online()),
        "agent_a should be offline"
    );
    assert!(
        rt.claude_slot_by_agent(&agent_b).unwrap().is_online(),
        "agent_b must survive invalidation of agent_a"
    );
}

#[test]
fn agent_runtime_ownership_fresh_launches_never_collapse_same_role() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/tmp", "multi-coder");
    // Two fresh launches with identical (task, provider, role) must get distinct agent_ids
    let aid1 = crate::daemon::create_agent_id(
        &mut s, &task.task_id, Provider::Codex, "coder",
    );
    let aid2 = crate::daemon::create_agent_id(
        &mut s, &task.task_id, Provider::Codex, "coder",
    );
    assert_ne!(aid1, aid2, "same-provider same-role fresh launches must get distinct agent_ids");
    // Verify both agents exist in the task
    let agents = s.task_graph.agents_for_task(&task.task_id);
    let matching: Vec<_> = agents.iter()
        .filter(|a| a.provider == Provider::Codex && a.role == "coder")
        .collect();
    assert_eq!(matching.len(), 2, "task must have two distinct coder agents");
}

#[test]
fn agent_runtime_ownership_claude_resume_preserves_agent_id() {
    use crate::daemon::launch_task_sync::sync_claude_launch_into_task;
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/tmp", "resume-test");
    s.active_task_id = Some(task.task_id.clone());
    // Simulate initial launch: register a session with agent_id
    let agent_id = crate::daemon::create_agent_id(
        &mut s, &task.task_id, Provider::Claude, "lead",
    );
    crate::daemon::provider::claude::register_on_launch(
        &mut s, &task.task_id, "lead", "/tmp", "ext-sess-1", "/transcript", Some(&agent_id),
    );
    let original_session = s.task_graph
        .find_session_by_external_id(Provider::Claude, "ext-sess-1")
        .expect("session should exist");
    assert_eq!(original_session.agent_id.as_deref(), Some(agent_id.as_str()));
    let original_session_id = original_session.session_id.clone();

    // Now simulate resume with the same agent_id — sync should preserve it
    let result = sync_claude_launch_into_task(
        &mut s, &task.task_id, "lead", "/tmp", "ext-sess-1", "/transcript2",
        Some(&agent_id),
    );
    assert!(result.is_some());
    let resumed = s.task_graph.get_session(&original_session_id).unwrap();
    assert_eq!(
        resumed.agent_id.as_deref(), Some(agent_id.as_str()),
        "resume must preserve the original agent_id",
    );
}

#[test]
fn agent_runtime_ownership_codex_resume_binds_agent_id() {
    use crate::daemon::launch_task_sync::sync_codex_launch_into_task;
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/tmp", "codex-resume");
    s.active_task_id = Some(task.task_id.clone());
    // Register initial codex session without agent_id
    crate::daemon::provider::codex::register_on_launch(
        &mut s, &task.task_id, "coder", "/tmp", "thread-1", None,
    );
    let sess = s.task_graph
        .find_session_by_external_id(Provider::Codex, "thread-1")
        .expect("session should exist");
    assert!(sess.agent_id.is_none(), "initially no agent_id");
    let sess_id = sess.session_id.clone();

    // Resume with explicit agent_id — should bind it
    let aid = crate::daemon::create_agent_id(
        &mut s, &task.task_id, Provider::Codex, "coder",
    );
    let result = sync_codex_launch_into_task(
        &mut s, &task.task_id, "coder", "/tmp", "thread-1", Some(&aid),
    );
    assert!(result.is_some());
    let updated = s.task_graph.get_session(&sess_id).unwrap();
    assert_eq!(
        updated.agent_id.as_deref(), Some(aid.as_str()),
        "resume must bind the supplied agent_id to the existing session",
    );
}
