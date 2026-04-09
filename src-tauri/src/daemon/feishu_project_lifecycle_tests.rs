use super::*;
use crate::feishu_project::types::{
    FeishuProjectConfig, FeishuProjectRuntimeState, McpConnectionStatus,
};
use std::sync::Arc;
use tokio::sync::RwLock;

fn shared(state: crate::daemon::state::DaemonState) -> SharedState {
    Arc::new(RwLock::new(state))
}

fn stale_runtime() -> FeishuProjectRuntimeState {
    FeishuProjectRuntimeState {
        enabled: true,
        domain: Some("https://project.feishu.cn".into()),
        workspace_hint: Some("old_ws".into()),
        refresh_interval_minutes: 10,
        sync_mode: crate::feishu_project::types::FeishuSyncMode::Issues,
        project_name: Some("Old Project Name".into()),
        team_members: vec!["Alice".into(), "Bob".into()],
        mcp_status: McpConnectionStatus::Connected,
        discovered_tool_count: 35,
        last_sync_at: Some(1000),
        last_error: None,
        token_label: Some("m-old***".into()),
    }
}

#[tokio::test]
async fn get_runtime_state_returns_cache_when_present() {
    let mut ds = crate::daemon::state::DaemonState::new();
    ds.feishu_project_runtime = Some(stale_runtime());
    let state = shared(ds);
    let rs = get_runtime_state(&state).await;
    assert_eq!(rs.project_name.as_deref(), Some("Old Project Name"));
    assert_eq!(rs.mcp_status, McpConnectionStatus::Connected);
    assert_eq!(rs.team_members.len(), 2);
}

#[tokio::test]
async fn get_runtime_state_rebuilds_from_config_when_cache_cleared() {
    let mut ds = crate::daemon::state::DaemonState::new();
    // Populate stale cache
    ds.feishu_project_runtime = Some(stale_runtime());
    let state = shared(ds);
    // Clear cache (simulates what save_and_restart does after stop)
    state.write().await.feishu_project_runtime = None;
    let rs = get_runtime_state(&state).await;
    // Should reflect fresh from_config defaults, not stale cache
    assert_eq!(rs.project_name, None);
    assert!(rs.team_members.is_empty());
    assert_eq!(rs.mcp_status, McpConnectionStatus::Disconnected);
    assert_eq!(rs.discovered_tool_count, 0);
}

#[tokio::test]
async fn disable_clears_stale_connected_state() {
    let mut ds = crate::daemon::state::DaemonState::new();
    ds.feishu_project_runtime = Some(stale_runtime());
    let state = shared(ds);
    // Simulate the disable path: clear cache, then read state
    state.write().await.feishu_project_runtime = None;
    let rs = get_runtime_state(&state).await;
    // Must not carry over old connected status or project name
    assert_ne!(rs.mcp_status, McpConnectionStatus::Connected);
    assert_eq!(rs.project_name, None);
    assert!(rs.team_members.is_empty());
}

#[tokio::test]
async fn start_failure_returns_fresh_error_state() {
    let mut ds = crate::daemon::state::DaemonState::new();
    ds.feishu_project_runtime = Some(stale_runtime());
    let state = shared(ds);
    // Simulate save_and_restart stop path: clear cache
    state.write().await.feishu_project_runtime = None;
    // get_runtime_state now returns fresh from_config (no stale connected/project)
    let rs = get_runtime_state(&state).await;
    assert_eq!(rs.mcp_status, McpConnectionStatus::Disconnected);
    assert_eq!(rs.project_name, None);
    assert!(rs.team_members.is_empty());
    // The caller (save_and_restart) can layer error info on top of this fresh state
}

fn existing_config() -> FeishuProjectConfig {
    FeishuProjectConfig {
        enabled: true,
        mcp_user_token: "m-secret-token".into(),
        workspace_hint: "manciyuan".into(),
        ..Default::default()
    }
}

#[test]
fn merge_config_enabled_preserves_old_token_when_empty() {
    let incoming = FeishuProjectConfig {
        enabled: true,
        mcp_user_token: "".into(),
        workspace_hint: "".into(),
        ..Default::default()
    };
    let existing = existing_config();
    let merged = merge_config(incoming, Some(&existing));
    assert_eq!(merged.mcp_user_token, "m-secret-token");
    assert_eq!(merged.workspace_hint, "manciyuan");
}

#[test]
fn merge_config_disabled_does_not_restore_old_values() {
    let incoming = FeishuProjectConfig {
        enabled: false,
        mcp_user_token: "".into(),
        workspace_hint: "".into(),
        ..Default::default()
    };
    let existing = existing_config();
    let merged = merge_config(incoming, Some(&existing));
    assert!(merged.mcp_user_token.is_empty());
    assert!(merged.workspace_hint.is_empty());
    assert!(!merged.enabled);
}

#[test]
fn merge_config_enabled_keeps_explicit_new_values() {
    let incoming = FeishuProjectConfig {
        enabled: true,
        mcp_user_token: "m-new-token".into(),
        workspace_hint: "new_ws".into(),
        ..Default::default()
    };
    let existing = existing_config();
    let merged = merge_config(incoming, Some(&existing));
    assert_eq!(merged.mcp_user_token, "m-new-token");
    assert_eq!(merged.workspace_hint, "new_ws");
}

#[test]
fn disable_runtime_state_has_no_stale_token_label() {
    let disabled_cfg = FeishuProjectConfig {
        enabled: false,
        mcp_user_token: "".into(),
        workspace_hint: "".into(),
        ..Default::default()
    };
    let existing = existing_config();
    let merged = merge_config(disabled_cfg, Some(&existing));
    let rs = FeishuProjectRuntimeState::from_config(&merged);
    assert!(!rs.enabled);
    assert_eq!(rs.token_label, None);
    assert_eq!(rs.workspace_hint, None);
}
