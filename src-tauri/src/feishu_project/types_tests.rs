use super::*;

#[test]
fn mask_token_hides_secret() {
    assert_eq!(mask_token("token_secret_123"), Some("token***".into()));
}

#[test]
fn mask_token_empty() {
    assert_eq!(mask_token(""), None);
}

#[test]
fn runtime_state_from_mcp_config() {
    let cfg = FeishuProjectConfig {
        enabled: true,
        domain: "https://project.feishu.cn".into(),
        mcp_user_token: "tok_abc123".into(),
        workspace_hint: "myspace".into(),
        refresh_interval_minutes: 15,
        ..Default::default()
    };
    let rs = FeishuProjectRuntimeState::from_config(&cfg);
    assert!(rs.enabled);
    assert_eq!(rs.domain.as_deref(), Some("https://project.feishu.cn"));
    assert_eq!(rs.workspace_hint.as_deref(), Some("myspace"));
    assert_eq!(rs.token_label.as_deref(), Some("tok_a***"));
    assert_eq!(rs.mcp_status, McpConnectionStatus::Disconnected);
    assert_eq!(rs.refresh_interval_minutes, 15);
}

#[test]
fn default_config_has_mcp_defaults() {
    let cfg = FeishuProjectConfig::default();
    assert!(!cfg.enabled);
    assert_eq!(cfg.domain, "https://project.feishu.cn");
    assert_eq!(cfg.refresh_interval_minutes, 10);
    assert!(cfg.mcp_user_token.is_empty());
}

#[test]
fn mcp_connection_status_serializes() {
    let json = serde_json::to_string(&McpConnectionStatus::Connected).unwrap();
    assert_eq!(json, "\"connected\"");
    let parsed: McpConnectionStatus = serde_json::from_str("\"unauthorized\"").unwrap();
    assert_eq!(parsed, McpConnectionStatus::Unauthorized);
}
