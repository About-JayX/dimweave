use super::store::TaskGraphStore;
use super::types::*;

// ── Task CRUD ───────────────────────────────────────────────

#[test]
fn create_task_returns_draft_status() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/workspace", "My Task");
    assert_eq!(task.status, TaskStatus::Draft);
    assert_eq!(task.task_worktree_root, "/workspace");
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
fn update_task_worktree_root_changes_path() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/repo", "T1");
    assert!(store.update_task_worktree_root(&task.task_id, "/repo/.worktrees/tasks/t1"));
    let t = store.get_task(&task.task_id).unwrap();
    assert_eq!(t.task_worktree_root, "/repo/.worktrees/tasks/t1");
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
        agent_id: None,
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
        agent_id: None,
    });
    let coder = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead.session_id),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
        agent_id: None,
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
        agent_id: None,
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
        agent_id: None,
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
        agent_id: None,
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
        agent_id: None,
    });
    store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
        agent_id: None,
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
        agent_id: None,
    });
    let lid = lead.session_id.clone();
    store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: Some(&lid),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "C1",
        agent_id: None,
    });
    store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: Some(&lid),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "C2",
        agent_id: None,
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
        agent_id: None,
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
        agent_id: None,
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
        agent_id: None,
    });
    let json = serde_json::to_string(&sess).unwrap();
    let de: SessionHandle = serde_json::from_str(&json).unwrap();
    assert_eq!(de.session_id, sess.session_id);
    assert_eq!(de.provider, Provider::Claude);
}

// ── SQLite persistence round-trip ─────────────────────────

use std::path::PathBuf;

fn tmp_db_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("dimweave_test_{name}_{}.db", std::process::id()))
}

struct CleanupFile(PathBuf);
impl Drop for CleanupFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        // WAL/SHM files
        let _ = std::fs::remove_file(self.0.with_extension("db-wal"));
        let _ = std::fs::remove_file(self.0.with_extension("db-shm"));
    }
}

#[test]
fn persist_save_and_load_round_trip() {
    let path = tmp_db_path("round_trip");
    let _cleanup = CleanupFile(path.clone());

    let tid;
    let sid;
    let aid;
    {
        let mut store = TaskGraphStore::open(&path).unwrap();
        let task = store.create_task("/ws", "Persist Me");
        tid = task.task_id.clone();
        let sess = store.create_session(CreateSessionParams {
            task_id: &tid,
            parent_session_id: None,
            provider: Provider::Codex,
            role: SessionRole::Coder,
            cwd: "/ws",
            title: "Coder S",
            agent_id: None,
        });
        sid = sess.session_id.clone();
        let art = store.add_artifact(CreateArtifactParams {
            task_id: &tid,
            session_id: &sid,
            kind: ArtifactKind::Plan,
            title: "Plan v1",
            content_ref: "plan.md",
        });
        aid = art.artifact_id.clone();
        store.save().expect("save should succeed");
    }

    // Re-open the database into a fresh store
    let loaded = TaskGraphStore::open(&path).expect("reopen should succeed");
    let t = loaded.get_task(&tid).expect("task should exist");
    assert_eq!(t.title, "Persist Me");
    assert_eq!(t.project_root, "/ws");
    assert_eq!(t.task_worktree_root, "/ws");
    let s = loaded.get_session(&sid).expect("session should exist");
    assert_eq!(s.provider, Provider::Codex);
    let a = loaded.get_artifact(&aid).expect("artifact should exist");
    assert_eq!(a.title, "Plan v1");
}

#[test]
fn persist_open_missing_file_creates_empty_store() {
    let path = tmp_db_path("missing");
    let _ = std::fs::remove_file(&path); // ensure absent
    let _cleanup = CleanupFile(path.clone());
    let store = TaskGraphStore::open(&path).expect("open missing should succeed");
    assert_eq!(store.list_tasks().len(), 0);
}

#[test]
fn persist_preserves_next_id_counter() {
    let path = tmp_db_path("next_id");
    let _cleanup = CleanupFile(path.clone());

    let existing_ids: Vec<String>;
    {
        let mut store = TaskGraphStore::open(&path).unwrap();
        store.create_task("/ws", "T1");
        store.create_task("/ws", "T2");
        existing_ids = store.list_tasks().iter().map(|t| t.task_id.clone()).collect();
        store.save().unwrap();
    }

    let mut loaded = TaskGraphStore::open(&path).unwrap();
    let t3 = loaded.create_task("/ws", "T3");
    assert!(!existing_ids.contains(&t3.task_id));
}

