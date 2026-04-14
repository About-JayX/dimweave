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
    assert_eq!(task.lead_provider, Provider::Claude);
    assert_eq!(task.coder_provider, Provider::Codex);
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

#[test]
fn update_workspace_root_changes_path() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/repo", "T1");
    assert!(store.update_workspace_root(&task.task_id, "/repo/.worktrees/tasks/t1"));
    let t = store.get_task(&task.task_id).unwrap();
    assert_eq!(t.workspace_root, "/repo/.worktrees/tasks/t1");
}

#[test]
fn task_provider_fields_serialize_round_trip() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Providers");
    let json = serde_json::to_string(&task).unwrap();
    let de: Task = serde_json::from_str(&json).unwrap();
    assert_eq!(de.lead_provider, Provider::Claude);
    assert_eq!(de.coder_provider, Provider::Codex);
}

// ── Task config contract ───────────────────────────────────

#[test]
fn create_task_with_config_stores_custom_providers() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task_with_config("/ws", "Custom", Provider::Codex, Provider::Claude);
    assert_eq!(task.lead_provider, Provider::Codex);
    assert_eq!(task.coder_provider, Provider::Claude);
    assert_eq!(task.status, TaskStatus::Draft);
}

#[test]
fn update_task_providers_changes_bindings() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    assert!(store.update_task_providers(&task.task_id, Provider::Codex, Provider::Claude));
    let t = store.get_task(&task.task_id).unwrap();
    assert_eq!(t.lead_provider, Provider::Codex);
    assert_eq!(t.coder_provider, Provider::Claude);
}

#[test]
fn update_task_providers_returns_false_for_missing() {
    let mut store = TaskGraphStore::new();
    assert!(!store.update_task_providers("nonexistent", Provider::Claude, Provider::Codex));
}

#[test]
fn create_task_empty_title_defaults_to_task_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "");
    assert_eq!(task.title, task.task_id);
}

#[test]
fn create_task_with_config_empty_title_defaults_to_task_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task_with_config("/ws", "", Provider::Claude, Provider::Codex);
    assert_eq!(task.title, task.task_id);
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
    assert_eq!(
        coder.parent_session_id.as_deref(),
        Some(lead.session_id.as_str())
    );
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
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
    });
    assert_eq!(store.sessions_for_task(&tid).len(), 2);
}

#[test]
fn children_of_session_returns_only_children() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    let lead = store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    let lid = lead.session_id.clone();
    store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: Some(&lid),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "C1",
    });
    store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: Some(&lid),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "C2",
    });
    assert_eq!(store.children_of_session(&lid).len(), 2);
}

#[test]
fn lead_session_for_task_returns_correct() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let tid = task.task_id.clone();
    let lead = store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
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
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "S1",
    });
    let s1id = s1.session_id.clone();
    store.add_artifact(CreateArtifactParams {
        task_id: &tid,
        session_id: &s1id,
        kind: ArtifactKind::Research,
        title: "A1",
        content_ref: "a1",
    });
    store.add_artifact(CreateArtifactParams {
        task_id: &tid,
        session_id: &s1id,
        kind: ArtifactKind::Plan,
        title: "A2",
        content_ref: "a2",
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
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "S1",
    });
    let json = serde_json::to_string(&sess).unwrap();
    let de: SessionHandle = serde_json::from_str(&json).unwrap();
    assert_eq!(de.session_id, sess.session_id);
    assert_eq!(de.provider, Provider::Claude);
}

// ── Persistence round-trip ─────────────────────────────────

use std::path::PathBuf;

fn tmp_persist_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("dimweave_test_{name}_{}.json", std::process::id()))
}

struct CleanupFile(PathBuf);
impl Drop for CleanupFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[test]
fn persist_save_and_load_round_trip() {
    let path = tmp_persist_path("round_trip");
    let _cleanup = CleanupFile(path.clone());

    let mut store = TaskGraphStore::with_persist_path(path.clone());
    let task = store.create_task("/ws", "Persist Me");
    let tid = task.task_id.clone();
    let sess = store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder S",
    });
    let sid = sess.session_id.clone();
    let art = store.add_artifact(CreateArtifactParams {
        task_id: &tid,
        session_id: &sid,
        kind: ArtifactKind::Plan,
        title: "Plan v1",
        content_ref: "plan.md",
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
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    let lid = lead.session_id.clone();
    store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lid),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
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

#[test]
fn find_session_by_external_id_filters_by_provider() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Lookup");
    let codex = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Codex",
    });
    let claude = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Claude",
    });
    assert!(store.set_external_session_id(&codex.session_id, "shared_id"));
    assert!(store.set_external_session_id(&claude.session_id, "shared_id"));

    let found = store
        .find_session_by_external_id(Provider::Codex, "shared_id")
        .expect("codex session");
    assert_eq!(found.session_id, codex.session_id);
    assert_eq!(found.provider, Provider::Codex);
}

