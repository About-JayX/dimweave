use super::*;
use std::path::{Path, PathBuf};

impl DaemonState {
    /// Create with the task graph loaded from the given SQLite path.
    /// Also restores any persisted buffered messages.
    pub fn with_task_graph_path(path: PathBuf) -> anyhow::Result<Self> {
        let task_graph = TaskGraphStore::open(&path)?;
        let mut state = Self {
            task_graph,
            ..Self::default()
        };
        state.load_buffered_messages();
        Ok(state)
    }

    /// Persist the full daemon snapshot (no-op if no db configured).
    pub fn save_task_graph(&self) -> anyhow::Result<()> {
        self.task_graph.save()?;
        self.save_buffered_messages()
    }

    /// Best-effort auto-save after mutations.
    ///
    /// Production path (save_tx wired by `daemon::run`): signals a
    /// debouncing saver task that coalesces bursts into a single SQLite
    /// write per ~200ms. The channel is unbounded and the signal payload
    /// is zero-sized, so this is near-free for callers under write lock.
    ///
    /// Test / standalone path (save_tx is None): fall back to synchronous
    /// `save_to_db`. Unit tests exercise state mutations without spinning
    /// up the saver task and still need disk writes to land immediately.
    pub(crate) fn auto_save_task_graph(&self) {
        if let Some(tx) = &self.save_tx {
            // send() on UnboundedSender only fails if receiver dropped,
            // which means the saver task shut down. Fall through to sync
            // save so the mutation still lands instead of being lost.
            if tx.send(()).is_ok() {
                return;
            }
        }
        if let Err(e) = self.save_task_graph() {
            eprintln!("[Daemon] task graph auto-save failed: {e}");
        }
    }

    /// Return the database path (for callers that need it).
    pub fn task_graph_db_path(&self) -> Option<&Path> {
        self.task_graph.db_path()
    }

    fn save_buffered_messages(&self) -> anyhow::Result<()> {
        let Some(db_path) = self.task_graph.db_path() else {
            return Ok(());
        };
        let conn = rusqlite::Connection::open(db_path)?;
        conn.execute("DELETE FROM buffered_messages", [])?;
        let mut stmt = conn.prepare(
            "INSERT INTO buffered_messages (payload) VALUES (?1)",
        )?;
        for msg in &self.buffered_messages {
            let payload = serde_json::to_string(msg)?;
            stmt.execute(rusqlite::params![payload])?;
        }
        Ok(())
    }

    fn load_buffered_messages(&mut self) {
        let Some(db_path) = self.task_graph.db_path() else {
            return;
        };
        let Ok(conn) = rusqlite::Connection::open(db_path) else {
            return;
        };
        let Ok(mut stmt) = conn.prepare(
            "SELECT payload FROM buffered_messages ORDER BY id",
        ) else {
            return;
        };
        let messages: Vec<BridgeMessage> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .into_iter()
            .flatten()
            .filter_map(|r| r.ok())
            .filter_map(|payload| serde_json::from_str(&payload).ok())
            .collect();
        self.restore_persisted_buffered_messages(messages);
    }
}
