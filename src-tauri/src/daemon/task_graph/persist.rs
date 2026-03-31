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
    pub next_id: u64,
}

impl TaskGraphStore {
    /// Convert the live store into a serializable snapshot.
    pub(super) fn to_snapshot(&self) -> TaskGraphSnapshot {
        TaskGraphSnapshot {
            tasks: self.tasks.values().cloned().collect(),
            sessions: self.sessions.values().cloned().collect(),
            artifacts: self.artifacts.values().cloned().collect(),
            next_id: self.next_id,
        }
    }

    /// Rebuild a store from a deserialized snapshot.
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
        Self {
            tasks,
            sessions,
            artifacts,
            next_id: snap.next_id,
            persist_path,
        }
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
        std::fs::write(path, json)?;
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
