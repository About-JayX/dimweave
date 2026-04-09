use super::{api, config, types::*};
use crate::daemon::gui;
use crate::daemon::SharedState;
use reqwest::Client;
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot};

pub struct TelegramHandle {
    pub outbound_tx: mpsc::Sender<TelegramOutbound>,
    pub config_tx: mpsc::Sender<TelegramConfig>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl TelegramHandle {
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

pub async fn start_runtime(
    state: SharedState,
    app: AppHandle,
    mut cfg: TelegramConfig,
) -> anyhow::Result<TelegramHandle> {
    let client = Client::new();
    let bot = api::get_me(&client, &cfg.bot_token).await?;
    let bot_username = bot
        .username
        .clone()
        .unwrap_or_else(|| bot.first_name.clone());
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Telegram] connected as @{bot_username}"),
    );

    // Persist bot_username so it survives restarts
    cfg.bot_username = Some(bot_username);
    let config_path = config::default_config_path()?;
    let _ = config::save_config(&config_path, &cfg);

    let (outbound_tx, mut outbound_rx) = mpsc::channel::<TelegramOutbound>(64);
    let (config_tx, mut config_rx) = mpsc::channel::<TelegramConfig>(8);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    let token = cfg.bot_token.clone();

    tokio::spawn(async move {
        let mut offset = cfg.last_update_id.map(|id| id + 1);
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    gui::emit_system_log(&app, "info", "[Telegram] runtime stopped");
                    return;
                }
                new_cfg = config_rx.recv() => {
                    if let Some(new_cfg) = new_cfg {
                        apply_config_update(&mut cfg, new_cfg);
                    }
                }
                outbound = outbound_rx.recv() => {
                    if let Some(msg) = outbound {
                        if let Err(e) = api::send_message_html(
                            &client, &token, msg.chat_id, &msg.text,
                            msg.parse_mode.as_deref(),
                        ).await {
                            gui::emit_system_log(
                                &app,
                                "warn",
                                &format!("[Telegram] send failed: {e}"),
                            );
                        }
                    }
                }
                result = api::get_updates(&client, &token, offset, 30) => {
                    while let Ok(new_cfg) = config_rx.try_recv() {
                        apply_config_update(&mut cfg, new_cfg);
                    }
                    match result {
                        Ok(updates) => {
                            for update in updates {
                                offset = Some(update.update_id + 1);
                                cfg.last_update_id = Some(update.update_id);
                                super::runtime_handlers::handle_update(
                                    &state, &app, &client, &token, &mut cfg, &update,
                                ).await;
                            }
                            let _ = config::save_config(&config_path, &cfg);
                        }
                        Err(e) => {
                            gui::emit_system_log(
                                &app,
                                "warn",
                                &format!("[Telegram] poll error: {e}"),
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        }
                    }
                }
            }
        }
    });

    Ok(TelegramHandle {
        outbound_tx,
        config_tx,
        shutdown_tx: Some(shutdown_tx),
    })
}

/// Merge lifecycle-pushed config into the runtime's in-memory copy.
fn apply_config_update(cfg: &mut TelegramConfig, new_cfg: TelegramConfig) {
    cfg.pending_pair_code = new_cfg.pending_pair_code;
    cfg.pending_pair_expires_at = new_cfg.pending_pair_expires_at;
    cfg.paired_chat_id = new_cfg.paired_chat_id;
    cfg.paired_chat_label = new_cfg.paired_chat_label;
    cfg.notifications_enabled = new_cfg.notifications_enabled;
}
