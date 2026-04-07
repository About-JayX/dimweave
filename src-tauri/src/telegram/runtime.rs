use super::{api, config, pairing, types::*};
use crate::daemon::{gui, routing, SharedState};
use reqwest::Client;
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot};

pub struct TelegramHandle {
    pub outbound_tx: mpsc::Sender<TelegramOutbound>,
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

    let (outbound_tx, mut outbound_rx) = mpsc::channel::<TelegramOutbound>(64);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    let token = cfg.bot_token.clone();
    let config_path = config::default_config_path()?;

    tokio::spawn(async move {
        let mut offset = cfg.last_update_id.map(|id| id + 1);
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    gui::emit_system_log(&app, "info", "[Telegram] runtime stopped");
                    return;
                }
                outbound = outbound_rx.recv() => {
                    if let Some(msg) = outbound {
                        if let Err(e) = api::send_message(
                            &client, &token, msg.chat_id, &msg.text,
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
                    match result {
                        Ok(updates) => {
                            for update in updates {
                                offset = Some(update.update_id + 1);
                                cfg.last_update_id = Some(update.update_id);
                                handle_update(
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
        shutdown_tx: Some(shutdown_tx),
    })
}

async fn handle_update(
    state: &SharedState,
    app: &AppHandle,
    client: &Client,
    token: &str,
    cfg: &mut TelegramConfig,
    update: &api::TelegramUpdate,
) {
    let Some(ref message) = update.message else {
        return;
    };
    let Some(ref text) = message.text else {
        return;
    };
    let chat_id = message.chat.id;
    let username = message.from.as_ref().and_then(|u| u.username.clone()).unwrap_or_else(|| {
        message.from.as_ref().map(|u| u.first_name.clone()).unwrap_or_else(|| "unknown".into())
    });

    // Handle /pair command
    if let Some(code) = pairing::match_pair_command(text) {
        handle_pair(client, token, cfg, app, chat_id, &username, code).await;
        return;
    }

    // Only accept messages from paired chat
    if cfg.paired_chat_id != Some(chat_id) {
        return;
    }

    // Route inbound text as user -> lead
    gui::emit_system_log(
        app,
        "info",
        &format!("[Telegram] inbound from @{username}: {}", truncate(text, 80)),
    );
    let msg = crate::daemon::types::BridgeMessage {
        id: uuid::Uuid::new_v4().to_string(),
        from: "user".into(),
        display_source: Some("telegram".into()),
        to: "lead".into(),
        content: text.clone(),
        timestamp: chrono::Utc::now().timestamp_millis() as u64,
        reply_to: None,
        priority: None,
        status: None,
        task_id: None,
        session_id: None,
        sender_agent_id: None,
        attachments: None,
    };
    routing::route_message(state, app, msg).await;
}

async fn handle_pair(
    client: &Client,
    token: &str,
    cfg: &mut TelegramConfig,
    app: &AppHandle,
    chat_id: i64,
    username: &str,
    code: &str,
) {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    if !pairing::is_code_valid(cfg.pending_pair_code.as_deref(), cfg.pending_pair_expires_at, now) {
        let _ =
            api::send_message(client, token, chat_id, "No active pairing code or code expired.")
                .await;
        return;
    }
    if cfg.pending_pair_code.as_deref() != Some(code) {
        let _ = api::send_message(client, token, chat_id, "Invalid pairing code.").await;
        return;
    }
    cfg.paired_chat_id = Some(chat_id);
    cfg.paired_chat_label = Some(format!("@{username}"));
    cfg.pending_pair_code = None;
    cfg.pending_pair_expires_at = None;
    let _ = api::send_message(
        client,
        token,
        chat_id,
        "Paired successfully! You will receive lead reports here.",
    )
    .await;
    gui::emit_system_log(
        app,
        "info",
        &format!("[Telegram] paired with @{username} (chat {chat_id})"),
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max])
    }
}
