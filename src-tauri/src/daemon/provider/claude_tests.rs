use crate::daemon::provider::claude;
use crate::daemon::provider::shared::SessionRegistration;
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;
use crate::daemon::DaemonState;

// ── register_session ───────────────────────────────────────

#[test]
fn register_claude_session_binds_session_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: None,
        role: SessionRole::Lead,
        cwd: "/ws".into(),
        title: "Claude lead".into(),
        external_id: Some("claude_sess_abc".into()),
    };
    let sess = claude::register_session(&mut store, reg);

    assert_eq!(sess.provider, Provider::Claude);
    assert_eq!(sess.role, SessionRole::Lead);
    assert_eq!(sess.external_session_id.as_deref(), Some("claude_sess_abc"));
    assert_eq!(sess.task_id, task.task_id);
}

#[test]
fn register_claude_session_without_external_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: None,
        role: SessionRole::Lead,
        cwd: "/ws".into(),
        title: "Claude lead".into(),
        external_id: None,
    };
    let sess = claude::register_session(&mut store, reg);
    assert!(sess.external_session_id.is_none());
    assert_eq!(sess.provider, Provider::Claude);
}

// ── bind_session_id ────────────────────────────────────────

#[test]
fn bind_session_id_updates_existing_session() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Claude",
    });

    let ok = claude::bind_session_id(&mut store, &sess.session_id, "claude_sess_late");
    assert!(ok);
    let updated = store.get_session(&sess.session_id).unwrap();
    assert_eq!(
        updated.external_session_id.as_deref(),
        Some("claude_sess_late")
    );
}

#[test]
fn bind_session_id_returns_false_for_missing() {
    let mut store = TaskGraphStore::new();
    assert!(!claude::bind_session_id(&mut store, "no-such", "x"));
}

// ── register_on_connect (integration with DaemonState) ─────

#[test]
fn register_on_connect_creates_lead_session() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.task_graph.update_task_status(&tid, TaskStatus::Planning);
    s.set_active_task(Some(tid.clone()));

    claude::register_on_connect(&mut s, "lead", "/ws", Some("claude_sess_123"));

    let task = s.task_graph.get_task(&tid).unwrap();
    let lead_sid = task.lead_session_id.as_ref().expect("lead session set");
    let sess = s.task_graph.get_session(lead_sid).unwrap();
    assert_eq!(sess.provider, Provider::Claude);
    assert_eq!(sess.role, SessionRole::Lead);
    assert_eq!(sess.external_session_id.as_deref(), Some("claude_sess_123"));
}

#[test]
fn register_on_connect_as_coder_sets_coder_session() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.task_graph
        .update_task_status(&tid, TaskStatus::Implementing);
    s.set_active_task(Some(tid.clone()));

    claude::register_on_connect(&mut s, "coder", "/ws", Some("claude_coder_1"));

    let task = s.task_graph.get_task(&tid).unwrap();
    let coder_sid = task
        .current_coder_session_id
        .as_ref()
        .expect("coder session set");
    let sess = s.task_graph.get_session(coder_sid).unwrap();
    assert_eq!(sess.provider, Provider::Claude);
    assert_eq!(sess.role, SessionRole::Coder);
    assert_eq!(sess.external_session_id.as_deref(), Some("claude_coder_1"));
}

#[test]
fn register_on_connect_noop_without_active_task() {
    let mut s = DaemonState::new();
    claude::register_on_connect(&mut s, "lead", "/ws", Some("sess_x"));
    assert!(s.task_graph.list_tasks().is_empty());
}

#[test]
fn register_on_connect_without_session_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.set_active_task(Some(tid.clone()));

    claude::register_on_connect(&mut s, "lead", "/ws", None);

    let task = s.task_graph.get_task(&tid).unwrap();
    let lead_sid = task.lead_session_id.as_ref().expect("lead session set");
    let sess = s.task_graph.get_session(lead_sid).unwrap();
    assert!(sess.external_session_id.is_none());
}
