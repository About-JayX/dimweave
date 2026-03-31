use super::store::TaskGraphStore;
use super::types::*;

// ── Task CRUD ───────────────────────────────────────────────

#[test]
fn create_task_returns_draft_status() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/workspace", "My Task");
    assert_eq!(task.status, TaskStatus::Draft);
    assert_eq!(task.workspace_root, "/workspace");
    assert_eq!(task.title, "My Task");
    assert!(!task.task_id.is_empty());
    assert!(task.lead_session_id.is_none());
    assert!(task.current_coder_session_id.is_none());
}

#[test]
fn get_task_returns_created_task() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let id = task.task_id.clone();
    let fetched = store.get_task(&id).expect("task should exist");
    assert_eq!(fetched.title, "T1");
}

#[test]
fn update_task_status_changes_status() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let id = task.task_id.clone();
    assert!(store.update_task_status(&id, TaskStatus::Planning));
    let t = store.get_task(&id).unwrap();
    assert_eq!(t.status, TaskStatus::Planning);
}

#[test]
fn update_task_status_returns_false_for_missing() {
    let mut store = TaskGraphStore::new();
    assert!(!store.update_task_status("nonexistent", TaskStatus::Done));
}

#[test]
fn list_tasks_returns_all() {
    let mut store = TaskGraphStore::new();
    store.create_task("/ws", "A");
    store.create_task("/ws", "B");
    assert_eq!(store.list_tasks().len(), 2);
}

// ── Session CRUD ────────────────────────────────────────────

#[test]
fn create_lead_session_linked_to_task() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead Session",
    });
    assert_eq!(sess.task_id, task.task_id);
    assert_eq!(sess.role, SessionRole::Lead);
    assert!(sess.parent_session_id.is_none());
    assert_eq!(sess.status, SessionStatus::Active);
}

#[test]
fn create_coder_child_session_with_parent() {
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
    let coder = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead.session_id),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
    });
    assert_eq!(coder.parent_session_id.as_deref(), Some(lead.session_id.as_str()));
    assert_eq!(coder.role, SessionRole::Coder);
}

#[test]
fn get_session_returns_created() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "S1",
    });
    let id = sess.session_id.clone();
    assert!(store.get_session(&id).is_some());
}

#[test]
fn update_session_status_works() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "S1",
    });
    assert!(store.update_session_status(&sess.session_id, SessionStatus::Completed));
    let s = store.get_session(&sess.session_id).unwrap();
    assert_eq!(s.status, SessionStatus::Completed);
}

// ── Artifact CRUD ───────────────────────────────────────────

#[test]
fn add_and_retrieve_artifact() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "S1",
    });
    let art = store.add_artifact(CreateArtifactParams {
        task_id: &task.task_id,
        session_id: &sess.session_id,
        kind: ArtifactKind::Plan,
        title: "Plan v1",
        content_ref: "plan.md",
    });
    assert_eq!(art.kind, ArtifactKind::Plan);
    let fetched = store.get_artifact(&art.artifact_id).unwrap();
    assert_eq!(fetched.title, "Plan v1");
}

// ── Parent-child session index ──────────────────────────────

#[test]
fn sessions_for_task_returns_all_linked() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    store.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "Lead",
    });
    store.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: None,
        provider: Provider::Codex, role: SessionRole::Coder,
        cwd: "/ws", title: "Coder",
    });
    assert_eq!(store.sessions_for_task(&tid).len(), 2);
}

#[test]
fn children_of_session_returns_only_children() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    let lead = store.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "Lead",
    });
    let lid = lead.session_id.clone();
    store.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: Some(&lid),
        provider: Provider::Codex, role: SessionRole::Coder,
        cwd: "/ws", title: "C1",
    });
    store.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: Some(&lid),
        provider: Provider::Codex, role: SessionRole::Coder,
        cwd: "/ws", title: "C2",
    });
    assert_eq!(store.children_of_session(&lid).len(), 2);
}

#[test]
fn lead_session_for_task_returns_correct() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    let lead = store.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "Lead",
    });
    store.set_lead_session(&tid, &lead.session_id);
    let found = store.lead_session_for_task(&tid).unwrap();
    assert_eq!(found.role, SessionRole::Lead);
}

// ── Task index ──────────────────────────────────────────────

#[test]
fn tasks_for_workspace_filters_correctly() {
    let mut store = TaskGraphStore::new();
    store.create_task("/ws1", "A");
    store.create_task("/ws2", "B");
    store.create_task("/ws1", "C");
    assert_eq!(store.tasks_for_workspace("/ws1").len(), 2);
}

#[test]
fn active_task_returns_non_terminal() {
    let mut store = TaskGraphStore::new();
    let t1 = store.create_task("/ws", "Done");
    store.update_task_status(&t1.task_id, TaskStatus::Done);
    let t2 = store.create_task("/ws", "Active");
    store.update_task_status(&t2.task_id, TaskStatus::Implementing);
    let active = store.active_task("/ws").unwrap();
    assert_eq!(active.title, "Active");
}

// ── Artifact index ──────────────────────────────────────────

