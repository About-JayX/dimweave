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
        status_options: vec!["处理中".into(), "已关闭".into()],
        assignee_options: vec!["Alice".into(), "Bob".into()],
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

fn make_item(
    work_item_id: &str,
    assignee: Option<&str>,
) -> crate::feishu_project::types::FeishuProjectInboxItem {
    crate::feishu_project::types::FeishuProjectInboxItem {
        record_id: format!("proj_{work_item_id}"),
        project_key: "proj".into(),
        work_item_id: work_item_id.into(),
        work_item_type_key: "issue".into(),
        title: format!("Issue {work_item_id}"),
        status_label: None,
        assignee_label: assignee.map(String::from),
        updated_at: 0,
        source_url: format!("https://project.feishu.cn/proj/issue/detail/{work_item_id}"),
        raw_snapshot_ref: String::new(),
        ignored: false,
        linked_task_id: None,
        last_ingress: crate::feishu_project::types::IngressSource::Mcp,
        last_event_uuid: None,
    }
}

/// After load-more appends enriched items, derive_team_members from the full
/// store must include assignees from both old and new pages.
#[tokio::test]
async fn enriched_load_more_items_contribute_to_team_members() {
    let mut ds = crate::daemon::state::DaemonState::new();
    ds.feishu_project_store.upsert(make_item("1001", Some("Alice")));
    let state = shared(ds);

    // Simulate load-more: new enriched items appended
    {
        let mut daemon = state.write().await;
        daemon.feishu_project_store.upsert(make_item("1002", Some("Bob")));
        daemon.feishu_project_store.upsert(make_item("1003", Some("Alice, Charlie")));
    }

    let members = {
        let daemon = state.read().await;
        crate::feishu_project::issue_operator::derive_team_members(
            &daemon.feishu_project_store.items,
        )
    };
    assert_eq!(members, vec!["Alice", "Bob", "Charlie"]);
}

/// Without an explicit runtime-state refresh after load-more, team_members
/// stays stale. After refresh it must reflect the full store.
#[tokio::test]
async fn runtime_team_members_refreshed_after_load_more() {
    let mut ds = crate::daemon::state::DaemonState::new();
    ds.feishu_project_runtime = Some(FeishuProjectRuntimeState {
        enabled: true,
        team_members: vec!["Alice".into()],
        sync_mode: crate::feishu_project::types::FeishuSyncMode::Issues,
        ..Default::default()
    });
    ds.feishu_project_store.upsert(make_item("1001", Some("Alice")));
    let state = shared(ds);

    // Simulate load-more: add enriched item with new assignee
    {
        let mut daemon = state.write().await;
        daemon.feishu_project_store.upsert(make_item("1002", Some("Bob")));
    }

    // Before refresh: runtime still stale
    let rs = get_runtime_state(&state).await;
    assert_eq!(rs.team_members, vec!["Alice".to_string()]);

    // Refresh (mirrors the logic added to load_more)
    let team_members = {
        let d = state.read().await;
        crate::feishu_project::issue_operator::derive_team_members(
            &d.feishu_project_store.items,
        )
    };
    {
        let mut d = state.write().await;
        if let Some(rs) = &mut d.feishu_project_runtime {
            rs.team_members = team_members;
        }
    }

    let rs = get_runtime_state(&state).await;
    assert!(rs.team_members.contains(&"Alice".to_string()));
    assert!(rs.team_members.contains(&"Bob".to_string()));
    assert_eq!(rs.team_members.len(), 2);
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

#[test]
fn from_config_initializes_empty_filter_options() {
    let cfg = existing_config();
    let rs = FeishuProjectRuntimeState::from_config(&cfg);
    assert!(rs.status_options.is_empty());
    assert!(rs.assignee_options.is_empty());
}

#[tokio::test]
async fn filter_options_hydrate_when_runtime_absent() {
    // Simulates the race: feishu_project_runtime is None when filter options arrive.
    let ds = crate::daemon::state::DaemonState::new();
    assert!(ds.feishu_project_runtime.is_none());
    let state = shared(ds);

    {
        let mut d = state.write().await;
        apply_filter_options(
            &mut d,
            vec!["处理中".into(), "已关闭".into()],
            vec!["Alice".into()],
        );
    }

    let d = state.read().await;
    assert!(
        d.feishu_project_runtime.is_some(),
        "runtime should be materialized by filter-option hydration"
    );
    let rs = d.feishu_project_runtime.as_ref().unwrap();
    assert_eq!(rs.status_options, vec!["处理中", "已关闭"]);
    assert_eq!(rs.assignee_options, vec!["Alice"]);
}

#[tokio::test]
async fn cursor_resets_when_filter_changes() {
    use crate::feishu_project::issue_query::IssueQueryCursor;
    use crate::feishu_project::types::IssueFilter;

    let mut ds = crate::daemon::state::DaemonState::new();
    ds.feishu_issue_cursor = Some(IssueQueryCursor {
        filter: IssueFilter { status: Some("处理中".into()), assignee: None },
        raw_offset: 100,
        exhausted: false,
    });
    let state = shared(ds);

    // Different filter should produce a fresh cursor
    let new_filter = IssueFilter { status: Some("已关闭".into()), assignee: None };
    let cursor = {
        let d = state.read().await;
        match &d.feishu_issue_cursor {
            Some(c) if c.filter == new_filter => c.clone(),
            _ => IssueQueryCursor {
                filter: new_filter.clone(),
                raw_offset: 0,
                exhausted: false,
            },
        }
    };
    assert_eq!(cursor.raw_offset, 0);
    assert_eq!(cursor.filter.status.as_deref(), Some("已关闭"));
}

#[tokio::test]
async fn cursor_continues_when_filter_same() {
    use crate::feishu_project::issue_query::IssueQueryCursor;
    use crate::feishu_project::types::IssueFilter;

    let filter = IssueFilter { status: Some("处理中".into()), assignee: None };
    let mut ds = crate::daemon::state::DaemonState::new();
    ds.feishu_issue_cursor = Some(IssueQueryCursor {
        filter: filter.clone(),
        raw_offset: 100,
        exhausted: false,
    });
    let state = shared(ds);

    let cursor = {
        let d = state.read().await;
        match &d.feishu_issue_cursor {
            Some(c) if c.filter == filter => c.clone(),
            _ => IssueQueryCursor {
                filter: filter.clone(),
                raw_offset: 0,
                exhausted: false,
            },
        }
    };
    assert_eq!(cursor.raw_offset, 100);
}
