use crate::daemon::DaemonCmd;
use crate::DaemonSender;
use tauri::State;

#[tauri::command]
pub async fn telegram_get_state(
    sender: State<'_, DaemonSender>,
) -> Result<crate::telegram::types::TelegramRuntimeState, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::GetTelegramState { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())
}

#[tauri::command]
pub async fn telegram_save_config(
    sender: State<'_, DaemonSender>,
    bot_token: String,
    enabled: bool,
    notifications_enabled: bool,
) -> Result<crate::telegram::types::TelegramRuntimeState, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::SaveTelegramConfig {
            bot_token,
            enabled,
            notifications_enabled,
            reply: reply_tx,
        })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}

#[tauri::command]
pub async fn telegram_generate_pair_code(
    sender: State<'_, DaemonSender>,
) -> Result<crate::telegram::types::TelegramRuntimeState, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::GenerateTelegramPairCode { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}

#[tauri::command]
pub async fn telegram_clear_pairing(
    sender: State<'_, DaemonSender>,
) -> Result<crate::telegram::types::TelegramRuntimeState, String> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    sender
        .0
        .send(DaemonCmd::ClearTelegramPairing { reply: reply_tx })
        .await
        .map_err(|_| "daemon offline".to_string())?;
    reply_rx.await.map_err(|_| "daemon dropped".to_string())?
}