#[test]
fn persist_parent_child_relationship_survives() {
    let path = tmp_db_path("parent_child");
    let _cleanup = CleanupFile(path.clone());

    let tid;
    let lid;
    {
        let mut store = TaskGraphStore::open(&path).unwrap();
        let task = store.create_task("/ws", "T1");
        tid = task.task_id.clone();
        let lead = store.create_session(CreateSessionParams {
            task_id: &tid,
            parent_session_id: None,
            provider: Provider::Claude,
            role: SessionRole::Lead,
            cwd: "/ws",
            title: "Lead",
            agent_id: None,
        });
        lid = lead.session_id.clone();
        store.create_session(CreateSessionParams {
            task_id: &tid,
            parent_session_id: Some(&lid),
            provider: Provider::Codex,
            role: SessionRole::Coder,
            cwd: "/ws",
            title: "Coder",
            agent_id: None,
        });
        store.save().unwrap();
    }

    let loaded = TaskGraphStore::open(&path).unwrap();
    assert_eq!(loaded.children_of_session(&lid).len(), 1);
    assert_eq!(loaded.sessions_for_task(&tid).len(), 2);
}

#[test]
fn persist_no_db_save_is_noop() {
    let store = TaskGraphStore::new(); // no db
    store.save().expect("noop save should succeed");
}

#[test]
fn persist_project_root_and_task_worktree_root_are_independent() {
    let path = tmp_db_path("root_split");
    let _cleanup = CleanupFile(path.clone());

    let tid;
    {
        let mut store = TaskGraphStore::open(&path).unwrap();
        let task = store.create_task("/project", "Split");
        tid = task.task_id.clone();
        assert_eq!(task.project_root, "/project");
        assert_eq!(task.task_worktree_root, "/project");
        store.update_task_worktree_root(&tid, "/project/.worktrees/feat-x");
        store.save().unwrap();
    }

    let loaded = TaskGraphStore::open(&path).unwrap();
    let t = loaded.get_task(&tid).unwrap();
    assert_eq!(t.project_root, "/project");
    assert_eq!(t.task_worktree_root, "/project/.worktrees/feat-x");
}

#[test]
fn persist_tasks_for_workspace_uses_project_root() {
    let mut store = TaskGraphStore::new();
    let t1 = store.create_task("/project", "T1");
    store.update_task_worktree_root(&t1.task_id, "/project/.worktrees/a");
    let _t2 = store.create_task("/project", "T2");
    let _t3 = store.create_task("/other", "T3");
    // Both t1 and t2 have project_root="/project" so they should appear
    let ws_tasks = store.tasks_for_workspace("/project");
    assert_eq!(ws_tasks.len(), 2);
    // t3 belongs to a different project
    let other_tasks = store.tasks_for_workspace("/other");
    assert_eq!(other_tasks.len(), 1);
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
        agent_id: None,
    });
    let claude = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Claude",
        agent_id: None,
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
        model: None,
        effort: None,
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

#[test]
fn set_session_agent_id_binds_agent() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "AgentBind");
    let sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Test",
        agent_id: None,
    });
    assert!(sess.agent_id.is_none());
    assert!(store.set_session_agent_id(&sess.session_id, "agent_42"));
    let updated = store.get_session(&sess.session_id).unwrap();
    assert_eq!(updated.agent_id.as_deref(), Some("agent_42"));
}

#[test]
fn set_session_agent_id_returns_false_for_missing() {
    let mut store = TaskGraphStore::new();
    assert!(!store.set_session_agent_id("nonexistent", "agent_1"));
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
        agent_id: None,
    });
    store.set_lead_session(&task.task_id, &lead_sess.session_id);
    let coder_sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead_sess.session_id),
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
        agent_id: None,
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
        agent_id: None,
    });
    store.set_lead_session(&task.task_id, &lead_sess.session_id);
    let coder_sess = store.create_session(CreateSessionParams {
        task_id: &task.task_id,
        parent_session_id: Some(&lead_sess.session_id),
        provider: Provider::Claude,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "Coder",
        agent_id: None,
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
        agent_id: None,
    });
    store.set_lead_session(&task.task_id, &sess.session_id);
    store.migrate_legacy_agents();
    let agents = store.agents_for_task(&task.task_id);
    assert_eq!(agents.len(), 1, "only lead session evidence → one agent");
    assert_eq!(agents[0].role, "lead");
    assert_eq!(agents[0].provider, Provider::Claude);
}

