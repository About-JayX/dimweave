use super::*;
use crate::daemon::task_graph::types::CreateSessionParams;

fn temp_state_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "dimweave_state_{}_{}_{}.json",
        name,
        std::process::id(),
        chrono::Utc::now().timestamp_millis(),
    ))
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

#[test]
fn status_snapshot_reports_current_online_agents() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );
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
fn status_snapshot_includes_provider_session_metadata() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );
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
fn online_role_conflict_only_blocks_live_other_agent() {
    let mut s = DaemonState::new();
    s.claude_role = "lead".into();
    s.codex_role = "lead".into();
    assert_eq!(s.online_role_conflict("codex", "lead"), None);

    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(claude_tx, 0),
    );
    assert_eq!(s.online_role_conflict("codex", "lead"), Some("claude"));
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
    assert!(s.buffered_messages.iter().all(|m| m.to != "lead"));
    assert!(s.buffered_messages.iter().any(|m| m.to == "coder"));
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
    assert_eq!(s.buffered_messages[0].to, "coder");
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

    crate::daemon::provider::codex::register_on_launch(&mut s, "coder", "/ws", "thread_resumed_1");

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
        from: "coder".into(),
        display_source: Some("codex".into()),
        to: "lead".into(),
        content: "finished current todo".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("codex".into()),
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
        from: "coder".into(),
        display_source: Some("codex".into()),
        to: "lead".into(),
        content: "finished current todo".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("codex".into()),
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
        from: "user".into(),
        display_source: None,
        to: "coder".into(),
        content: "resume".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        sender_agent_id: None,
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
    });
    let session_b = s.task_graph.create_session(CreateSessionParams {
        task_id: &task_b.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws/b",
        title: "Coder B",
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
        from: "coder".into(),
        display_source: Some("codex".into()),
        to: "lead".into(),
        content: "done".into(),
        timestamp: 1,
        reply_to: None,
        priority: None,
        status: Some(crate::daemon::types::MessageStatus::Done),
        task_id: None,
        session_id: None,
        sender_agent_id: Some("codex".into()),
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
