use super::{api, pairing, types::*};
use crate::daemon::{gui, routing, SharedState};
use reqwest::Client;
use tauri::AppHandle;

pub(super) async fn handle_update(
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
    let username = message
        .from
        .as_ref()
        .and_then(|u| u.username.clone())
        .unwrap_or_else(|| {
            message
                .from
                .as_ref()
                .map(|u| u.first_name.clone())
                .unwrap_or_else(|| "unknown".into())
        });

    if let Some(code) = pairing::match_pair_command(text) {
        handle_pair(state, client, token, cfg, app, chat_id, &username, code).await;
        return;
    }

    if cfg.paired_chat_id != Some(chat_id) {
        return;
    }

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
    state: &SharedState,
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

    {
        let mut s = state.write().await;
        s.telegram_paired_chat_id = Some(chat_id);
    }

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
    let mut rs = TelegramRuntimeState::from_config(cfg);
    rs.connected = true;
    gui::emit_telegram_state(app, &rs);
}

fn truncate(s: &str, max_chars: usize) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}