// ── Cascade Delete ─────────────────────────────────────────

#[test]
fn remove_task_cascade_removes_task_and_children() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "CascadeMe");
    let tid = task.task_id.clone();
    let sess = store.create_session(CreateSessionParams {
        task_id: &tid,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "Lead",
        agent_id: None,
    });
    store.add_artifact(CreateArtifactParams {
        task_id: &tid,
        session_id: &sess.session_id,
        kind: ArtifactKind::Plan,
        title: "Plan",
        content_ref: "plan.md",
    });
    store.add_task_agent(&tid, Provider::Claude, "lead");
    assert!(store.remove_task_cascade(&tid));
    assert!(store.get_task(&tid).is_none());
    assert!(store.sessions_for_task(&tid).is_empty());
    assert!(store.artifacts_for_task(&tid).is_empty());
    assert!(store.agents_for_task(&tid).is_empty());
}

#[test]
fn remove_task_cascade_returns_false_for_missing() {
    let mut store = TaskGraphStore::new();
    assert!(!store.remove_task_cascade("nonexistent"));
}

#[test]
fn remove_task_cascade_preserves_other_tasks() {
    let mut store = TaskGraphStore::new();
    let t1 = store.create_task("/ws", "Keep");
    let t2 = store.create_task("/ws", "Delete");
    store.create_session(CreateSessionParams {
        task_id: &t1.task_id,
        parent_session_id: None,
        provider: Provider::Claude,
        role: SessionRole::Lead,
        cwd: "/ws",
        title: "T1 Lead",
        agent_id: None,
    });
    store.create_session(CreateSessionParams {
        task_id: &t2.task_id,
        parent_session_id: None,
        provider: Provider::Codex,
        role: SessionRole::Coder,
        cwd: "/ws",
        title: "T2 Coder",
        agent_id: None,
    });
    store.add_task_agent(&t1.task_id, Provider::Claude, "lead");
    store.add_task_agent(&t2.task_id, Provider::Codex, "coder");
    assert!(store.remove_task_cascade(&t2.task_id));
    assert!(store.get_task(&t1.task_id).is_some());
    assert_eq!(store.sessions_for_task(&t1.task_id).len(), 1);
    assert_eq!(store.agents_for_task(&t1.task_id).len(), 1);
    assert!(store.get_task(&t2.task_id).is_none());
}

// ── Reorder TaskAgents ─────────────────────────────────────

#[test]
fn reorder_task_agents_updates_order() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Reorder");
    let a = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    let b = store.add_task_agent(&task.task_id, Provider::Codex, "coder");
    let c = store.add_task_agent(&task.task_id, Provider::Claude, "reviewer");
    // Reverse the order
    let new_order = vec![c.agent_id.clone(), b.agent_id.clone(), a.agent_id.clone()];
    assert!(store.reorder_task_agents(&task.task_id, &new_order));
    let agents = store.agents_for_task(&task.task_id);
    assert_eq!(agents[0].agent_id, c.agent_id);
    assert_eq!(agents[1].agent_id, b.agent_id);
    assert_eq!(agents[2].agent_id, a.agent_id);
    assert_eq!(agents[0].order, 0);
    assert_eq!(agents[1].order, 1);
    assert_eq!(agents[2].order, 2);
}

#[test]
fn reorder_task_agents_rejects_unknown_agent_id() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Reorder");
    let a = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    let ids = vec![a.agent_id.clone(), "nonexistent".to_string()];
    assert!(!store.reorder_task_agents(&task.task_id, &ids));
}

