use super::DaemonState;
use std::path::PathBuf;

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "dimweave_persist_{}_{}.json",
        label,
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
    ))
}

/// Contract 1: create path does NOT auto-persist.
/// The handler is responsible for the single authoritative save.
#[test]
fn create_task_does_not_auto_persist() {
    let path = temp_path("no_auto");
    let _ = std::fs::remove_file(&path);

    let mut s = DaemonState::with_task_graph_path(path.clone()).unwrap();
    let _task = s.create_and_select_task("/ws", "T1");
    assert!(!path.exists(), "create_and_select_task must not auto-save");
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
    let _ = std::fs::remove_file(&path);
}

/// Contract 3: save returns Err on unwritable path.
#[test]
fn save_task_graph_returns_err_on_unwritable_path() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!(
            "dimweave_readonly_{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0),
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o444)).unwrap();
        let path = dir.join("test.json");

        let mut s = DaemonState::with_task_graph_path(path).unwrap();
        let _task = s.create_and_select_task("/ws", "T1");
        let result = s.save_task_graph();
        assert!(result.is_err(), "save to read-only dir must fail");

        // Cleanup
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Contract 4: select_task does NOT persist.
#[test]
fn select_task_does_not_persist() {
    let path = temp_path("select");
    let _ = std::fs::remove_file(&path);

    let mut s = DaemonState::with_task_graph_path(path.clone()).unwrap();
    let task = s.create_and_select_task("/ws", "T1");
    // Explicitly save once so the task exists on disk, then remove
    s.save_task_graph().unwrap();
    std::fs::remove_file(&path).unwrap();

    // select_task must NOT trigger persistence
    s.select_task(&task.task_id).unwrap();
    assert!(!path.exists(), "select_task must not auto-save");
    let _ = std::fs::remove_file(&path);
}
