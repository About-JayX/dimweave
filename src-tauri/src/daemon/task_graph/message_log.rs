//! Per-task chat message persistence. Bridges `BridgeMessage` ↔ the
//! `task_messages` SQLite table so the unified dimweave routing transcript
//! survives app restarts. Not to be confused with `buffered_messages`,
//! which holds undelivered routing payloads.

use rusqlite::params;

use super::store::TaskGraphStore;
use crate::daemon::types::BridgeMessage;

/// Cap how many messages we return per task on hydrate. A long-running
/// chat can accumulate thousands; the UI only needs recent ones and the
/// main message-panel viewport is virtualized anyway.
const MAX_MESSAGES_PER_TASK_LOAD: usize = 500;

impl TaskGraphStore {
    /// Persist a chat message. No-op when the store has no db connection
    /// (in-memory tests) or when `msg.task_id` is missing — untied messages
    /// are system diagnostics we don't want to clutter the transcript with.
    pub fn persist_task_message(&self, msg: &BridgeMessage) {
        let Some(db) = &self.db else { return };
        let Some(task_id) = msg.task_id.as_deref() else {
            return;
        };
        let payload = match serde_json::to_string(msg) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[task_messages] serialize failed for {}: {e}", msg.id);
                return;
            }
        };
        let Ok(conn) = db.lock() else {
            eprintln!("[task_messages] db mutex poisoned");
            return;
        };
        let res = conn.execute(
            "INSERT OR REPLACE INTO task_messages (id, task_id, payload, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            params![msg.id, task_id, payload, msg.timestamp as i64],
        );
        if let Err(e) = res {
            eprintln!("[task_messages] insert failed for {}: {e}", msg.id);
        }
    }

    /// Load the most recent messages for a task in chronological (oldest →
    /// newest) order. Empty vec if no db or no matches.
    pub fn list_task_messages(&self, task_id: &str) -> Vec<BridgeMessage> {
        let Some(db) = &self.db else { return vec![] };
        let Ok(conn) = db.lock() else { return vec![] };
        let mut stmt = match conn.prepare(
            "SELECT payload FROM task_messages \
             WHERE task_id = ?1 \
             ORDER BY created_at DESC, id DESC \
             LIMIT ?2",
        ) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[task_messages] prepare failed: {e}");
                return vec![];
            }
        };
        let rows = stmt.query_map(
            params![task_id, MAX_MESSAGES_PER_TASK_LOAD as i64],
            |r| r.get::<_, String>(0),
        );
        let Ok(rows) = rows else { return vec![] };
        let mut out: Vec<BridgeMessage> = rows
            .filter_map(|r| r.ok())
            .filter_map(|payload| serde_json::from_str::<BridgeMessage>(&payload).ok())
            .collect();
        out.reverse();
        out
    }

    /// Drop all persisted messages for a task. Called on task deletion.
    pub fn delete_task_messages(&self, task_id: &str) {
        let Some(db) = &self.db else { return };
        let Ok(conn) = db.lock() else { return };
        let _ = conn.execute(
            "DELETE FROM task_messages WHERE task_id = ?1",
            params![task_id],
        );
    }
}