// ── TaskAgent Model ────────────────────────────────────────

#[test]
fn add_task_agent_returns_record_with_stable_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Agent Test");
    let agent = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    assert!(agent.agent_id.starts_with("agent_"));
    assert_eq!(agent.task_id, task.task_id);
    assert_eq!(agent.provider, Provider::Claude);
    assert_eq!(agent.role, "lead");
    assert_eq!(agent.order, 0);
}

#[test]
fn add_task_agent_increments_order() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Order Test");
    let a1 = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    let a2 = store.add_task_agent(&task.task_id, Provider::Codex, "coder");
    assert_eq!(a1.order, 0);
    assert_eq!(a2.order, 1);
}

#[test]
fn agents_for_task_returns_all_linked() {
    let mut store = TaskGraphStore::new();
    let t1 = store.create_task("/ws", "T1");
    let t2 = store.create_task("/ws", "T2");
    store.add_task_agent(&t1.task_id, Provider::Claude, "lead");
    store.add_task_agent(&t1.task_id, Provider::Codex, "coder");
    store.add_task_agent(&t2.task_id, Provider::Claude, "lead");
    let agents = store.agents_for_task(&t1.task_id);
    assert_eq!(agents.len(), 2);
    assert!(agents.iter().all(|a| a.task_id == t1.task_id));
}

#[test]
fn get_task_agent_returns_by_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Lookup");
    let agent = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    let fetched = store.get_task_agent(&agent.agent_id).expect("should exist");
    assert_eq!(fetched.role, "lead");
}

#[test]
fn remove_task_agent_works() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Remove Test");
    let agent = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    assert!(store.remove_task_agent(&agent.agent_id));
    assert!(store.get_task_agent(&agent.agent_id).is_none());
    assert!(!store.remove_task_agent(&agent.agent_id));
}

#[test]
fn task_agent_serialization_round_trip() {
    let agent = TaskAgent {
        agent_id: "agent_1".into(),
        task_id: "task_1".into(),
        provider: Provider::Claude,
        role: "lead".into(),
        display_name: None,
        order: 0,
        created_at: 100,
        updated_at: 200,
    };
    let json = serde_json::to_string(&agent).unwrap();
    let decoded: TaskAgent = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.agent_id, "agent_1");
    assert_eq!(decoded.role, "lead");
    assert_eq!(decoded.provider, Provider::Claude);
}

// ── Legacy Migration ───────────────────────────────────────

#[test]
fn migrate_legacy_agents_creates_two_agents_when_both_sessions_exist() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task_with_config("/ws", "Migrate", Provider::Claude, Provider::Codex);
    // Create sessions to provide occupancy evidence
    let lead_sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    store.set_lead_session(&task.task_id, &lead_sess.session_id);
    let coder_sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead_sess.session_id),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
    });
    store.set_coder_session(&task.task_id, &coder_sess.session_id);
    // No agents exist yet
    assert!(store.agents_for_task(&task.task_id).is_empty());
    store.migrate_legacy_agents();
    let agents = store.agents_for_task(&task.task_id);
    assert_eq!(agents.len(), 2);
    let roles: Vec<&str> = agents.iter().map(|a| a.role.as_str()).collect();
    assert!(roles.contains(&"lead"));
    assert!(roles.contains(&"coder"));
    let lead = agents.iter().find(|a| a.role == "lead").unwrap();
    let coder = agents.iter().find(|a| a.role == "coder").unwrap();
    assert_eq!(lead.provider, Provider::Claude);
    assert_eq!(coder.provider, Provider::Codex);
}

#[test]
fn migrate_legacy_agents_idempotent() {
    let mut store = TaskGraphStore::new();
    store.create_task("/ws", "Idempotent");
    store.migrate_legacy_agents();
    let count_after_first = store.agents_for_task(
        &store.list_tasks()[0].task_id,
    ).len();
    store.migrate_legacy_agents();
    let count_after_second = store.agents_for_task(
        &store.list_tasks()[0].task_id,
    ).len();
    assert_eq!(count_after_first, count_after_second);
}

