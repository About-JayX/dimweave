use super::types::FeishuProjectConfig;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base.join("com.dimweave.app").join("config.db"))
}

fn ensure_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS feishu_project_config (
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

pub fn load_config(path: &Path) -> anyhow::Result<FeishuProjectConfig> {
    if !path.exists() {
        return Ok(FeishuProjectConfig::default());
    }
    let conn = open_db(path)?;
    match conn.query_row(
        "SELECT payload FROM feishu_project_config WHERE id = 1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        Ok(payload) => Ok(serde_json::from_str(&payload)?),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(FeishuProjectConfig::default()),
        Err(e) => Err(e.into()),
    }
}

pub fn save_config(path: &Path, cfg: &FeishuProjectConfig) -> anyhow::Result<()> {
    let conn = open_db(path)?;
    let payload = serde_json::to_string(cfg)?;
    conn.execute(
        "INSERT OR REPLACE INTO feishu_project_config (id, payload) VALUES (1, ?1)",
        rusqlite::params![payload],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "dimweave_feishu_project_cfg_{name}_{}_{}.db",
            std::process::id(),
            chrono::Utc::now().timestamp_millis(),
        ))
    }

    #[test]
    fn config_round_trip_mcp_fields() {
        let cfg = FeishuProjectConfig {
            enabled: true,
            domain: "https://project.feishu.cn".into(),
            mcp_user_token: "tok_123".into(),
            workspace_hint: "myspace".into(),
            refresh_interval_minutes: 15,
            ..Default::default()
        };
        let path = temp_db_path("mcp_round_trip");
        save_config(&path, &cfg).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.domain, "https://project.feishu.cn");
        assert_eq!(loaded.mcp_user_token, "tok_123");
        assert_eq!(loaded.workspace_hint, "myspace");
        assert_eq!(loaded.refresh_interval_minutes, 15);
        assert!(loaded.enabled);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn config_round_trip_preserves_legacy_fields() {
        let cfg = FeishuProjectConfig {
            enabled: true,
            project_key: "proj".into(),
            plugin_token: "plugin_123".into(),
            user_key: "u_123".into(),
            ..Default::default()
        };
        let path = temp_db_path("legacy_compat");
        save_config(&path, &cfg).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.project_key, "proj");
        assert_eq!(loaded.plugin_token, "plugin_123");
        assert_eq!(loaded.user_key, "u_123");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let cfg = load_config(Path::new("/tmp/nonexistent_dimweave_fp.db")).unwrap();
        assert!(!cfg.enabled);
        assert_eq!(cfg.domain, "https://project.feishu.cn");
        assert!(cfg.mcp_user_token.is_empty());
    }

    #[test]
    fn save_creates_parent_directories() {
        let path = temp_db_path("nested");
        let nested = path.parent().unwrap().join("subdir").join("cfg.db");
        let cfg = FeishuProjectConfig {
            workspace_hint: "test".into(),
            ..Default::default()
        };
        save_config(&nested, &cfg).unwrap();
        let loaded = load_config(&nested).unwrap();
        assert_eq!(loaded.workspace_hint, "test");
        let _ = std::fs::remove_file(&nested);
        let _ = std::fs::remove_dir_all(nested.parent().unwrap());
    }
}
