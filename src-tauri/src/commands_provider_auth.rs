use tauri::State;

use crate::daemon::task_graph::types::ProviderAuthConfig;
use crate::daemon::DaemonCmd;
use crate::DaemonSender;

#[tauri::command]
pub async fn daemon_get_provider_auth(
    provider: String,
    sender: State<'_, DaemonSender>,
) -> Result<Option<ProviderAuthConfig>, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::GetProviderAuth {
            provider,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped get_provider_auth reply".to_string())
}

#[tauri::command]
pub async fn daemon_save_provider_auth(
    config: ProviderAuthConfig,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    if config.provider != "claude" && config.provider != "codex" {
        return Err(format!("unsupported provider: {}", config.provider));
    }
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::SaveProviderAuth {
            config,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped save_provider_auth reply".to_string())?
}

#[tauri::command]
pub async fn daemon_clear_provider_auth(
    provider: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ClearProviderAuth {
            provider,
            reply: reply_tx,
        })
        .await
        .map_err(|e| e.to_string())?;
    reply_rx
        .await
        .map_err(|_| "daemon dropped clear_provider_auth reply".to_string())?
}
