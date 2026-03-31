use crate::daemon::provider::codex;
use crate::daemon::provider::shared::SessionRegistration;
use crate::daemon::task_graph::store::TaskGraphStore;
use crate::daemon::task_graph::types::*;

// ── register_session ───────────────────────────────────────

#[test]
fn register_codex_session_binds_thread_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: None,
        role: SessionRole::Coder,
        cwd: "/ws".into(),
        title: "Codex coder".into(),
        external_id: Some("thread_abc123".into()),
    };
    let sess = codex::register_session(&mut store, reg);

    assert_eq!(sess.provider, Provider::Codex);
    assert_eq!(sess.role, SessionRole::Coder);
    assert_eq!(sess.external_session_id.as_deref(), Some("thread_abc123"));
    assert_eq!(sess.task_id, task.task_id);
    assert_eq!(sess.status, SessionStatus::Active);
}

#[test]
fn register_codex_session_without_thread_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: None,
        role: SessionRole::Coder,
        cwd: "/ws".into(),
        title: "Codex coder".into(),
        external_id: None,
    };
    let sess = codex::register_session(&mut store, reg);
    assert!(sess.external_session_id.is_none());
}

#[test]
fn register_codex_session_as_child_of_lead() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let lead = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });

    let reg = SessionRegistration {
        task_id: task.task_id.clone(),
        parent_session_id: Some(lead.session_id.clone()),
        role: SessionRole::Coder,
        cwd: "/ws".into(),
        title: "Codex coder".into(),
        external_id: Some("thread_xyz".into()),
    };
    let sess = codex::register_session(&mut store, reg);

    assert_eq!(sess.parent_session_id.as_deref(), Some(lead.session_id.as_str()));
    let children = store.children_of_session(&lead.session_id);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].external_session_id.as_deref(), Some("thread_xyz"));
}

#[test]
fn bind_thread_id_updates_existing_session() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Codex",
    });
    assert!(sess.external_session_id.is_none());

    let ok = codex::bind_thread_id(&mut store, &sess.session_id, "thread_late");
    assert!(ok);
    let updated = store.get_session(&sess.session_id).unwrap();
    assert_eq!(updated.external_session_id.as_deref(), Some("thread_late"));
}

#[test]
fn bind_thread_id_returns_false_for_missing_session() {
    let mut store = TaskGraphStore::new();
    assert!(!codex::bind_thread_id(&mut store, "no-such", "thread_x"));
}

// ── register_on_launch (integration with DaemonState) ──────

use crate::daemon::DaemonState;

#[test]
fn register_on_launch_creates_session_with_thread_id() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    s.task_graph.update_task_status(&tid, TaskStatus::Implementing);
    s.set_active_task(Some(tid.clone()));

    codex::register_on_launch(&mut s, "coder", "/ws", "thread_abc");

    // Session registered with correct external_session_id
    let task = s.task_graph.get_task(&tid).unwrap();
    let coder_sid = task.current_coder_session_id.as_ref().expect("coder session set");
    let sess = s.task_graph.get_session(coder_sid).unwrap();
    assert_eq!(sess.provider, Provider::Codex);
    assert_eq!(sess.role, SessionRole::Coder);
    assert_eq!(sess.external_session_id.as_deref(), Some("thread_abc"));
    assert_eq!(sess.task_id, tid);
}

#[test]
fn register_on_launch_noop_without_active_task() {
    let mut s = DaemonState::new();
    // No active task — should not panic or create anything
    codex::register_on_launch(&mut s, "coder", "/ws", "thread_xyz");
    assert!(s.task_graph.list_tasks().is_empty());
}

#[test]
fn register_on_launch_links_to_lead_session() {
    let mut s = DaemonState::new();
    let task = s.task_graph.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    let lead = s.task_graph.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "Lead",
    });
    s.task_graph.set_lead_session(&tid, &lead.session_id);
    s.task_graph.update_task_status(&tid, TaskStatus::Implementing);
    s.set_active_task(Some(tid.clone()));

    codex::register_on_launch(&mut s, "coder", "/ws", "thread_child");

    let task = s.task_graph.get_task(&tid).unwrap();
    let coder_sid = task.current_coder_session_id.as_ref().unwrap();
    let sess = s.task_graph.get_session(coder_sid).unwrap();
    assert_eq!(sess.parent_session_id.as_deref(), Some(lead.session_id.as_str()));
    assert_eq!(s.task_graph.children_of_session(&lead.session_id).len(), 1);
}

// ── adapter trait shape (future-proofing) ──────────────────

#[test]
fn codex_adapter_has_expected_interface() {
    let _ = codex::register_session as fn(&mut TaskGraphStore, SessionRegistration) -> SessionHandle;
    let _ = codex::bind_thread_id as fn(&mut TaskGraphStore, &str, &str) -> bool;
}
