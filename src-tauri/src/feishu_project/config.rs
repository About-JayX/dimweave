use super::types::FeishuProjectConfig;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    Ok(base.join("com.dimweave.app").join("feishu_project.json"))
}

pub fn load_config(path: &Path) -> anyhow::Result<FeishuProjectConfig> {
    if !path.exists() {
        return Ok(FeishuProjectConfig::default());
    }
    let data = std::fs::read_to_string(path)?;
    let cfg: FeishuProjectConfig = serde_json::from_str(&data)?;
    Ok(cfg)
}

pub fn save_config(path: &Path, cfg: &FeishuProjectConfig) -> anyhow::Result<()> {
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
            "dimweave_feishu_project_cfg_{name}_{}_{}.json",
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
        let path = temp_path("mcp_round_trip");
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
        let path = temp_path("legacy_compat");
        save_config(&path, &cfg).unwrap();
        let loaded = load_config(&path).unwrap();
        assert_eq!(loaded.project_key, "proj");
        assert_eq!(loaded.plugin_token, "plugin_123");
        assert_eq!(loaded.user_key, "u_123");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let cfg = load_config(Path::new("/tmp/nonexistent_dimweave_fp.json")).unwrap();
        assert!(!cfg.enabled);
        assert_eq!(cfg.domain, "https://project.feishu.cn");
        assert!(cfg.mcp_user_token.is_empty());
    }

    #[test]
    fn save_creates_parent_directories() {
        let path = temp_path("nested");
        let nested = path.parent().unwrap().join("subdir").join("cfg.json");
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
