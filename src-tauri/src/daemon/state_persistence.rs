use super::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
struct DaemonPersistedSnapshot {
    task_graph: serde_json::Value,
    #[serde(default)]
    buffered_messages: Vec<BridgeMessage>,
}

impl DaemonState {
    /// Create with the daemon snapshot loaded from the given path.
    ///
    /// The persisted file now contains the task graph plus buffered delivery
    /// and review-gate state. Older task-graph-only files still load via the
    /// legacy fallback in `load_persisted_state`.
    pub fn with_task_graph_path(path: PathBuf) -> anyhow::Result<Self> {
        Self::load_persisted_state(&path)
    }

    /// Persist the full daemon snapshot to disk (no-op if no path configured).
    ///
    /// The historical method name is kept for call-site compatibility even
    /// though this now writes more than the task graph alone.
    pub fn save_task_graph(&self) -> anyhow::Result<()> {
        let Some(path) = self.task_graph.persist_path() else {
            return Ok(());
        };
        let snapshot = DaemonPersistedSnapshot {
            task_graph: self.task_graph.to_snapshot_json()?,
            buffered_messages: self.persisted_buffered_messages(),
        };
        let json = serde_json::to_string_pretty(&snapshot)?;
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

    /// Best-effort auto-save after mutations.
    pub(crate) fn auto_save_task_graph(&self) {
        if let Err(e) = self.save_task_graph() {
            eprintln!("[Daemon] task graph auto-save failed: {e}");
        }
    }

    fn load_persisted_state(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self {
                task_graph: TaskGraphStore::with_persist_path(path.to_path_buf()),
                ..Self::default()
            });
        }

        let data = std::fs::read_to_string(path)?;
        if let Ok(snapshot) = serde_json::from_str::<DaemonPersistedSnapshot>(&data) {
            let mut state = Self {
                task_graph: TaskGraphStore::from_snapshot_json(
                    snapshot.task_graph,
                    Some(path.to_path_buf()),
                )?,
                ..Self::default()
            };
            state.restore_persisted_buffered_messages(snapshot.buffered_messages);
            return Ok(state);
        }

        Ok(Self {
            task_graph: TaskGraphStore::load(path)?,
            ..Self::default()
        })
    }
}
