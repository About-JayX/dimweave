use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::types::*;

/// Central in-memory store for the task graph domain.
/// Optionally backed by a SQLite database for persistence.
pub struct TaskGraphStore {
    pub(super) tasks: HashMap<String, Task>,
    pub(super) sessions: HashMap<String, SessionHandle>,
    pub(super) artifacts: HashMap<String, Artifact>,
    pub(super) task_agents: HashMap<String, TaskAgent>,
    pub(super) next_id: u64,
    pub(super) db_path: Option<PathBuf>,
    pub(super) db: Option<std::sync::Mutex<rusqlite::Connection>>,
}

impl TaskGraphStore {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            sessions: HashMap::new(),
            artifacts: HashMap::new(),
            task_agents: HashMap::new(),
            next_id: 0,
            db_path: None,
            db: None,
        }
    }

    /// Return the database path (if backed by SQLite).
    pub fn db_path(&self) -> Option<&Path> {
        self.db_path.as_deref()
    }

    /// Create a new task in Draft status.
    /// `workspace_root` is treated as both the project root and the initial
    /// working directory. If `title` is empty, the generated `task_id` is used.
    pub fn create_task(&mut self, workspace_root: &str, title: &str) -> Task {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let task_id = self.next_id_str("task");
        let effective_title = if title.is_empty() { task_id.clone() } else { title.to_string() };
        let task = Task {
            task_id,
            project_root: workspace_root.to_string(),
            workspace_root: workspace_root.to_string(),
            title: effective_title,
            status: TaskStatus::Draft,
            lead_session_id: None,
            current_coder_session_id: None,
            lead_provider: Provider::Claude,
            coder_provider: Provider::Codex,
            created_at: now,
            updated_at: now,
        };
        self.tasks.insert(task.task_id.clone(), task.clone());
        task
    }

    /// Create a new task with explicit provider bindings.
    /// If `title` is empty, the generated `task_id` is used as the title.
    pub fn create_task_with_config(
        &mut self,
        workspace_root: &str,
        title: &str,
        lead_provider: Provider,
        coder_provider: Provider,
    ) -> Task {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let task_id = self.next_id_str("task");
        let effective_title = if title.is_empty() { task_id.clone() } else { title.to_string() };
        let task = Task {
            task_id,
            project_root: workspace_root.to_string(),
            workspace_root: workspace_root.to_string(),
            title: effective_title,
            status: TaskStatus::Draft,
            lead_session_id: None,
            current_coder_session_id: None,
            lead_provider,
            coder_provider,
            created_at: now,
            updated_at: now,
        };
        self.tasks.insert(task.task_id.clone(), task.clone());
        task
    }

    /// Update provider bindings for a task. Returns false if not found.
    pub fn update_task_providers(
        &mut self,
        task_id: &str,
        lead_provider: Provider,
        coder_provider: Provider,
    ) -> bool {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.lead_provider = lead_provider;
            task.coder_provider = coder_provider;
            task.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Retrieve a task by ID.
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    /// Update task status. Returns false if task not found.
    pub fn update_task_status(&mut self, task_id: &str, status: TaskStatus) -> bool {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.status = status;
            task.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// List all tasks.
    pub fn list_tasks(&self) -> Vec<&Task> {
        self.tasks.values().collect()
    }

    /// Update workspace_root for a task (e.g. after worktree creation).
    pub fn update_workspace_root(&mut self, task_id: &str, workspace_root: &str) -> bool {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.workspace_root = workspace_root.to_string();
            task.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Remove a task by ID. Returns true if the task existed.
    pub fn remove_task(&mut self, task_id: &str) -> bool {
        self.tasks.remove(task_id).is_some()
    }

    /// Set the lead session for a task.
    pub fn set_lead_session(&mut self, task_id: &str, session_id: &str) -> bool {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.lead_session_id = Some(session_id.to_string());
            task.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Set the current coder session for a task.
    pub fn set_coder_session(&mut self, task_id: &str, session_id: &str) -> bool {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.current_coder_session_id = Some(session_id.to_string());
            task.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    pub fn clear_lead_session_if_matches(&mut self, task_id: &str, session_id: &str) -> bool {
        if let Some(task) = self.tasks.get_mut(task_id) {
            if task.lead_session_id.as_deref() == Some(session_id) {
                task.lead_session_id = None;
                task.updated_at = chrono::Utc::now().timestamp_millis() as u64;
                return true;
            }
        }
        false
    }

    pub fn clear_coder_session_if_matches(&mut self, task_id: &str, session_id: &str) -> bool {
        if let Some(task) = self.tasks.get_mut(task_id) {
            if task.current_coder_session_id.as_deref() == Some(session_id) {
                task.current_coder_session_id = None;
                task.updated_at = chrono::Utc::now().timestamp_millis() as u64;
                return true;
            }
        }
        false
    }

    /// Create a new session linked to a task.
    pub fn create_session(&mut self, params: CreateSessionParams) -> SessionHandle {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let session = SessionHandle {
            session_id: self.next_id_str("sess"),
            task_id: params.task_id.to_string(),
            parent_session_id: params.parent_session_id.map(String::from),
            provider: params.provider,
            role: params.role,
            external_session_id: None,
            transcript_path: None,
            agent_id: params.agent_id.map(String::from),
            status: SessionStatus::Active,
            cwd: params.cwd.to_string(),
            title: params.title.to_string(),
            created_at: now,
            updated_at: now,
        };
        self.sessions
            .insert(session.session_id.clone(), session.clone());
        session
    }

    /// Retrieve a session by ID.
    pub fn get_session(&self, session_id: &str) -> Option<&SessionHandle> {
        self.sessions.get(session_id)
    }

    /// Find a session by provider-specific external ID.
    pub fn find_session_by_external_id(
        &self,
        provider: Provider,
        external_id: &str,
    ) -> Option<&SessionHandle> {
        self.sessions.values().find(|session| {
            session.provider == provider
                && session.external_session_id.as_deref() == Some(external_id)
        })
    }

    /// Update session status. Returns false if session not found.
    pub fn update_session_status(&mut self, session_id: &str, status: SessionStatus) -> bool {
        if let Some(sess) = self.sessions.get_mut(session_id) {
            sess.status = status;
            sess.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Bind a provider-specific external ID to a session.
    pub fn set_external_session_id(&mut self, session_id: &str, external_id: &str) -> bool {
        if let Some(sess) = self.sessions.get_mut(session_id) {
            sess.external_session_id = Some(external_id.to_string());
            sess.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Bind a provider-owned transcript path to a session.
    pub fn set_transcript_path(&mut self, session_id: &str, transcript_path: &str) -> bool {
        if let Some(sess) = self.sessions.get_mut(session_id) {
            sess.transcript_path = Some(transcript_path.to_string());
            sess.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Bind an agent_id to a session.
    pub fn set_session_agent_id(&mut self, session_id: &str, agent_id: &str) -> bool {
        if let Some(sess) = self.sessions.get_mut(session_id) {
            sess.agent_id = Some(agent_id.to_string());
            sess.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Add an artifact to the store.
    pub fn add_artifact(&mut self, params: CreateArtifactParams) -> Artifact {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let artifact = Artifact {
            artifact_id: self.next_id_str("art"),
            task_id: params.task_id.to_string(),
            session_id: params.session_id.to_string(),
            kind: params.kind,
            title: params.title.to_string(),
            content_ref: params.content_ref.to_string(),
            created_at: now,
        };
        self.artifacts
            .insert(artifact.artifact_id.clone(), artifact.clone());
        artifact
    }

    /// Retrieve an artifact by ID.
    pub fn get_artifact(&self, artifact_id: &str) -> Option<&Artifact> {
        self.artifacts.get(artifact_id)
    }

    // ── TaskAgent CRUD ─────────────────────────────────────

    /// Add an agent to a task. Order auto-increments within the task.
    pub fn add_task_agent(&mut self, task_id: &str, provider: Provider, role: &str) -> TaskAgent {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let order = self.agents_for_task(task_id).len() as u32;
        let agent = TaskAgent {
            agent_id: self.next_id_str("agent"),
            task_id: task_id.to_string(),
            provider,
            role: role.to_string(),
            display_name: None,
            order,
            created_at: now,
            updated_at: now,
        };
        self.task_agents.insert(agent.agent_id.clone(), agent.clone());
        agent
    }

    /// Retrieve an agent by ID.
    pub fn get_task_agent(&self, agent_id: &str) -> Option<&TaskAgent> {
        self.task_agents.get(agent_id)
    }

    /// All agents belonging to a task, sorted by order.
    pub fn agents_for_task(&self, task_id: &str) -> Vec<&TaskAgent> {
        let mut agents: Vec<_> = self.task_agents.values()
            .filter(|a| a.task_id == task_id)
            .collect();
        agents.sort_by_key(|a| a.order);
        agents
    }

    /// Remove an agent by ID. Returns true if it existed.
    pub fn remove_task_agent(&mut self, agent_id: &str) -> bool {
        self.task_agents.remove(agent_id).is_some()
    }

    /// Reorder agents within a task. `agent_ids` must all belong to `task_id`.
    /// Returns false if any ID is unknown or belongs to another task.
    pub fn reorder_task_agents(&mut self, task_id: &str, agent_ids: &[String]) -> bool {
        // Validate first — no partial mutation on failure
        for aid in agent_ids {
            match self.task_agents.get(aid) {
                Some(a) if a.task_id == task_id => {}
                _ => return false,
            }
        }
        let now = chrono::Utc::now().timestamp_millis() as u64;
        for (i, aid) in agent_ids.iter().enumerate() {
            if let Some(agent) = self.task_agents.get_mut(aid) {
                agent.order = i as u32;
                agent.updated_at = now;
            }
        }
        true
    }

    /// Update an agent's provider, role, and display_name.
    pub fn update_task_agent(
        &mut self,
        agent_id: &str,
        provider: Provider,
        role: &str,
        display_name: Option<String>,
    ) -> bool {
        if let Some(agent) = self.task_agents.get_mut(agent_id) {
            agent.provider = provider;
            agent.role = role.to_string();
            agent.display_name = display_name;
            agent.updated_at = chrono::Utc::now().timestamp_millis() as u64;
            true
        } else {
            false
        }
    }

    /// Deterministic legacy migration: for each task with no agents,
    /// generate agents from singleton provider fields only when
    /// concrete session occupancy evidence exists.
    /// Idempotent — skips tasks that already have agents.
    pub fn migrate_legacy_agents(&mut self) {
        let task_ids: Vec<String> = self.tasks.keys().cloned().collect();
        for task_id in task_ids {
            let has_agents = self.task_agents.values().any(|a| a.task_id == task_id);
            if has_agents {
                continue;
            }
            let task = self.tasks.get(&task_id).unwrap();
            let has_lead = task.lead_session_id.is_some();
            let has_coder = task.current_coder_session_id.is_some();
            let lead_provider = task.lead_provider;
            let coder_provider = task.coder_provider;
            if has_lead {
                self.add_task_agent(&task_id, lead_provider, "lead");
            }
            if has_coder {
                self.add_task_agent(&task_id, coder_provider, "coder");
            }
        }
    }

    pub(super) fn next_id_str(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        let ts = chrono::Utc::now().timestamp_millis() as u64;
        format!("{prefix}_{ts}_{}", self.next_id)
    }
}

impl Default for TaskGraphStore {
    fn default() -> Self {
        Self::new()
    }
}
