use std::path::Path;

use serde::{Deserialize, Serialize};

use super::store::TaskGraphStore;
use super::types::*;

/// Serializable snapshot of the entire task graph.
#[derive(Serialize, Deserialize)]
pub(super) struct TaskGraphSnapshot {
    pub tasks: Vec<Task>,
    pub sessions: Vec<SessionHandle>,
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub task_agents: Vec<TaskAgent>,
    pub next_id: u64,
}

impl TaskGraphStore {
    pub(crate) fn persist_path(&self) -> Option<&Path> {
        self.persist_path.as_deref()
    }

    pub(crate) fn to_snapshot_json(&self) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::to_value(self.to_snapshot())?)
    }

    pub(crate) fn from_snapshot_json(
        value: serde_json::Value,
        persist_path: Option<std::path::PathBuf>,
    ) -> anyhow::Result<Self> {
        let snap: TaskGraphSnapshot = serde_json::from_value(value)?;
        Ok(Self::from_snapshot(snap, persist_path))
    }

    /// Convert the live store into a serializable snapshot.
    pub(super) fn to_snapshot(&self) -> TaskGraphSnapshot {
        TaskGraphSnapshot {
            tasks: self.tasks.values().cloned().collect(),
            sessions: self.sessions.values().cloned().collect(),
            artifacts: self.artifacts.values().cloned().collect(),
            task_agents: self.task_agents.values().cloned().collect(),
            next_id: self.next_id,
        }
    }

    /// Rebuild a store from a deserialized snapshot, running legacy
    /// migration for tasks that have no `TaskAgent` records yet.
    pub(super) fn from_snapshot(
        snap: TaskGraphSnapshot,
        persist_path: Option<std::path::PathBuf>,
    ) -> Self {
        let tasks = snap
            .tasks
            .into_iter()
            .map(|t| (t.task_id.clone(), t))
            .collect();
        let sessions = snap
            .sessions
            .into_iter()
            .map(|s| (s.session_id.clone(), s))
            .collect();
        let artifacts = snap
            .artifacts
            .into_iter()
            .map(|a| (a.artifact_id.clone(), a))
            .collect();
        let task_agents = snap
            .task_agents
            .into_iter()
            .map(|a| (a.agent_id.clone(), a))
            .collect();
        let mut store = Self {
            tasks,
            sessions,
            artifacts,
            task_agents,
            next_id: snap.next_id,
            persist_path,
        };
        store.migrate_legacy_agents();
        store
    }

    /// Save the store to its configured persist path.
    /// No-op if no persist_path is set.
    pub fn save(&self) -> anyhow::Result<()> {
        let Some(path) = &self.persist_path else {
            return Ok(());
        };
        let snap = self.to_snapshot();
        let json = serde_json::to_string_pretty(&snap)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp_path = path.with_extension("tmp");
        std::fs::write(&tmp_path, json)?;
        if let Err(err) = std::fs::rename(&tmp_path, path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(err.into());
        }
        Ok(())
    }

    /// Load from a file path. Returns an empty store if the
    /// file does not exist.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::with_persist_path(path.to_path_buf()));
        }
        let data = std::fs::read_to_string(path)?;
        let snap: TaskGraphSnapshot = serde_json::from_str(&data)?;
        Ok(Self::from_snapshot(snap, Some(path.to_path_buf())))
    }
}
