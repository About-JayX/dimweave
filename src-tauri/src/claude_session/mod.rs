mod process;
mod prompt;
mod text_utils;

use portable_pty::{MasterPty, PtySize};
use process::*;
use std::{
    io::Write,
    sync::{Arc, Mutex as StdMutex},
};
use tauri::AppHandle;
use tokio::sync::Mutex;

pub struct ClaudeSessionManager(Mutex<Option<ActiveClaudeSession>>);

struct ActiveClaudeSession {
    pid: i32,
    writer: Arc<StdMutex<Box<dyn Write + Send>>>,
    master: Arc<StdMutex<Box<dyn MasterPty + Send>>>,
}

impl Default for ClaudeSessionManager {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn launch(
    manager: Arc<ClaudeSessionManager>,
    dir: &str,
    claude_bin: &std::path::Path,
    extra: &[String],
    cols: Option<u16>,
    rows: Option<u16>,
    app: AppHandle,
    emit_debug_logs: bool,
) -> Result<(), String> {
    let mut guard = manager.0.lock().await;
    if let Some(session) = guard.as_ref() {
        if process_alive(session.pid)? {
            return Err("Claude session already running".into());
        }
        *guard = None;
    }

    let spawned = spawn_session(
        dir,
        claude_bin,
        extra,
        cols.unwrap_or(220),
        rows.unwrap_or(50),
        app.clone(),
        emit_debug_logs,
    )?;
    crate::daemon::gui::emit_claude_terminal_reset(&app);
    crate::daemon::gui::emit_claude_terminal_status(&app, true, None, None);
    eprintln!("[Claude] PTY session started pid={}", spawned.session.pid);
    *guard = Some(spawned.session);
    drop(guard);
    spawn_exit_watcher(manager, spawned.child, app);
    Ok(())
}

pub async fn stop(manager: &ClaudeSessionManager) -> Result<(), String> {
    let session = {
        let mut guard = manager.0.lock().await;
        guard
            .take()
            .ok_or_else(|| "No active Claude session found".to_string())?
    };

    if !process_alive(session.pid)? {
        return Err("Tracked Claude session was already gone".into());
    }
    terminate_pid(session.pid).await?;
    eprintln!("[Claude] PTY session stopped pid={}", session.pid);
    Ok(())
}

pub async fn stop_if_running(manager: &ClaudeSessionManager) {
    if let Err(err) = stop(manager).await {
        if err != "No active Claude session found" {
            eprintln!("[Claude] shutdown stop failed: {err}");
        }
    }
}

pub(super) async fn clear_if_pid_matches(manager: &ClaudeSessionManager, pid: i32) -> bool {
    let mut guard = manager.0.lock().await;
    if guard.as_ref().map(|session| session.pid) == Some(pid) {
        *guard = None;
        return true;
    }
    false
}

pub async fn write_input(manager: &ClaudeSessionManager, data: &str) -> Result<(), String> {
    let writer = {
        let guard = manager.0.lock().await;
        guard
            .as_ref()
            .map(|session| session.writer.clone())
            .ok_or_else(|| "No active Claude session found".to_string())?
    };
    let mut writer = writer
        .lock()
        .map_err(|_| "Claude PTY writer lock poisoned".to_string())?;
    writer
        .write_all(data.as_bytes())
        .and_then(|_| writer.flush())
        .map_err(|e| format!("failed to write to Claude PTY: {e}"))
}

pub async fn resize(manager: &ClaudeSessionManager, cols: u16, rows: u16) -> Result<(), String> {
    if cols == 0 || rows == 0 {
        return Ok(());
    }
    let master = {
        let guard = manager.0.lock().await;
        guard
            .as_ref()
            .map(|session| session.master.clone())
            .ok_or_else(|| "No active Claude session found".to_string())?
    };
    let result = master
        .lock()
        .map_err(|_| "Claude PTY master lock poisoned".to_string())?
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("failed to resize Claude PTY: {e}"));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use portable_pty::PtySize;
    use std::io::Read;

    struct DummyMasterPty;

    impl MasterPty for DummyMasterPty {
        fn resize(&self, _size: PtySize) -> anyhow::Result<()> {
            Ok(())
        }

        fn get_size(&self) -> anyhow::Result<PtySize> {
            Ok(PtySize::default())
        }

        fn try_clone_reader(&self) -> anyhow::Result<Box<dyn Read + Send>> {
            Ok(Box::new(std::io::empty()))
        }

        fn take_writer(&self) -> anyhow::Result<Box<dyn Write + Send>> {
            Ok(Box::new(Vec::<u8>::new()))
        }

        #[cfg(unix)]
        fn process_group_leader(&self) -> Option<libc::pid_t> {
            None
        }

        #[cfg(unix)]
        fn as_raw_fd(&self) -> Option<std::os::fd::RawFd> {
            None
        }

        #[cfg(unix)]
        fn tty_name(&self) -> Option<std::path::PathBuf> {
            None
        }
    }

    fn dummy_session(pid: i32) -> ActiveClaudeSession {
        ActiveClaudeSession {
            pid,
            writer: Arc::new(StdMutex::new(Box::new(Vec::<u8>::new()))),
            master: Arc::new(StdMutex::new(Box::new(DummyMasterPty))),
        }
    }

    #[tokio::test]
    async fn clear_if_pid_matches_removes_matching_session() {
        let manager = ClaudeSessionManager(Mutex::new(Some(dummy_session(42))));
        assert!(clear_if_pid_matches(&manager, 42).await);
        assert!(manager.0.lock().await.is_none());
    }

    #[tokio::test]
    async fn clear_if_pid_matches_keeps_different_session() {
        let manager = ClaudeSessionManager(Mutex::new(Some(dummy_session(7))));
        assert!(!clear_if_pid_matches(&manager, 42).await);
        assert_eq!(manager.0.lock().await.as_ref().map(|session| session.pid), Some(7));
    }
}
