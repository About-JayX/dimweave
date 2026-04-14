use super::*;
use std::path::{Path, PathBuf};

impl DaemonState {
    /// Create with the task graph loaded from the given SQLite path.
    pub fn with_task_graph_path(path: PathBuf) -> anyhow::Result<Self> {
        let task_graph = TaskGraphStore::open(&path)?;
        Ok(Self {
            task_graph,
            ..Self::default()
        })
    }

    /// Persist the full daemon snapshot (no-op if no db configured).
    pub fn save_task_graph(&self) -> anyhow::Result<()> {
        self.task_graph.save()
    }

    /// Best-effort auto-save after mutations.
    pub(crate) fn auto_save_task_graph(&self) {
        if let Err(e) = self.save_task_graph() {
            eprintln!("[Daemon] task graph auto-save failed: {e}");
        }
    }

    /// Return the database path (for callers that need it).
    pub fn task_graph_db_path(&self) -> Option<&Path> {
        self.task_graph.db_path()
    }
}
