use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::{
    path::Path,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};
use tauri::async_runtime;
use tauri::AppHandle;

use super::{clear_if_pid_matches, ActiveClaudeSession, ClaudeSessionManager};

pub(super) struct SpawnedSession {
    pub(super) session: ActiveClaudeSession,
    pub(super) child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub fn spawn_session(
    dir: &str,
    claude_bin: &Path,
    extra: &[String],
    cols: u16,
    rows: u16,
    app: AppHandle,
    emit_debug_logs: bool,
) -> Result<SpawnedSession, String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.max(24),
            cols: cols.max(80),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("failed to open Claude PTY: {e}"))?;

    let cmd = build_claude_command(dir, claude_bin, extra);

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
    Ok(SpawnedSession {
        session: ActiveClaudeSession {
            pid,
            writer,
            master,
        },
        child,
    })
}

fn build_claude_command(dir: &str, claude_bin: &Path, extra: &[String]) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(claude_bin.to_string_lossy().to_string());
    cmd.cwd(dir);
    cmd.env("TERM", "xterm-256color");
    cmd.env("COLORTERM", "truecolor");
    cmd.env("PATH", crate::claude_cli::enriched_path());
    cmd.arg("--dangerously-load-development-channels");
    cmd.arg("server:agentnexus");
    cmd.arg("--dangerously-skip-permissions");
    // Explicitly load project .mcp.json so Claude spawns the agentnexus MCP server.
    // Without this, Claude may not read the project-level config and the channel bridge
    // never gets spawned, resulting in "1 MCP server failed".
    let mcp_config_path = std::path::Path::new(dir).join(".mcp.json");
    cmd.arg("--mcp-config");
    cmd.arg(mcp_config_path.to_string_lossy().to_string());
    for arg in extra {
        cmd.arg(arg);
    }
    cmd
}

pub fn spawn_exit_watcher(
    manager: Arc<ClaudeSessionManager>,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
    app: AppHandle,
) {
    let _ = std::thread::Builder::new()
        .name("claude-pty-exit-watch".into())
        .spawn(move || {
            let pid = child.process_id().unwrap_or_default() as i32;
            let status = child.wait();
            async_runtime::spawn(async move {
                let cleared = clear_if_pid_matches(manager.as_ref(), pid).await;
                if !cleared {
                    return;
                }
                let (level, exit_code, summary) = describe_exit(status);

                eprintln!("[Claude] {summary}");
                crate::daemon::gui::emit_claude_stream(
                    &app,
                    crate::daemon::gui::ClaudeStreamPayload::Reset,
                );
                crate::daemon::gui::emit_claude_terminal_status(
                    &app,
                    false,
                    exit_code,
                    Some(summary.clone()),
                );
                crate::daemon::gui::emit_claude_terminal_data(
                    &app,
                    &format!("\r\n[AgentNexus] {summary}\r\n"),
                );
                crate::daemon::gui::emit_system_log(
                    &app,
                    level,
                    &format!("[Claude PTY] {summary}"),
                );
                crate::daemon::gui::emit_agent_status(&app, "claude", false, exit_code, None);
            });
        });
}

fn describe_exit(
    status: std::io::Result<portable_pty::ExitStatus>,
) -> (&'static str, Option<i32>, String) {
    match status {
        Ok(status) if status.signal().is_some() => {
            let signal = status.signal().unwrap_or("unknown");
            (
                "warn",
                None,
                format!("Claude terminal exited after signal {signal}"),
            )
        }
        Ok(status) => {
            let code = status.exit_code() as i32;
            let level = if code == 0 { "info" } else { "warn" };
            (
                level,
                Some(code),
                format!("Claude terminal exited with code {code}"),
            )
        }
        Err(err) => (
            "error",
            None,
            format!("Claude terminal exit watcher failed: {err}"),
        ),
    }
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

#[cfg(test)]
#[path = "process_tests.rs"]
mod process_tests;
