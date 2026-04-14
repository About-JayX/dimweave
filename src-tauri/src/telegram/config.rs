use super::types::TelegramConfig;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base.join("com.dimweave.app").join("config.db"))
}

fn ensure_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS telegram_config (
            id      INTEGER PRIMARY KEY CHECK(id = 1),
            payload TEXT NOT NULL
        );",
    )
}

fn open_db(path: &Path) -> anyhow::Result<rusqlite::Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = rusqlite::Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
    ensure_schema(&conn)?;
    Ok(conn)
}

pub fn load_config(path: &Path) -> anyhow::Result<TelegramConfig> {
    if !path.exists() {
        return Ok(TelegramConfig::default());
    }
    let conn = open_db(path)?;
    match conn.query_row(
        "SELECT payload FROM telegram_config WHERE id = 1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(payload) => Ok(serde_json::from_str(&payload)?),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(TelegramConfig::default()),
        Err(e) => Err(e.into()),
    }
}

pub fn save_config(path: &Path, cfg: &TelegramConfig) -> anyhow::Result<()> {
    let conn = open_db(path)?;
    let payload = serde_json::to_string(cfg)?;
    conn.execute(
        "INSERT OR REPLACE INTO telegram_config (id, payload) VALUES (1, ?1)",
        rusqlite::params![payload],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dimweave_telegram_cfg_{name}_{}_{}.db",
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
            bot_user_id: None,
        };
        let path = temp_db_path("round_trip");
        save_config(&path, &cfg).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.paired_chat_id, Some(777001));
        assert_eq!(loaded.last_update_id, Some(42));
        assert_eq!(loaded.bot_token, "123:abc");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let cfg = load_config(Path::new("/tmp/nonexistent_dimweave_tg.db")).unwrap();
        assert!(!cfg.enabled);
        assert!(cfg.bot_token.is_empty());
    }
}
