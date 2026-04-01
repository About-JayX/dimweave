use crate::claude_session::ClaudeSessionManager;
use crate::daemon::{
    types::{DaemonStatusSnapshot, PermissionBehavior},
    DaemonCmd,
};
use crate::DaemonSender;
use std::sync::Arc;
use tauri::State;

pub(crate) mod oauth;

fn validate_codex_launch_args(role_id: &str, cwd: &str) -> Result<(), String> {
    if !crate::daemon::is_valid_agent_role(role_id) {
        return Err(format!("invalid role: {role_id}"));
    }
    if cwd.trim().is_empty() {
        return Err("cwd is required".to_string());
    }
    Ok(())
}

/// User typed a message — daemon handles GUI echo + fan-out internally.
#[tauri::command]
pub async fn daemon_send_user_input(
    content: String,
    target: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    if target != "auto" && !crate::daemon::is_valid_agent_role(&target) {
        return Err(format!("invalid target role: {target}"));
    }
    sender
        .0
        .send(DaemonCmd::SendUserInput { content, target })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn daemon_launch_codex(
    role_id: String,
    cwd: String,
    model: Option<String>,
    reasoning_effort: Option<String>,
    resume_thread_id: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    validate_codex_launch_args(&role_id, &cwd)?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::LaunchCodex {
            role_id,
            cwd,
            model,
            reasoning_effort,
            resume_thread_id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped codex launch result".to_string())?
}

#[tauri::command]
pub async fn daemon_stop_codex(sender: State<'_, DaemonSender>) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::StopCodex)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn daemon_set_claude_role(
    role: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    if !crate::daemon::is_valid_agent_role(&role) {
        return Err(format!("invalid role: {role}"));
    }
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::SetClaudeRole {
            role,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped role reply".to_string())?
}

#[tauri::command]
pub async fn daemon_set_codex_role(
    role: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    if !crate::daemon::is_valid_agent_role(&role) {
        return Err(format!("invalid role: {role}"));
    }
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::SetCodexRole {
            role,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped role reply".to_string())?
}

#[tauri::command]
pub async fn daemon_respond_permission(
    request_id: String,
    behavior: PermissionBehavior,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::RespondPermission {
            request_id,
            behavior,
        })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn daemon_get_status_snapshot(
    sender: State<'_, DaemonSender>,
) -> Result<DaemonStatusSnapshot, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ReadStatusSnapshot { reply: reply_tx })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped status snapshot reply".to_string())
}

/// Launch Claude via --sdk-url direct WS connection (new path).
#[tauri::command]
pub async fn daemon_launch_claude_sdk(
    role_id: String,
    cwd: String,
    model: Option<String>,
    resume_session_id: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    if !crate::daemon::is_valid_agent_role(&role_id) {
        return Err(format!("invalid role: {role_id}"));
    }
    if cwd.trim().is_empty() {
        return Err("cwd is required".to_string());
    }
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::LaunchClaudeSdk {
            role_id,
            cwd,
            model,
            resume_session_id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped claude sdk launch result".to_string())?
}

/// Stop Claude SDK session.
#[tauri::command]
pub async fn daemon_stop_claude_sdk(sender: State<'_, DaemonSender>) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::StopClaudeSdk)
        .await
        .map_err(|e| e.to_string())
}

/// Stop the tracked Claude CLI session and/or force-disconnect the bridge agent.
/// Handles managed PTY, SDK, and externally-connected Claude instances.
#[tauri::command]
pub async fn stop_claude(
    session: State<'_, Arc<ClaudeSessionManager>>,
    sender: State<'_, DaemonSender>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // 1. Try to stop SDK session
    let _ = sender.0.send(DaemonCmd::StopClaudeSdk).await;
    // 2. Try to stop managed PTY session (legacy path)
    let had_pty = crate::claude_session::stop(session.inner().as_ref())
        .await
        .is_ok();
    if had_pty {
        crate::daemon::gui::emit_claude_terminal_status(
            &app,
            false,
            None,
            Some("Claude terminal stopped by user".into()),
        );
        crate::daemon::gui::emit_claude_terminal_data(
            &app,
            "\r\n[AgentNexus] Claude terminal stopped by user\r\n",
        );
    }
    // 3. Force-disconnect bridge agent (handles externally-connected Claude)
    let _ = sender
        .0
        .send(DaemonCmd::ForceDisconnectAgent {
            agent_id: "claude".into(),
        })
        .await;
    crate::daemon::gui::emit_system_log(&app, "info", "[Claude] disconnected by user");
    Ok(())
}

#[tauri::command]
pub async fn claude_terminal_input(
    data: String,
    session: State<'_, Arc<ClaudeSessionManager>>,
) -> Result<(), String> {
    crate::claude_session::write_input(session.inner().as_ref(), &data).await
}

#[tauri::command]
pub async fn claude_terminal_resize(
    cols: u16,
    rows: u16,
    session: State<'_, Arc<ClaudeSessionManager>>,
) -> Result<(), String> {
    crate::claude_session::resize(session.inner().as_ref(), cols, rows).await
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