#[test]
fn migrate_legacy_agents_same_provider_both_roles() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task_with_config("/ws", "Same", Provider::Claude, Provider::Claude);
    let lead_sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    store.set_lead_session(&task.task_id, &lead_sess.session_id);
    let coder_sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead_sess.session_id),
        provider: Provider::Claude,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
    });
    store.set_coder_session(&task.task_id, &coder_sess.session_id);
    store.migrate_legacy_agents();
    let agents = store.agents_for_task(&task.task_id);
    assert_eq!(agents.len(), 2);
    assert!(agents.iter().all(|a| a.provider == Provider::Claude));
    let roles: Vec<&str> = agents.iter().map(|a| a.role.as_str()).collect();
    assert!(roles.contains(&"lead"));
    assert!(roles.contains(&"coder"));
}

#[test]
fn migrate_legacy_agents_skips_tasks_that_already_have_agents() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "PreExisting");
    store.add_task_agent(&task.task_id, Provider::Claude, "architect");
    store.migrate_legacy_agents();
    let agents = store.agents_for_task(&task.task_id);
    // Should still have exactly 1 (the manually added one), not 3
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].role, "architect");
}

#[test]
fn migrate_legacy_agents_zero_agents_when_no_sessions() {
    let mut store = TaskGraphStore::new();
    // Task with default providers but no sessions — no occupancy evidence
    store.create_task("/ws", "NoSessions");
    store.migrate_legacy_agents();
    let agents = store.agents_for_task(&store.list_tasks()[0].task_id);
    assert!(agents.is_empty(), "task with no session evidence must produce zero agents");
}

#[test]
fn migrate_legacy_agents_one_agent_when_only_lead_session() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task_with_config("/ws", "LeadOnly", Provider::Claude, Provider::Codex);
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
    });
    store.set_lead_session(&task.task_id, &sess.session_id);
    store.migrate_legacy_agents();
    let agents = store.agents_for_task(&task.task_id);
    assert_eq!(agents.len(), 1, "only lead session evidence → one agent");
    assert_eq!(agents[0].role, "lead");
    assert_eq!(agents[0].provider, Provider::Claude);
}

// ── Persistence with TaskAgents ────────────────────────────

#[test]
fn persist_task_agents_round_trip() {
    let path = tmp_persist_path("agents_roundtrip");
    let _cleanup = CleanupFile(path.clone());
    {
        let mut store = TaskGraphStore::with_persist_path(path.clone());
        let task = store.create_task("/ws", "Persist Agents");
        store.add_task_agent(&task.task_id, Provider::Claude, "lead");
        store.add_task_agent(&task.task_id, Provider::Codex, "coder");
        store.save().unwrap();
    }
    {
        let store = TaskGraphStore::load(&path).unwrap();
        let tasks = store.list_tasks();
        assert_eq!(tasks.len(), 1);
        let agents = store.agents_for_task(&tasks[0].task_id);
        assert_eq!(agents.len(), 2);
    }
}

#[test]
fn persist_migration_runs_on_old_format_load() {
    let path = tmp_persist_path("migration_on_load");
    let _cleanup = CleanupFile(path.clone());
    // Write an old-format snapshot with sessions (occupancy evidence)
    {
        let mut store = TaskGraphStore::with_persist_path(path.clone());
        let task = store.create_task_with_config("/ws", "OldFormat", Provider::Codex, Provider::Claude);
        let lead_sess = store.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Codex,
            role: SessionRole::Lead,
            cwd: "/ws",
            title: "Lead",
        });
        store.set_lead_session(&task.task_id, &lead_sess.session_id);
        let coder_sess = store.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: Some(&lead_sess.session_id),
            provider: Provider::Claude,
            role: SessionRole::Coder,
            cwd: "/ws",
            title: "Coder",
        });
        store.set_coder_session(&task.task_id, &coder_sess.session_id);
        store.save().unwrap();
    }
    // Remove agents by saving raw JSON without the field
    {
        let data = std::fs::read_to_string(&path).unwrap();
        let mut val: serde_json::Value = serde_json::from_str(&data).unwrap();
        val.as_object_mut().unwrap().remove("task_agents");
        std::fs::write(&path, serde_json::to_string_pretty(&val).unwrap()).unwrap();
    }
    // Load should trigger migration using session evidence
    {
        let store = TaskGraphStore::load(&path).unwrap();
        let tasks = store.list_tasks();
        let agents = store.agents_for_task(&tasks[0].task_id);
        assert_eq!(agents.len(), 2);
        let lead = agents.iter().find(|a| a.role == "lead").unwrap();
        assert_eq!(lead.provider, Provider::Codex);
    }
}
