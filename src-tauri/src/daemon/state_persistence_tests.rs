use super::DaemonState;
use std::path::PathBuf;

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "dimweave_persist_{}_{}.sqlite",
        label,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
    ))
}

/// Count rows in the `tasks` table on disk. Returns 0 if table missing.
fn task_row_count(path: &std::path::Path) -> usize {
    let Ok(conn) = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    ) else {
        return 0;
    };
    conn.query_row("SELECT COUNT(*) FROM tasks", [], |row| row.get::<_, i64>(0))
        .map(|n| n as usize)
        .unwrap_or(0)
}

/// Contract 1: `create_and_select_task` does NOT flush task rows to disk.
/// The handler is responsible for the single authoritative save.
///
/// Note: SQLite creates the DB file on `open()` (empty schema), so we check
/// task row count on disk rather than file existence.
#[test]
fn create_task_does_not_auto_persist() {
    let path = temp_path("no_auto");
    let _ = std::fs::remove_file(&path);

    let mut s = DaemonState::with_task_graph_path(path.clone()).unwrap();
    // DB file now exists (empty schema). Row count must stay at 0 after
    // in-memory task creation until someone explicitly saves.
    assert_eq!(
        task_row_count(&path),
        0,
        "empty DB must have 0 task rows before any save"
    );
    let _task = s.create_and_select_task("/ws", "T1");
    assert_eq!(
        task_row_count(&path),
        0,
        "create_and_select_task must not auto-save task rows"
    );
    let _ = std::fs::remove_file(&path);
}

/// Contract 2: explicit save succeeds and returns Ok.
#[test]
fn save_task_graph_returns_ok_on_success() {
    let path = temp_path("ok");
    let _ = std::fs::remove_file(&path);

    let mut s = DaemonState::with_task_graph_path(path.clone()).unwrap();
    let _task = s.create_and_select_task("/ws", "T1");
    assert!(s.save_task_graph().is_ok());
    assert!(path.exists());
    assert_eq!(task_row_count(&path), 1, "one task should be persisted");
    let _ = std::fs::remove_file(&path);
}

/// Contract 3: `save_task_graph` propagates errors when the write target
/// is unreachable.
///
/// SQLite's on-disk semantics make OS-permission-based failure tests
/// brittle: `journal_mode=WAL` opens a long-lived connection whose fd
/// bypasses post-open `chmod`, WAL/SHM siblings buffer writes, and macOS
/// SIP/APFS sometimes refuses dir-level `chmod 0o555` under `/var/folders`.
/// None of that is a property of our save code — it's SQLite's writer
/// robustness.
///
/// Instead of simulating disk failure, we exercise the error-propagation
/// path directly: attach a second `DaemonState` to the same DB path so the
/// SQLite busy-timeout fires, and assert `save_task_graph` surfaces the
/// resulting `SQLITE_BUSY` error. This is the actual production failure
/// mode for "unwritable target" (exclusive lock held by another writer).
#[test]
fn save_task_graph_returns_err_on_unwritable_path() {
    let path = temp_path("busylock");
    let _ = std::fs::remove_file(&path);

    // Open writer B FIRST so `init_schema` (which requires a writer lock)
    // completes before we introduce contention.
    let mut writer_b = DaemonState::with_task_graph_path(path.clone()).unwrap();
    writer_b.save_task_graph().unwrap(); // ensure WAL + tables exist

    // Second raw SQLite connection holds an EXCLUSIVE transaction open,
    // simulating an unwritable path / stuck writer.
    //
    // Note: `busy_timeout` is per-connection. writer_b's cached connection
    // inherits the 5000ms timeout set by `TaskGraphStore::open`, which this
    // test cannot shrink from outside (would require a test-only pragma
    // hook on `TaskGraphStore`). The test therefore waits up to ~5s for
    // SQLITE_BUSY to surface — slow but bounded and correct.
    let contender = rusqlite::Connection::open(&path).unwrap();

    // BEGIN IMMEDIATE grabs the reserved-writer lock without needing any
    // actual row-level write; that's enough to make writer_b's next save
    // hit SQLITE_BUSY and fail.
    contender.execute_batch("BEGIN IMMEDIATE").unwrap();

    // Writer B tries to commit a fresh task while contender holds the
    // writer lock. save_task_graph propagates the BUSY error as Err.
    let _task = writer_b.create_and_select_task("/ws", "T_contention");
    let result = writer_b.save_task_graph();

    // Release the contender's lock before cleanup.
    let _ = contender.execute_batch("ROLLBACK");
    drop(contender);

    let _ = std::fs::remove_file(&path);
    let wal = path.with_extension("sqlite-wal");
    let shm = path.with_extension("sqlite-shm");
    let _ = std::fs::remove_file(&wal);
    let _ = std::fs::remove_file(&shm);

    assert!(
        result.is_err(),
        "save_task_graph must propagate SQLITE_BUSY as Err, got Ok"
    );
}

/// Contract 4: `select_task` does NOT persist.
#[test]
fn select_task_does_not_persist() {
    let path = temp_path("select");
    let _ = std::fs::remove_file(&path);

    let mut s = DaemonState::with_task_graph_path(path.clone()).unwrap();
    let task = s.create_and_select_task("/ws", "T1");
    s.save_task_graph().unwrap();
    let rows_after_save = task_row_count(&path);

    // select_task must NOT trigger a re-save (no additional row churn).
    s.select_task(&task.task_id).unwrap();
    assert_eq!(
        task_row_count(&path),
        rows_after_save,
        "select_task must not auto-save"
    );
    let _ = std::fs::remove_file(&path);
}
