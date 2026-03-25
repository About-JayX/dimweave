mod process;
mod prompt;

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
    _child: Box<dyn portable_pty::Child + Send>,
    writer: Arc<StdMutex<Box<dyn Write + Send>>>,
    master: Arc<StdMutex<Box<dyn MasterPty + Send>>>,
}

impl Default for ClaudeSessionManager {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

pub async fn launch(
    manager: &ClaudeSessionManager,
    dir: &str,
    claude_bin: &std::path::Path,
    extra: &[String],
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

    let session = spawn_session(dir, claude_bin, extra, app.clone(), emit_debug_logs)?;
    crate::daemon::gui::emit_claude_terminal_reset(&app);
    eprintln!("[Claude] PTY session started pid={}", session.pid);
    *guard = Some(session);
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