#[test]
fn reorder_task_agents_rejects_wrong_task() {
    let mut store = TaskGraphStore::new();
    let t1 = store.create_task("/ws", "T1");
    let t2 = store.create_task("/ws", "T2");
    let a = store.add_task_agent(&t1.task_id, Provider::Claude, "lead");
    let b = store.add_task_agent(&t2.task_id, Provider::Codex, "coder");
    // b belongs to t2, not t1
    assert!(!store.reorder_task_agents(&t1.task_id, &[a.agent_id, b.agent_id]));
}

// ── Update TaskAgent ──────────────────────────────────────

#[test]
fn update_task_agent_changes_fields() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Update");
    let agent = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    assert!(store.update_task_agent(
        &agent.agent_id,
        Provider::Codex,
        "architect",
        Some("My Architect".to_string()),
    ));
    let updated = store.get_task_agent(&agent.agent_id).unwrap();
    assert_eq!(updated.provider, Provider::Codex);
    assert_eq!(updated.role, "architect");
    assert_eq!(updated.display_name.as_deref(), Some("My Architect"));
}

#[test]
fn update_task_agent_returns_false_for_missing() {
    let mut store = TaskGraphStore::new();
    assert!(!store.update_task_agent("nonexistent", Provider::Claude, "lead", None));
}

#[test]
fn update_task_agent_clears_display_name() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "Clear");
    let agent = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    store.update_task_agent(&agent.agent_id, Provider::Claude, "lead", Some("Named".into()));
    assert_eq!(store.get_task_agent(&agent.agent_id).unwrap().display_name.as_deref(), Some("Named"));
    store.update_task_agent(&agent.agent_id, Provider::Claude, "lead", None);
    assert!(store.get_task_agent(&agent.agent_id).unwrap().display_name.is_none());
}

// ── Persistence with TaskAgents ────────────────────────────

#[test]
fn persist_task_agents_round_trip() {
    let path = tmp_db_path("agents_roundtrip");
    let _cleanup = CleanupFile(path.clone());
    {
        let mut store = TaskGraphStore::open(&path).unwrap();
        let task = store.create_task("/ws", "Persist Agents");
        store.add_task_agent(&task.task_id, Provider::Claude, "lead");
        store.add_task_agent(&task.task_id, Provider::Codex, "coder");
        store.save().unwrap();
    }
    {
        let store = TaskGraphStore::open(&path).unwrap();
        let tasks = store.list_tasks();
        assert_eq!(tasks.len(), 1);
        let agents = store.agents_for_task(&tasks[0].task_id);
        assert_eq!(agents.len(), 2);
    }
}

#[test]
fn persist_migration_runs_on_old_format_load() {
    let path = tmp_db_path("migration_on_load");
    let _cleanup = CleanupFile(path.clone());
    let task_id;
    // Write a store with sessions (occupancy evidence)
    {
        let mut store = TaskGraphStore::open(&path).unwrap();
        let task = store.create_task_with_config(
            "/ws", "OldFormat", Provider::Codex, Provider::Claude,
        );
        task_id = task.task_id.clone();
        let lead_sess = store.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: None,
            provider: Provider::Codex,
            role: SessionRole::Lead,
            cwd: "/ws",
            title: "Lead",
            agent_id: None,
        });
        store.set_lead_session(&task.task_id, &lead_sess.session_id);
        let coder_sess = store.create_session(CreateSessionParams {
            task_id: &task.task_id,
            parent_session_id: Some(&lead_sess.session_id),
            provider: Provider::Claude,
            role: SessionRole::Coder,
            cwd: "/ws",
            title: "Coder",
            agent_id: None,
        });
        store.set_coder_session(&task.task_id, &coder_sess.session_id);
        store.save().unwrap();
    }
    // Remove agents from SQLite to simulate old format without agents
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute("DELETE FROM task_agents", []).unwrap();
    }
    // Reopen — open() triggers migrate_legacy_agents via session evidence
    {
        let store = TaskGraphStore::open(&path).unwrap();
        let agents = store.agents_for_task(&task_id);
        assert_eq!(agents.len(), 2);
        let lead = agents.iter().find(|a| a.role == "lead").unwrap();
        assert_eq!(lead.provider, Provider::Codex);
    }
}

// ── TaskAgent.model / .effort persistence ──────────────────

