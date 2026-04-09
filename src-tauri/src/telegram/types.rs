use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: String,
    pub notifications_enabled: bool,
    pub paired_chat_id: Option<i64>,
    pub paired_chat_label: Option<String>,
    pub last_update_id: Option<i64>,
    pub pending_pair_code: Option<String>,
    pub pending_pair_expires_at: Option<u64>,
    #[serde(default)]
    pub bot_username: Option<String>,
}

/// Masked runtime state safe for frontend display.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TelegramRuntimeState {
    pub enabled: bool,
    pub connected: bool,
    pub notifications_enabled: bool,
    pub token_label: Option<String>,
    pub bot_username: Option<String>,
    pub paired_chat_label: Option<String>,
    pub pending_pair_code: Option<String>,
    pub pending_pair_expires_at: Option<u64>,
    pub last_error: Option<String>,
    pub last_delivery_at: Option<u64>,
    pub last_inbound_at: Option<u64>,
}

impl TelegramRuntimeState {
    pub fn from_config(cfg: &TelegramConfig) -> Self {
        Self {
            enabled: cfg.enabled,
            connected: false,
            notifications_enabled: cfg.notifications_enabled,
            token_label: mask_token(&cfg.bot_token),
            bot_username: cfg.bot_username.clone(),
            paired_chat_label: cfg.paired_chat_label.clone(),
            pending_pair_code: cfg.pending_pair_code.clone(),
            pending_pair_expires_at: cfg.pending_pair_expires_at,
            last_error: None,
            last_delivery_at: None,
            last_inbound_at: None,
        }
    }
}

pub fn mask_token(token: &str) -> Option<String> {
    if token.is_empty() {
        return None;
    }
    match token.split_once(':') {
        Some((prefix, _)) => Some(format!("{prefix}:***")),
        None => Some("***".into()),
    }
}

/// Outbound message queued for Telegram delivery.
#[derive(Debug, Clone)]
pub struct TelegramOutbound {
    pub chat_id: i64,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_token_hides_secret_part() {
        assert_eq!(mask_token("123:secret"), Some("123:***".into()));
    }

    #[test]
    fn mask_token_handles_empty() {
        assert_eq!(mask_token(""), None);
    }

    #[test]
    fn mask_token_handles_no_colon() {
        assert_eq!(mask_token("nocolon"), Some("***".into()));
    }

    #[test]
    fn runtime_state_from_config_masks_token() {
        let cfg = TelegramConfig {
            bot_token: "123:secret".into(),
            paired_chat_label: Some("@jason".into()),
            ..Default::default()
        };
        let state = TelegramRuntimeState::from_config(&cfg);
        assert_eq!(state.token_label.as_deref(), Some("123:***"));
        assert_eq!(state.paired_chat_label.as_deref(), Some("@jason"));
    }
}