#[test]
fn artifacts_for_task_and_session() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    let s1 = store.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "S1",
    });
    let s1id = s1.session_id.clone();
    store.add_artifact(CreateArtifactParams {
        task_id: &tid, session_id: &s1id,
        kind: ArtifactKind::Research, title: "A1", content_ref: "a1",
    });
    store.add_artifact(CreateArtifactParams {
        task_id: &tid, session_id: &s1id,
        kind: ArtifactKind::Plan, title: "A2", content_ref: "a2",
    });
    assert_eq!(store.artifacts_for_task(&tid).len(), 2);
    assert_eq!(store.artifacts_for_session(&s1id).len(), 2);
}

// ── Serialization round-trip ────────────────────────────────

#[test]
fn task_serialization_round_trip() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Round Trip");
    let json = serde_json::to_string(&task).unwrap();
    let deserialized: Task = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.task_id, task.task_id);
    assert_eq!(deserialized.status, TaskStatus::Draft);
}

#[test]
fn session_serialization_round_trip() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "S1",
    });
    let json = serde_json::to_string(&sess).unwrap();
    let de: SessionHandle = serde_json::from_str(&json).unwrap();
    assert_eq!(de.session_id, sess.session_id);
    assert_eq!(de.provider, Provider::Claude);
}

// ── Persistence round-trip ─────────────────────────────────

use std::path::PathBuf;

fn tmp_persist_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("agentnexus_test_{name}_{}.json", std::process::id()))
}

struct CleanupFile(PathBuf);
impl Drop for CleanupFile {
    fn drop(&mut self) { let _ = std::fs::remove_file(&self.0); }
}

#[test]
fn persist_save_and_load_round_trip() {
    let path = tmp_persist_path("round_trip");
    let _cleanup = CleanupFile(path.clone());

    let mut store = TaskGraphStore::with_persist_path(path.clone());
    let task = store.create_task("/ws", "Persist Me");
    let tid = task.task_id.clone();
    let sess = store.create_session(CreateSessionParams {
        task_id: &tid, parent_session_id: None,
        provider: Provider::Codex, role: SessionRole::Coder,
        cwd: "/ws", title: "Coder S",
    });
    let sid = sess.session_id.clone();
    let art = store.add_artifact(CreateArtifactParams {
        task_id: &tid, session_id: &sid,
        kind: ArtifactKind::Plan, title: "Plan v1", content_ref: "plan.md",
    });
    let aid = art.artifact_id.clone();
    store.save().expect("save should succeed");

    // Load into a fresh store from the same path
    let loaded = TaskGraphStore::load(&path).expect("load should succeed");
    let t = loaded.get_task(&tid).expect("task should exist");
    assert_eq!(t.title, "Persist Me");
    let s = loaded.get_session(&sid).expect("session should exist");
    assert_eq!(s.provider, Provider::Codex);
    let a = loaded.get_artifact(&aid).expect("artifact should exist");
    assert_eq!(a.title, "Plan v1");
}

#[test]
fn persist_load_missing_file_returns_empty_store() {
    let path = tmp_persist_path("missing");
    let _ = std::fs::remove_file(&path); // ensure absent
    let store = TaskGraphStore::load(&path).expect("load missing should succeed");
    assert_eq!(store.list_tasks().len(), 0);
}

#[test]
fn persist_preserves_next_id_counter() {
    let path = tmp_persist_path("next_id");
    let _cleanup = CleanupFile(path.clone());

    let mut store = TaskGraphStore::with_persist_path(path.clone());
    store.create_task("/ws", "T1");
    store.create_task("/ws", "T2");
    store.save().unwrap();

    let mut loaded = TaskGraphStore::load(&path).unwrap();
    let t3 = loaded.create_task("/ws", "T3");
    // next_id must not collide with existing IDs
    assert_ne!(t3.task_id, store.list_tasks()[0].task_id);
    assert_ne!(t3.task_id, store.list_tasks()[1].task_id);
}

#[test]
fn persist_parent_child_relationship_survives() {
    let path = tmp_persist_path("parent_child");
    let _cleanup = CleanupFile(path.clone());

    let mut store = TaskGraphStore::with_persist_path(path.clone());
    let task = store.create_task("/ws", "T1");
    let lead = store.create_session(CreateSessionParams {
        task_id: &task.task_id, parent_session_id: None,
        provider: Provider::Claude, role: SessionRole::Lead,
        cwd: "/ws", title: "Lead",
    });
    let lid = lead.session_id.clone();
    store.create_session(CreateSessionParams {
        task_id: &task.task_id, parent_session_id: Some(&lid),
        provider: Provider::Codex, role: SessionRole::Coder,
        cwd: "/ws", title: "Coder",
    });
    store.save().unwrap();

    let loaded = TaskGraphStore::load(&path).unwrap();
    assert_eq!(loaded.children_of_session(&lid).len(), 1);
    assert_eq!(loaded.sessions_for_task(&task.task_id).len(), 2);
}

#[test]
fn persist_no_path_save_is_noop() {
    let store = TaskGraphStore::new(); // no persist_path
    // save on in-memory-only store should succeed (no-op)
    store.save().expect("noop save should succeed");
}