#[test]
fn add_task_agent_with_config_persists_model_and_effort() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let agent = store.add_task_agent_with_config(
        &task.task_id,
        Provider::Codex,
        "lead",
        Some("gpt-5-codex".into()),
        Some("high".into()),
    );
    let fetched = store.get_task_agent(&agent.agent_id).unwrap();
    assert_eq!(fetched.model.as_deref(), Some("gpt-5-codex"));
    assert_eq!(fetched.effort.as_deref(), Some("high"));
}

#[test]
fn add_task_agent_default_has_null_model_and_effort() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let agent = store.add_task_agent(&task.task_id, Provider::Claude, "lead");
    let fetched = store.get_task_agent(&agent.agent_id).unwrap();
    assert!(fetched.model.is_none());
    assert!(fetched.effort.is_none());
}

#[test]
fn update_task_agent_with_config_overwrites_model_and_effort() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let agent = store.add_task_agent_with_config(
        &task.task_id,
        Provider::Codex,
        "lead",
        Some("gpt-5".into()),
        Some("low".into()),
    );
    assert!(store.update_task_agent_with_config(
        &agent.agent_id,
        Provider::Codex,
        "lead",
        None,
        Some("gpt-5-codex".into()),
        Some("high".into()),
    ));
    let fetched = store.get_task_agent(&agent.agent_id).unwrap();
    assert_eq!(fetched.model.as_deref(), Some("gpt-5-codex"));
    assert_eq!(fetched.effort.as_deref(), Some("high"));
}

#[test]
fn update_task_agent_with_config_clears_model_and_effort() {
    let mut store = TaskGraphStore::new();
    let task = store.create_task("/ws", "T1");
    let agent = store.add_task_agent_with_config(
        &task.task_id,
        Provider::Codex,
        "lead",
        Some("gpt-5".into()),
        Some("low".into()),
    );
    assert!(store.update_task_agent_with_config(
        &agent.agent_id,
        Provider::Codex,
        "lead",
        None,
        None,
        None,
    ));
    let fetched = store.get_task_agent(&agent.agent_id).unwrap();
    assert!(fetched.model.is_none());
    assert!(fetched.effort.is_none());
}

#[test]
fn persist_task_agent_model_effort_round_trip() {
    let path = tmp_db_path("agent_model_effort_roundtrip");
    let _cleanup = CleanupFile(path.clone());
    let agent_id;
    {
        let mut store = TaskGraphStore::open(&path).unwrap();
        let task = store.create_task("/ws", "T1");
        let agent = store.add_task_agent_with_config(
            &task.task_id,
            Provider::Codex,
            "lead",
            Some("gpt-5-codex".into()),
            Some("medium".into()),
        );
        agent_id = agent.agent_id.clone();
        store.save().unwrap();
    }
    {
        let store = TaskGraphStore::open(&path).unwrap();
        let fetched = store.get_task_agent(&agent_id).unwrap();
        assert_eq!(fetched.model.as_deref(), Some("gpt-5-codex"));
        assert_eq!(fetched.effort.as_deref(), Some("medium"));
    }
}

#[test]
fn migration_adds_model_and_effort_to_v1_schema() {
    let path = tmp_db_path("migration_v1_to_v2");
    let _cleanup = CleanupFile(path.clone());
    // Create a legacy v1 schema by hand (no model / effort columns).
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE tasks (
                task_id TEXT PRIMARY KEY, project_root TEXT NOT NULL,
                task_worktree_root TEXT NOT NULL, title TEXT NOT NULL,
                status TEXT NOT NULL, lead_session_id TEXT,
                current_coder_session_id TEXT, lead_provider TEXT NOT NULL,
                coder_provider TEXT NOT NULL, created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
             );
             CREATE TABLE sessions (
                session_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                parent_session_id TEXT, provider TEXT NOT NULL, role TEXT NOT NULL,
                external_session_id TEXT, transcript_path TEXT, agent_id TEXT,
                status TEXT NOT NULL, cwd TEXT NOT NULL, title TEXT NOT NULL,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
             );
             CREATE TABLE artifacts (
                artifact_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                session_id TEXT NOT NULL, kind TEXT NOT NULL, title TEXT NOT NULL,
                content_ref TEXT NOT NULL, created_at INTEGER NOT NULL
             );
             CREATE TABLE task_agents (
                agent_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                provider TEXT NOT NULL, role TEXT NOT NULL,
                display_name TEXT, sort_order INTEGER NOT NULL,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
             );
             CREATE TABLE buffered_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT, payload TEXT NOT NULL
             );
             INSERT INTO meta(key, value) VALUES ('schema_version', '1');
             INSERT INTO task_agents
                (agent_id, task_id, provider, role, display_name,
                 sort_order, created_at, updated_at)
             VALUES ('agent_legacy', 'task_legacy', 'claude', 'lead',
                     NULL, 0, 1, 1);",
        )
        .unwrap();
    }
    // Open via TaskGraphStore — migrate_if_needed runs in init_schema.
    let store = TaskGraphStore::open(&path).unwrap();
    let fetched = store.get_task_agent("agent_legacy").unwrap();
    assert_eq!(fetched.provider, Provider::Claude);
    assert!(fetched.model.is_none());
    assert!(fetched.effort.is_none());
}

