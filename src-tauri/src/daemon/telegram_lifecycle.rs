use super::gui;
use super::SharedState;
use tauri::AppHandle;

pub async fn get_runtime_state(
    _state: &SharedState,
    connected: bool,
) -> crate::telegram::types::TelegramRuntimeState {
    let cfg = match crate::telegram::config::default_config_path()
        .and_then(|p| crate::telegram::config::load_config(&p))
    {
        Ok(cfg) => cfg,
        Err(_) => crate::telegram::types::TelegramConfig::default(),
    };
    let mut rs = crate::telegram::types::TelegramRuntimeState::from_config(&cfg);
    rs.connected = connected;
    rs
}

pub async fn save_and_restart(
    state: &SharedState,
    app: &AppHandle,
    handle: &mut Option<crate::telegram::runtime::TelegramHandle>,
    bot_token: String,
    enabled: bool,
    notifications_enabled: bool,
) -> Result<crate::telegram::types::TelegramRuntimeState, String> {
    if let Some(h) = handle.take() {
        let mut h = h;
        h.stop().await;
    }
    {
        let mut s = state.write().await;
        s.telegram_outbound_tx = None;
        s.telegram_paired_chat_id = None;
    }

    let config_path = crate::telegram::config::default_config_path().map_err(|e| e.to_string())?;
    let mut cfg = crate::telegram::config::load_config(&config_path).unwrap_or_default();
    cfg.bot_token = bot_token;
    cfg.enabled = enabled;
    cfg.notifications_enabled = notifications_enabled;
    crate::telegram::config::save_config(&config_path, &cfg).map_err(|e| e.to_string())?;

    if enabled && !cfg.bot_token.is_empty() {
        match crate::telegram::runtime::start_runtime(state.clone(), app.clone(), cfg.clone()).await
        {
            Ok(h) => {
                let mut s = state.write().await;
                s.telegram_outbound_tx = Some(h.outbound_tx.clone());
                s.telegram_paired_chat_id = cfg.paired_chat_id;
                *handle = Some(h);
            }
            Err(e) => {
                gui::emit_system_log(app, "error", &format!("[Telegram] start failed: {e}"));
                let mut rs = crate::telegram::types::TelegramRuntimeState::from_config(&cfg);
                rs.last_error = Some(e.to_string());
                gui::emit_telegram_state(app, &rs);
                return Err(e.to_string());
            }
        }
    }

    let rs = get_runtime_state(state, handle.is_some()).await;
    gui::emit_telegram_state(app, &rs);
    Ok(rs)
}

pub async fn generate_pair(
    _state: &SharedState,
    app: &AppHandle,
    connected: bool,
) -> Result<crate::telegram::types::TelegramRuntimeState, String> {
    let config_path = crate::telegram::config::default_config_path().map_err(|e| e.to_string())?;
    let mut cfg = crate::telegram::config::load_config(&config_path).unwrap_or_default();
    let now = chrono::Utc::now().timestamp_millis() as u64;
    let (code, expires_at) = crate::telegram::pairing::generate_pair_code(now);
    cfg.pending_pair_code = Some(code);
    cfg.pending_pair_expires_at = Some(expires_at);
    crate::telegram::config::save_config(&config_path, &cfg).map_err(|e| e.to_string())?;
    let mut rs = crate::telegram::types::TelegramRuntimeState::from_config(&cfg);
    rs.connected = connected;
    gui::emit_telegram_state(app, &rs);
    Ok(rs)
}

pub async fn clear_pair(
    state: &SharedState,
    app: &AppHandle,
    connected: bool,
) -> Result<crate::telegram::types::TelegramRuntimeState, String> {
    let config_path = crate::telegram::config::default_config_path().map_err(|e| e.to_string())?;
    let mut cfg = crate::telegram::config::load_config(&config_path).unwrap_or_default();
    cfg.paired_chat_id = None;
    cfg.paired_chat_label = None;
    cfg.pending_pair_code = None;
    cfg.pending_pair_expires_at = None;
    crate::telegram::config::save_config(&config_path, &cfg).map_err(|e| e.to_string())?;
    {
        let mut s = state.write().await;
        s.telegram_paired_chat_id = None;
    }
    let mut rs = crate::telegram::types::TelegramRuntimeState::from_config(&cfg);
    rs.connected = connected;
    gui::emit_telegram_state(app, &rs);
    Ok(rs)
}
