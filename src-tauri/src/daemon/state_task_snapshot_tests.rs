use super::*;

// ── Task snapshot methods ─────────────────────────────────────

#[test]
fn task_snapshot_returns_none_without_active_task() {
    let s = DaemonState::new();
    assert!(s.task_snapshot().is_none());
}

#[test]
fn create_and_select_task_sets_active() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "T1");
    assert_eq!(s.active_task_id.as_deref(), Some(task.task_id.as_str()));
}

#[test]
fn task_snapshot_returns_active_task_data() {
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "Test Task");
    let snap = s.task_snapshot().expect("snapshot exists");
    assert_eq!(snap.task.task_id, task.task_id);
    assert_eq!(snap.task.title, "Test Task");
    assert!(snap.sessions.is_empty());
    assert!(snap.artifacts.is_empty());
}

#[test]
fn task_snapshot_includes_sessions() {
    use crate::daemon::task_graph::types::*;
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "With Sessions");
    s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "Lead",
    });
    let snap = s.task_snapshot().unwrap();
    assert_eq!(snap.sessions.len(), 1);
    assert_eq!(snap.sessions[0].provider, Provider::Claude);
}

#[test]
fn select_task_returns_error_for_missing() {
    let mut s = DaemonState::new();
    assert!(s.select_task("no-such").is_err());
}

#[test]
fn select_task_switches_active() {
    let mut s = DaemonState::new();
    let t1 = s.create_and_select_task("/ws", "T1");
    let t2 = s.task_graph.create_task("/ws", "T2");
    assert_eq!(s.active_task_id.as_deref(), Some(t1.task_id.as_str()));
    s.select_task(&t2.task_id).unwrap();
    assert_eq!(s.active_task_id.as_deref(), Some(t2.task_id.as_str()));
}

#[test]
fn task_list_returns_all_tasks() {
    let mut s = DaemonState::new();
    s.task_graph.create_task("/ws1", "A");
    s.task_graph.create_task("/ws2", "B");
    assert_eq!(s.task_list(None).len(), 2);
}

#[test]
fn task_list_filters_by_workspace() {
    let mut s = DaemonState::new();
    s.task_graph.create_task("/ws1", "A");
    s.task_graph.create_task("/ws2", "B");
    s.task_graph.create_task("/ws1", "C");
    assert_eq!(s.task_list(Some("/ws1")).len(), 2);
    assert_eq!(s.task_list(Some("/ws2")).len(), 1);
    assert_eq!(s.task_list(Some("/ws3")).len(), 0);
}

// ── Session tree ──────────────────────────────────────────────

#[test]
fn session_tree_returns_none_for_missing_task() {
    let s = DaemonState::new();
    assert!(s.session_tree("no-such").is_none());
}

#[test]
fn session_tree_returns_sessions_for_task() {
    use crate::daemon::task_graph::types::*;
    let mut s = DaemonState::new();
    let task = s.create_and_select_task("/ws", "T1");
    s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "Lead",
    });
    let tree = s.session_tree(&task.task_id).unwrap();
    assert_eq!(tree.task_id, task.task_id);
    assert_eq!(tree.sessions.len(), 1);
}

// ── History ───────────────────────────────────────────────────

#[test]
fn task_history_returns_entries_with_counts() {
    use crate::daemon::task_graph::types::*;
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "Lead",
    });
    let history = s.task_history(Some("/ws"));
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].session_count, 1);
    assert_eq!(history[0].artifact_count, 0);
}

#[test]
fn task_history_empty_workspace_returns_empty() {
    let s = DaemonState::new();
    assert!(s.task_history(Some("/ws")).is_empty());
}

// ── Resume session ────────────────────────────────────────────

#[test]
fn resume_session_sets_active_task_and_pointer() {
    use crate::daemon::task_graph::types::*;
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let sess = s.task_graph.create_session(CreateSessionParams {
        task_id: &task.task_id, parent_session_id: None,
        provider: Provider::Codex, role: SessionRole::Coder,
        cwd: "/ws", title: "Coder",
    });
    s.task_graph.update_session_status(&sess.session_id, SessionStatus::Paused);
    s.active_task_id = None;

    let returned_task_id = s.resume_session(&sess.session_id).unwrap();

    assert_eq!(returned_task_id, task.task_id, "resume_session must return task_id, not session_id");
    assert_eq!(s.active_task_id.as_deref(), Some(task.task_id.as_str()));
    let updated = s.task_graph.get_task(&task.task_id).unwrap();
    assert_eq!(updated.current_coder_session_id.as_deref(), Some(sess.session_id.as_str()));
    let updated_sess = s.task_graph.get_session(&sess.session_id).unwrap();
    assert_eq!(updated_sess.status, SessionStatus::Active);
}

#[test]
fn resume_session_returns_error_for_missing() {
    let mut s = DaemonState::new();
    assert!(s.resume_session("no-such").is_err());
}