// ── ProviderAuth CRUD + persistence ────────────────────────

fn sample_codex_auth() -> ProviderAuthConfig {
    ProviderAuthConfig {
        provider: "codex".into(),
        api_key: Some("sk-or-abc".into()),
        base_url: Some("https://openrouter.ai/api/v1".into()),
        wire_api: Some("chat".into()),
        auth_mode: None,
        provider_name: Some("dimweave-openrouter".into()),
        active_mode: None,
        updated_at: 0,
    }
}

#[test]
fn upsert_provider_auth_inserts_new_row() {
    let mut store = TaskGraphStore::new();
    store.upsert_provider_auth(sample_codex_auth());
    let fetched = store.get_provider_auth("codex").unwrap();
    assert_eq!(fetched.api_key.as_deref(), Some("sk-or-abc"));
    assert_eq!(fetched.provider_name.as_deref(), Some("dimweave-openrouter"));
    assert!(fetched.updated_at > 0);
}

#[test]
fn upsert_provider_auth_replaces_existing_row() {
    let mut store = TaskGraphStore::new();
    store.upsert_provider_auth(sample_codex_auth());
    store.upsert_provider_auth(ProviderAuthConfig {
        provider: "codex".into(),
        api_key: Some("sk-new".into()),
        base_url: None,
        wire_api: None,
        auth_mode: None,
        provider_name: None,
        active_mode: None,
        updated_at: 0,
    });
    let fetched = store.get_provider_auth("codex").unwrap();
    assert_eq!(fetched.api_key.as_deref(), Some("sk-new"));
    assert!(fetched.base_url.is_none());
}

#[test]
fn clear_provider_auth_removes_row() {
    let mut store = TaskGraphStore::new();
    store.upsert_provider_auth(sample_codex_auth());
    assert!(store.clear_provider_auth("codex"));
    assert!(store.get_provider_auth("codex").is_none());
    assert!(!store.clear_provider_auth("codex"));
}

#[test]
fn provider_auth_round_trip_survives_reopen() {
    let path = tmp_db_path("provider_auth_roundtrip");
    let _cleanup = CleanupFile(path.clone());
    {
        let mut store = TaskGraphStore::open(&path).unwrap();
        store.upsert_provider_auth(sample_codex_auth());
        store.upsert_provider_auth(ProviderAuthConfig {
            provider: "claude".into(),
            api_key: Some("sk-ant-xyz".into()),
            base_url: None,
            wire_api: None,
            auth_mode: Some("bearer".into()),
            provider_name: None,
            active_mode: None,
            updated_at: 0,
        });
        store.save().unwrap();
    }
    let store = TaskGraphStore::open(&path).unwrap();
    let codex = store.get_provider_auth("codex").unwrap();
    assert_eq!(codex.base_url.as_deref(), Some("https://openrouter.ai/api/v1"));
    let claude = store.get_provider_auth("claude").unwrap();
    assert_eq!(claude.auth_mode.as_deref(), Some("bearer"));
    assert!(claude.base_url.is_none());
}

