use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{
    path::Path,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};
use tauri::AppHandle;

use super::ActiveClaudeSession;

pub fn spawn_session(
    dir: &str,
    claude_bin: &Path,
    extra: &[String],
    app: AppHandle,
    emit_debug_logs: bool,
) -> Result<ActiveClaudeSession, String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("failed to open Claude PTY: {e}"))?;

    let mut cmd = CommandBuilder::new(claude_bin.to_string_lossy().to_string());
    cmd.cwd(dir);
    cmd.arg("--dangerously-load-development-channels");
    cmd.arg("server:agentbridge");
    for arg in extra {
        cmd.arg(arg);
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("failed to spawn Claude in PTY: {e}"))?;
    let pid = child
        .process_id()
        .ok_or_else(|| "failed to read Claude pid".to_string())? as i32;
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("failed to open Claude PTY reader: {e}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("failed to open Claude PTY writer: {e}"))?;
    let writer = Arc::new(StdMutex::new(writer));
    let master = Arc::new(StdMutex::new(pair.master));

    super::prompt::spawn_auto_confirm_thread(reader, writer.clone(), app, emit_debug_logs);
    Ok(ActiveClaudeSession {
        pid,
        _child: child,
        writer,
        master,
    })
}

pub async fn terminate_pid(pid: i32) -> Result<(), String> {
    if !signal_pid(pid, libc::SIGTERM)? {
        return Err("Tracked Claude session was already gone".into());
    }
    if wait_for_exit(pid, Duration::from_secs(3)).await? {
        return Ok(());
    }
    signal_pid(pid, libc::SIGKILL)?;
    let _ = wait_for_exit(pid, Duration::from_secs(2)).await?;
    Ok(())
}

pub fn signal_pid(pid: i32, signal: i32) -> Result<bool, String> {
    let rc = unsafe { libc::kill(pid, signal) };
    if rc == 0 {
        return Ok(true);
    }
    let err = std::io::Error::last_os_error();
    match err.raw_os_error() {
        Some(code) if code == libc::ESRCH => Ok(false),
        _ => Err(format!("failed to signal Claude pid {pid}: {err}")),
    }
}

pub fn process_alive(pid: i32) -> Result<bool, String> {
    let rc = unsafe { libc::kill(pid, 0) };
    if rc == 0 {
        return Ok(true);
    }
    let err = std::io::Error::last_os_error();
    match err.raw_os_error() {
        Some(code) if code == libc::ESRCH => Ok(false),
        Some(code) if code == libc::EPERM => Ok(true),
        _ => Err(format!("failed to inspect Claude pid {pid}: {err}")),
    }
}

pub async fn wait_for_exit(pid: i32, timeout: Duration) -> Result<bool, String> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if !process_alive(pid)? {
            return Ok(true);
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok(false);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
