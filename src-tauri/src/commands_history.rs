use crate::daemon::{
    provider::shared::ProviderHistoryEntry,
    types::HistoryEntry,
    DaemonCmd,
};
use crate::DaemonSender;
use tauri::State;

#[tauri::command]
pub async fn daemon_list_history(
    workspace: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<Vec<HistoryEntry>, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ListHistory {
            workspace,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped list_history reply".to_string())
}

#[tauri::command]
pub async fn daemon_list_provider_history(
    workspace: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<Vec<ProviderHistoryEntry>, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ListProviderHistory {
            workspace,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped list_provider_history reply".to_string())
}

#[tauri::command]
pub async fn daemon_resume_session(
    session_id: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ResumeSession {
            session_id,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped resume_session reply".to_string())?
}

#[tauri::command]
pub async fn daemon_attach_provider_history(
    provider: crate::daemon::task_graph::types::Provider,
    external_id: String,
    cwd: String,
    role: crate::daemon::task_graph::types::SessionRole,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::AttachProviderHistory {
            provider,
            external_id,
            cwd,
            role,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped attach_provider_history reply".to_string())?
}
