use super::*;
use crate::daemon::types::{OnlineAgentInfo, RuntimeHealthLevel, RuntimeHealthStatus};

#[test]
fn init_and_get_task_runtime() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "RT Test");
    assert!(s.get_task_runtime(&task.task_id).is_none());

    s.init_task_runtime(&task.task_id, std::path::PathBuf::from("/ws/tasks/t1"));
    let rt = s.get_task_runtime(&task.task_id).expect("runtime exists");
    assert_eq!(rt.task_id, task.task_id);
    assert_eq!(rt.workspace_root, std::path::PathBuf::from("/ws/tasks/t1"));
}

#[test]
fn task_runtimes_empty_by_default() {
    let s = DaemonState::new();
    assert!(s.task_runtimes.is_empty());
}

#[test]
fn online_agents_snapshot_empty_when_no_agents() {
    let s = DaemonState::new();
    let snapshot = s.online_agents_snapshot();
    assert!(snapshot.is_empty());
}

#[test]
fn online_agents_snapshot_only_claude_sdk() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel::<String>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "lead".into();
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", tx).is_some());

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(
        snapshot[0],
        OnlineAgentInfo {
            agent_id: "claude".into(),
            role: "lead".into(),
            model_source: "claude".into(),
        }
    );
}

#[test]
fn online_agents_snapshot_excludes_bridge_only_claude() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel::<ToAgent>(1);
    s.attached_agents.insert(
        "claude".into(),
        crate::daemon::state::AgentSender::new(tx, 0),
    );

    let snapshot = s.online_agents_snapshot();
    assert!(snapshot.is_empty());
}

#[test]
fn online_agents_snapshot_only_codex() {
    let mut s = DaemonState::new();
    let (tx, _rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    s.codex_role = "coder".into();
    s.codex_inject_tx = Some(tx);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 1);
    assert_eq!(
        snapshot[0],
        OnlineAgentInfo {
            agent_id: "codex".into(),
            role: "coder".into(),
            model_source: "codex".into(),
        }
    );
}

#[test]
fn online_agents_snapshot_both_agents_in_fixed_order() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<String>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "lead".into();
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", claude_tx).is_some());
    s.codex_inject_tx = Some(codex_tx);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot.len(), 2);
    assert_eq!(snapshot[0].agent_id, "claude");
    assert_eq!(snapshot[1].agent_id, "codex");
}

#[test]
fn online_agents_snapshot_role_reflects_current_state() {
    let mut s = DaemonState::new();
    let (claude_tx, _claude_rx) = tokio::sync::mpsc::channel::<String>(1);
    let (codex_tx, _codex_rx) = tokio::sync::mpsc::channel::<(Vec<serde_json::Value>, bool)>(1);
    let epoch = s.begin_claude_sdk_launch("nonce-a".into());
    s.claude_role = "coder".into();
    s.codex_role = "lead".into();
    assert!(s.attach_claude_sdk_ws(epoch, "nonce-a", claude_tx).is_some());
    s.codex_inject_tx = Some(codex_tx);

    let snapshot = s.online_agents_snapshot();
    assert_eq!(snapshot[0].role, "coder");
    assert_eq!(snapshot[1].role, "lead");
}

#[test]
fn status_snapshot_includes_runtime_health() {
    let mut s = DaemonState::new();
    s.set_runtime_health(RuntimeHealthStatus {
        level: RuntimeHealthLevel::Error,
        source: "claude_sdk".into(),
        message: "Claude reconnect failed after 5 attempts".into(),
    });

    let snapshot = s.status_snapshot();

    assert_eq!(
        snapshot.runtime_health,
        Some(RuntimeHealthStatus {
            level: RuntimeHealthLevel::Error,
            source: "claude_sdk".into(),
            message: "Claude reconnect failed after 5 attempts".into(),
        })
    );
}

#[test]
fn rollback_task_creation_cleans_up_all_state() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "Rollback Test");
    let task_id = task.task_id.clone();
    s.init_task_runtime(&task_id, std::path::PathBuf::from("/ws/tasks/t1"));

    // Verify state exists before rollback
    assert!(s.task_graph.get_task(&task_id).is_some());
    assert_eq!(s.active_task_id.as_deref(), Some(task_id.as_str()));
    assert!(s.get_task_runtime(&task_id).is_some());

    s.rollback_task_creation(&task_id);

    assert!(s.task_graph.get_task(&task_id).is_none());
    assert!(s.active_task_id.is_none());
    assert!(s.get_task_runtime(&task_id).is_none());
}