#[test]
fn migration_v2_to_v3_creates_provider_auth_table() {
    let path = tmp_db_path("migration_v2_to_v3");
    let _cleanup = CleanupFile(path.clone());
    // Create a v2 schema by hand (task_agents has model/effort; no provider_auth).
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE tasks (task_id TEXT PRIMARY KEY, project_root TEXT NOT NULL,
                task_worktree_root TEXT NOT NULL, title TEXT NOT NULL,
                status TEXT NOT NULL, lead_session_id TEXT,
                current_coder_session_id TEXT, lead_provider TEXT NOT NULL,
                coder_provider TEXT NOT NULL, created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL);
             CREATE TABLE sessions (session_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                parent_session_id TEXT, provider TEXT NOT NULL, role TEXT NOT NULL,
                external_session_id TEXT, transcript_path TEXT, agent_id TEXT,
                status TEXT NOT NULL, cwd TEXT NOT NULL, title TEXT NOT NULL,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
             CREATE TABLE artifacts (artifact_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                session_id TEXT NOT NULL, kind TEXT NOT NULL, title TEXT NOT NULL,
                content_ref TEXT NOT NULL, created_at INTEGER NOT NULL);
             CREATE TABLE task_agents (agent_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                provider TEXT NOT NULL, role TEXT NOT NULL, display_name TEXT,
                sort_order INTEGER NOT NULL, created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL, model TEXT, effort TEXT);
             CREATE TABLE buffered_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, payload TEXT NOT NULL);
             INSERT INTO meta(key, value) VALUES ('schema_version', '2');",
        )
        .unwrap();
    }
    // Open triggers v2→v3 migration.
    let store = TaskGraphStore::open(&path).unwrap();
    assert!(store.get_provider_auth("codex").is_none());
    // Confirm the table now exists by upserting via the store.
    let mut store = store;
    store.upsert_provider_auth(sample_codex_auth());
    store.save().unwrap();
    let reopened = TaskGraphStore::open(&path).unwrap();
    assert_eq!(
        reopened.get_provider_auth("codex").unwrap().api_key.as_deref(),
        Some("sk-or-abc")
    );
}

#[test]
fn migration_v3_to_v4_adds_active_mode_column() {
    let path = tmp_db_path("migration_v3_to_v4");
    let _cleanup = CleanupFile(path.clone());
    // Build a v3 provider_auth table by hand (no active_mode column).
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE tasks (task_id TEXT PRIMARY KEY, project_root TEXT NOT NULL,
                task_worktree_root TEXT NOT NULL, title TEXT NOT NULL,
                status TEXT NOT NULL, lead_session_id TEXT,
                current_coder_session_id TEXT, lead_provider TEXT NOT NULL,
                coder_provider TEXT NOT NULL, created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL);
             CREATE TABLE sessions (session_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                parent_session_id TEXT, provider TEXT NOT NULL, role TEXT NOT NULL,
                external_session_id TEXT, transcript_path TEXT, agent_id TEXT,
                status TEXT NOT NULL, cwd TEXT NOT NULL, title TEXT NOT NULL,
                created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL);
             CREATE TABLE artifacts (artifact_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                session_id TEXT NOT NULL, kind TEXT NOT NULL, title TEXT NOT NULL,
                content_ref TEXT NOT NULL, created_at INTEGER NOT NULL);
             CREATE TABLE task_agents (agent_id TEXT PRIMARY KEY, task_id TEXT NOT NULL,
                provider TEXT NOT NULL, role TEXT NOT NULL, display_name TEXT,
                sort_order INTEGER NOT NULL, created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL, model TEXT, effort TEXT);
             CREATE TABLE buffered_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, payload TEXT NOT NULL);
             CREATE TABLE provider_auth (
                provider TEXT PRIMARY KEY, api_key TEXT, base_url TEXT,
                wire_api TEXT, auth_mode TEXT, provider_name TEXT,
                updated_at INTEGER NOT NULL
             );
             INSERT INTO meta(key, value) VALUES ('schema_version', '3');
             INSERT INTO provider_auth
                (provider, api_key, base_url, wire_api, auth_mode, provider_name, updated_at)
             VALUES ('codex', 'sk-legacy', NULL, NULL, NULL, NULL, 1);",
        )
        .unwrap();
    }
    // Open triggers v3→v4 migration (ALTER TABLE ADD COLUMN active_mode).
    let store = TaskGraphStore::open(&path).unwrap();
    let row = store.get_provider_auth("codex").unwrap();
    assert_eq!(row.api_key.as_deref(), Some("sk-legacy"));
    assert!(row.active_mode.is_none());
}
