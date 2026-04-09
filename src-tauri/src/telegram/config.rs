use super::types::TelegramConfig;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base.join("com.dimweave.app").join("telegram.json"))
}

pub fn load_config(path: &Path) -> anyhow::Result<TelegramConfig> {
    if !path.exists() {
        return Ok(TelegramConfig::default());
    }
    let data = std::fs::read_to_string(path)?;
    if data.trim().is_empty() {
        return Ok(TelegramConfig::default());
    }
    let cfg: TelegramConfig = serde_json::from_str(&data)?;
    Ok(cfg)
}

pub fn save_config(path: &Path, cfg: &TelegramConfig) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(cfg)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, &json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dimweave_telegram_cfg_{name}_{}_{}.json",
            std::process::id(),
            chrono::Utc::now().timestamp_millis(),
        ))
    }

    #[test]
    fn round_trip_preserves_pairing_and_cursor() {
        let cfg = TelegramConfig {
            enabled: true,
            bot_token: "123:abc".into(),
            notifications_enabled: true,
            paired_chat_id: Some(777001),
            paired_chat_label: Some("jason".into()),
            last_update_id: Some(42),
            pending_pair_code: None,
            pending_pair_expires_at: None,
            bot_username: None,
        };
        let path = temp_path("round_trip");
        save_config(&path, &cfg).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.paired_chat_id, Some(777001));
        assert_eq!(loaded.last_update_id, Some(42));
        assert_eq!(loaded.bot_token, "123:abc");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let cfg = load_config(Path::new("/tmp/nonexistent_dimweave_tg.json")).unwrap();
        assert!(!cfg.enabled);
        assert!(cfg.bot_token.is_empty());
    }
}
