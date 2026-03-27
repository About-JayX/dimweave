use super::*;

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
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(String, bool)>(1);
    s.attached_agents.insert("claude".into(), crate::daemon::state::AgentSender::new(claude_tx, 0));
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
fn migrate_buffered_role_retargets_messages() {
    let mut s = DaemonState::new();
    s.buffer_message(BridgeMessage::system("hello", "lead"));
    s.buffer_message(BridgeMessage::system("world", "coder"));
    s.migrate_buffered_role("lead", "reviewer");
    assert!(s.buffered_messages.iter().all(|m| m.to != "lead"));
    assert!(s.buffered_messages.iter().any(|m| m.to == "reviewer"));
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
    s.buffer_permission_verdict("claude", PermissionVerdict {
        request_id: "req-1".into(),
        behavior: PermissionBehavior::Allow,
    });
    s.buffer_permission_verdict("claude", PermissionVerdict {
        request_id: "req-2".into(),
        behavior: PermissionBehavior::Deny,
    });
    let verdicts = s.take_buffered_verdicts_for("claude");
    assert_eq!(verdicts.len(), 2);
    assert!(s.take_buffered_verdicts_for("claude").is_empty());
}

#[test]
fn buffered_verdicts_cap_at_50() {
    let mut s = DaemonState::new();
    for i in 0..60 {
        s.buffer_permission_verdict("claude", PermissionVerdict {
            request_id: format!("req-{i}"),
            behavior: PermissionBehavior::Allow,
        });
    }
    let verdicts = s.take_buffered_verdicts_for("claude");
    assert!(verdicts.len() <= 50);
}
