use crate::claude_session::ClaudeSessionManager;
use crate::codex::oauth::{OAuthHandle, OAuthLaunchInfo};
use crate::daemon::{
    types::{BridgeMessage, DaemonStatusSnapshot, PermissionBehavior},
    DaemonCmd,
};
use crate::DaemonSender;
use std::sync::Arc;
use tauri::{Manager, State};

#[tauri::command]
pub async fn daemon_send_message(
    msg: BridgeMessage,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::SendMessage(msg))
        .await
        .map_err(|e| e.to_string())
}

/// User typed a message — daemon handles GUI echo + fan-out internally.
#[tauri::command]
pub async fn daemon_send_user_input(
    content: String,
    target: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
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
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::LaunchCodex {
            role_id,
            cwd,
            model,
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
    sender
        .0
        .send(DaemonCmd::SetClaudeRole(role))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn daemon_set_codex_role(
    role: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::SetCodexRole(role))
        .await
        .map_err(|e| e.to_string())
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

/// Stop the tracked Claude CLI session.
/// When Claude exits, its channel subprocess should drop too and daemon status
/// will fall back to disconnected once the control websocket closes.
#[tauri::command]
pub async fn stop_claude(
    session: State<'_, Arc<ClaudeSessionManager>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    crate::claude_session::stop(session.inner().as_ref()).await?;
    crate::daemon::gui::emit_claude_terminal_status(
        &app,
        false,
        None,
        Some("Claude terminal stopped by user".into()),
    );
    crate::daemon::gui::emit_claude_terminal_data(
        &app,
        "\r\n[AgentBridge] Claude terminal stopped by user\r\n",
    );
    crate::daemon::gui::emit_system_log(&app, "info", "[Claude PTY] stopped by user");
    // Defensive: emit claude offline immediately (WS disconnect will also emit later)
    crate::daemon::gui::emit_agent_status(&app, "claude", false, None);
    eprintln!("[Claude] stop: terminated managed Claude PTY session");
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

#[tauri::command]
pub async fn codex_login(app: tauri::AppHandle) -> Result<OAuthLaunchInfo, String> {
    let handle = app.state::<Arc<OAuthHandle>>();
    crate::codex::oauth::start_login(handle.inner().clone()).await
}

#[tauri::command]
pub fn codex_cancel_login(app: tauri::AppHandle) -> bool {
    app.state::<Arc<OAuthHandle>>().cancel()
}

#[tauri::command]
pub async fn codex_logout() -> Result<(), String> {
    crate::codex::oauth::do_logout().await
}
