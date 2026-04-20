//! Alignment contract tests for `task_messages` table.
//!
//! Motivation: messages now live in two independent stores — our
//! `task_messages` table (GUI timeline) and the agent's own transcript
//! (`~/.claude/...jsonl` / `~/.codex/sessions/...jsonl`). On reconnect
//! the agent resumes from its own transcript; our DB is just for UI
//! restore. Both writes must happen per routed message or the two
//! views diverge.
//!
//! These tests pin down **our side of the alignment contract**:
//! - every call site that decides to persist actually writes the row
//! - write is keyed by id (idempotent re-persist on same id)
//! - reads return chronological order
//! - per-task purge is surgical
//! - no-op when task_id is absent (system diagnostics don't pollute)
//!
//! What we explicitly don't test here: that Codex/Claude subprocesses
//! wrote to their own transcript for the same message. That's their
//! contract, exercised by live E2E.

use super::store::TaskGraphStore;
use crate::daemon::types::{BridgeMessage, MessageSource, MessageTarget};
use std::path::PathBuf;

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "dimweave_msg_log_{}_{}.sqlite",
        label,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
    ))
}

fn make_msg(id: &str, task_id: Option<&str>, timestamp: u64, body: &str) -> BridgeMessage {
    BridgeMessage {
        id: id.into(),
        source: MessageSource::User,
        target: MessageTarget::User,
        reply_target: None,
        message: body.into(),
        timestamp,
        reply_to: None,
        priority: None,
        status: None,
        task_id: task_id.map(String::from),
        session_id: None,
        attachments: None,
    }
}

#[test]
fn persist_and_list_roundtrip() {
    let path = temp_path("roundtrip");
    let _ = std::fs::remove_file(&path);
    let store = TaskGraphStore::open(&path).unwrap();

    store.persist_task_message(&make_msg("m1", Some("t1"), 1000, "hello"));
    let out = store.list_task_messages("t1");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].id, "m1");
    assert_eq!(out[0].message, "hello");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn list_returns_chronological_oldest_first() {
    let path = temp_path("chrono");
    let _ = std::fs::remove_file(&path);
    let store = TaskGraphStore::open(&path).unwrap();

    // Insert out of order.
    store.persist_task_message(&make_msg("m3", Some("t1"), 3000, "third"));
    store.persist_task_message(&make_msg("m1", Some("t1"), 1000, "first"));
    store.persist_task_message(&make_msg("m2", Some("t1"), 2000, "second"));

    let out = store.list_task_messages("t1");
    let ids: Vec<&str> = out.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, vec!["m1", "m2", "m3"]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn persist_without_task_id_is_noop() {
    let path = temp_path("noop");
    let _ = std::fs::remove_file(&path);
    let store = TaskGraphStore::open(&path).unwrap();

    // System diagnostic without task_id — must not be persisted into
    // any task bucket (we'd have nowhere to read it from anyway).
    store.persist_task_message(&make_msg("sys_1", None, 500, "diagnostic"));
    // Also no task_id should be quietly cross-persisted elsewhere.
    assert_eq!(store.list_task_messages("").len(), 0);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn persist_by_id_is_idempotent() {
    let path = temp_path("idempotent");
    let _ = std::fs::remove_file(&path);
    let store = TaskGraphStore::open(&path).unwrap();

    store.persist_task_message(&make_msg("m1", Some("t1"), 1000, "v1"));
    store.persist_task_message(&make_msg("m1", Some("t1"), 1000, "v2"));
    store.persist_task_message(&make_msg("m1", Some("t1"), 1000, "v3"));

    let out = store.list_task_messages("t1");
    assert_eq!(out.len(), 1, "INSERT OR REPLACE must dedup by id");
    assert_eq!(out[0].message, "v3", "latest write wins");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn list_is_scoped_per_task_id() {
    let path = temp_path("scoped");
    let _ = std::fs::remove_file(&path);
    let store = TaskGraphStore::open(&path).unwrap();

    store.persist_task_message(&make_msg("a1", Some("ta"), 100, "a"));
    store.persist_task_message(&make_msg("b1", Some("tb"), 100, "b"));

    let a = store.list_task_messages("ta");
    let b = store.list_task_messages("tb");
    assert_eq!(a.len(), 1);
    assert_eq!(b.len(), 1);
    assert_eq!(a[0].id, "a1");
    assert_eq!(b[0].id, "b1");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_is_surgical_per_task() {
    let path = temp_path("delete");
    let _ = std::fs::remove_file(&path);
    let store = TaskGraphStore::open(&path).unwrap();

    store.persist_task_message(&make_msg("a1", Some("ta"), 100, "a"));
    store.persist_task_message(&make_msg("a2", Some("ta"), 200, "a2"));
    store.persist_task_message(&make_msg("b1", Some("tb"), 100, "b"));

    store.delete_task_messages("ta");

    assert_eq!(store.list_task_messages("ta").len(), 0);
    // tb must not be touched.
    assert_eq!(store.list_task_messages("tb").len(), 1);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn list_survives_store_reopen_for_restart_recovery() {
    let path = temp_path("reopen");
    let _ = std::fs::remove_file(&path);

    {
        let store = TaskGraphStore::open(&path).unwrap();
        for i in 0..5 {
            store.persist_task_message(&make_msg(
                &format!("m{i}"),
                Some("t1"),
                (i as u64) * 100,
                &format!("msg {i}"),
            ));
        }
    } // drop store — simulate app shutdown

    // Reopen (simulates app restart) and assert messages are still there.
    let store2 = TaskGraphStore::open(&path).unwrap();
    let out = store2.list_task_messages("t1");
    assert_eq!(out.len(), 5);
    let ids: Vec<&str> = out.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(ids, vec!["m0", "m1", "m2", "m3", "m4"]);
    let _ = std::fs::remove_file(&path);
}
