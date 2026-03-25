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
pub async fn stop_claude(session: State<'_, Arc<ClaudeSessionManager>>) -> Result<(), String> {
    crate::claude_session::stop(session.inner().as_ref()).await?;
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
