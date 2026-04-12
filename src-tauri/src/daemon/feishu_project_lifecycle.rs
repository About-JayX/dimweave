use super::gui;
use super::SharedState;
use tauri::AppHandle;

fn load_cfg() -> crate::feishu_project::types::FeishuProjectConfig {
    crate::feishu_project::config::default_config_path()
        .and_then(|p| crate::feishu_project::config::load_config(&p))
        .unwrap_or_default()
}

/// Merge incoming config with existing on-disk config.
/// Only preserves empty fields from the existing config when `incoming.enabled`
/// is true (edit-save). When disabling, empty fields stay empty (clear intent).
fn merge_config(
    mut incoming: crate::feishu_project::types::FeishuProjectConfig,
    existing: Option<&crate::feishu_project::types::FeishuProjectConfig>,
) -> crate::feishu_project::types::FeishuProjectConfig {
    if incoming.enabled {
        if let Some(existing) = existing {
            if incoming.mcp_user_token.trim().is_empty() {
                incoming.mcp_user_token = existing.mcp_user_token.clone();
            }
            if incoming.workspace_hint.trim().is_empty() {
                incoming.workspace_hint = existing.workspace_hint.clone();
            }
        }
    }
    incoming
}

pub async fn get_runtime_state(
    state: &SharedState,
) -> crate::feishu_project::types::FeishuProjectRuntimeState {
    if let Some(rs) = &state.read().await.feishu_project_runtime {
        return rs.clone();
    }
    crate::feishu_project::types::FeishuProjectRuntimeState::from_config(&load_cfg())
}

pub async fn save_and_restart(
    state: &SharedState,
    app: &AppHandle,
    handle: &mut Option<crate::feishu_project::runtime::FeishuProjectHandle>,
    incoming: crate::feishu_project::types::FeishuProjectConfig,
) -> Result<crate::feishu_project::types::FeishuProjectRuntimeState, String> {
    if let Some(h) = handle.take() {
        let mut h = h;
        h.stop().await;
    }
    // Clear stale runtime cache so get_runtime_state() rebuilds from fresh config
    state.write().await.feishu_project_runtime = None;
    let config_path =
        crate::feishu_project::config::default_config_path().map_err(|e| e.to_string())?;
    let existing = crate::feishu_project::config::load_config(&config_path).ok();
    let merged = merge_config(incoming, existing.as_ref());
    crate::feishu_project::config::save_config(&config_path, &merged)
        .map_err(|e| e.to_string())?;
    // Start MCP runtime if enabled and token is configured
    if merged.enabled && !merged.mcp_user_token.is_empty() {
        match crate::feishu_project::runtime::start_mcp_runtime(
            state.clone(),
            app.clone(),
            merged.clone(),
        )
        .await
        {
            Ok(h) => *handle = Some(h),
            Err(e) => {
                gui::emit_system_log(app, "error", &format!("[FeishuProject] start failed: {e}"));
                if let Ok(mut saved) = crate::feishu_project::config::load_config(&config_path) {
                    saved.last_error = Some(e.to_string());
                    let _ = crate::feishu_project::config::save_config(&config_path, &saved);
                }
                let rs = get_runtime_state(state).await;
                gui::emit_feishu_project_state(app, &rs);
                return Err(e.to_string());
            }
        }
    }
    let rs = get_runtime_state(state).await;
    gui::emit_feishu_project_state(app, &rs);
    Ok(rs)
}

pub async fn list_items(
    state: &SharedState,
) -> Vec<crate::feishu_project::types::FeishuProjectInboxItem> {
    state.read().await.feishu_issue_view.clone()
}

/// Trigger an immediate MCP sync cycle (manual "Sync now").
/// Updates runtime state (last_sync_at, mcp_status, project_name, team_members)
/// on both success and failure paths.
pub async fn sync_now(
    state: &SharedState,
    app: &AppHandle,
) -> Result<(), String> {
    let cfg = load_cfg();
    if cfg.mcp_user_token.is_empty() {
        return Err("MCP user token not configured".into());
    }
    let client = crate::feishu_project::runtime::run_mcp_sync_cycle(&cfg, state, app).await?;
    // Refresh runtime state so frontend sees updated last_sync_at / project_name / etc.
    crate::feishu_project::runtime::update_mcp_state(&cfg, &client, None, state, app).await;
    Ok(())
}

/// Load next page of issues (append to existing store).
pub async fn load_more(
    state: &SharedState,
    app: &AppHandle,
) -> Result<usize, String> {
    let cfg = load_cfg();
    if cfg.mcp_user_token.is_empty() {
        return Err("MCP user token not configured".into());
    }
    if cfg.sync_mode != crate::feishu_project::types::FeishuSyncMode::Issues {
        return Err("load_more only available in issues mode".into());
    }
    let offset = state.read().await.feishu_project_store.items.len() as u32;
    let client =
        crate::feishu_project::runtime::connect_lite(&cfg, app).await?;
    let items = crate::feishu_project::mcp_sync::sync_issues_page(
        &client,
        &cfg.workspace_hint,
        offset,
    )
    .await?;
    let count = items.len();
    {
        let mut daemon = state.write().await;
        for item in items {
            daemon.feishu_project_store.upsert(item);
        }
        // Unfiltered load-more: mirror full store to view
        daemon.feishu_issue_view = daemon.feishu_project_store.items.clone();
    }
    crate::feishu_project::runtime::persist_and_emit(state, app).await;
    gui::emit_system_log(
        app,
        "info",
        &format!("[FeishuProject] loaded {count} more items (offset {offset})"),
    );
    Ok(count)
}

/// Load next page of issues with filters (status + current owner via MQL).
/// Resets cursor when filter changes, both filters applied server-side.
pub async fn load_more_filtered(
    state: &SharedState,
    app: &AppHandle,
    filter: crate::feishu_project::types::IssueFilter,
) -> Result<usize, String> {
    let cfg = load_cfg();
    if cfg.mcp_user_token.is_empty() {
        return Err("MCP user token not configured".into());
    }
    if cfg.sync_mode != crate::feishu_project::types::FeishuSyncMode::Issues {
        return Err("load_more_filtered only available in issues mode".into());
    }
    let client = crate::feishu_project::runtime::connect_lite(&cfg, app).await?;
    let mut cursor = {
        let d = state.read().await;
        match &d.feishu_issue_cursor {
            Some(c) if c.filter == filter => c.clone(),
            _ => crate::feishu_project::issue_query::IssueQueryCursor {
                filter: filter.clone(),
                raw_offset: 0,
                exhausted: false,
            },
        }
    };
    if cursor.exhausted {
        return Ok(0);
    }
    let was_reset = cursor.raw_offset == 0;
    let items = crate::feishu_project::issue_query::fetch_filtered_page(
        &client,
        &cfg.workspace_hint,
        cursor.raw_offset,
        &filter,
    )
    .await?;
    let count = items.len();
    if (count as u32) < 50 {
        cursor.exhausted = true;
    }
    cursor.raw_offset += count as u32;
    {
        let mut daemon = state.write().await;
        for item in &items {
            daemon.feishu_project_store.upsert(item.clone());
        }
        // Filter changed → replace view; same filter → append
        if was_reset {
            daemon.feishu_issue_view = items;
        } else {
            daemon.feishu_issue_view.extend(items);
        }
        daemon.feishu_issue_cursor = Some(cursor);
    }
    crate::feishu_project::runtime::persist_and_emit(state, app).await;
    gui::emit_system_log(app, "info", &format!("[FeishuProject] filtered load: {count} items"));
    Ok(count)
}

/// Parse project name from MCP `search_project_info` response.
pub(crate) fn parse_project_name_from_response(response: &serde_json::Value) -> Option<String> {
    let text = response.get("content")?
        .as_array()?
        .iter()
        .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("text"))?
        .get("text")?
        .as_str()?;
    let parsed: serde_json::Value = serde_json::from_str(text).ok()?;
    parsed.get("name").and_then(|n| n.as_str()).map(String::from)
}

/// Write fetched filter options into daemon state.
/// Materializes runtime from config if absent, so options are never silently dropped.
pub(crate) fn apply_filter_options(
    d: &mut crate::daemon::state::DaemonState,
    status_options: Vec<String>,
    assignee_options: Vec<String>,
) {
    apply_filter_options_with_project(d, status_options, assignee_options, None)
}

/// Write fetched filter options and optionally persist a resolved project name.
pub(crate) fn apply_filter_options_with_project(
    d: &mut crate::daemon::state::DaemonState,
    status_options: Vec<String>,
    assignee_options: Vec<String>,
    resolved_project_name: Option<String>,
) {
    let rs = d.feishu_project_runtime.get_or_insert_with(|| {
        crate::feishu_project::types::FeishuProjectRuntimeState::from_config(&load_cfg())
    });
    rs.status_options = status_options;
    rs.assignee_options = assignee_options;
    if let Some(name) = resolved_project_name {
        if rs.project_name.is_none() {
            rs.project_name = Some(name);
        }
    }
}

/// Fetch filter options (status labels via MQL GROUP BY + owner names via project team).
pub async fn fetch_filter_options(
    state: &SharedState,
    app: &AppHandle,
) -> Result<(), String> {
    let cfg = load_cfg();
    if cfg.mcp_user_token.is_empty() {
        return Err("MCP user token not configured".into());
    }
    let mut project_name = state.read().await
        .feishu_project_runtime.as_ref()
        .and_then(|r| r.project_name.clone());
    let client = crate::feishu_project::runtime::connect_lite(&cfg, app).await?;
    // Hydrate project_name from MCP if not yet in runtime state
    if project_name.is_none() && !cfg.workspace_hint.is_empty() {
        let args = serde_json::json!({"project_key": &cfg.workspace_hint});
        if let Ok(resp) = client.call_tool("search_project_info", args).await {
            project_name = parse_project_name_from_response(&resp);
        }
    }
    let (statuses, assignees) = tokio::join!(
        crate::feishu_project::issue_query::fetch_status_options(&client, &cfg.workspace_hint),
        crate::feishu_project::issue_query_team::fetch_team_member_names(
            &client, &cfg.workspace_hint, project_name.as_deref(),
        ),
    );
    let status_options = statuses.unwrap_or_default();
    let assignee_options = assignees.unwrap_or_default();
    {
        let mut d = state.write().await;
        apply_filter_options_with_project(&mut d, status_options, assignee_options, project_name);
        if let Some(rs) = &d.feishu_project_runtime {
            gui::emit_feishu_project_state(app, rs);
        }
    }
    Ok(())
}

pub async fn start_handling(
    state: &SharedState,
    app: &AppHandle,
    work_item_id: &str,
) -> Result<String, String> {
    super::feishu_project_task_link::start_handling(state, app, work_item_id).await
}

pub async fn set_ignored(
    state: &SharedState,
    app: &AppHandle,
    work_item_id: &str,
    ignored: bool,
) -> Result<(), String> {
    let ok = {
        let mut daemon = state.write().await;
        let ok = daemon.feishu_project_store.set_ignored(work_item_id, ignored);
        if ok {
            if let Some(vi) = daemon.feishu_issue_view.iter_mut()
                .find(|i| i.work_item_id == work_item_id)
            {
                vi.ignored = ignored;
            }
        }
        ok
    };
    if !ok {
        return Err(format!("work item not found: {work_item_id}"));
    }
    if let Ok(path) = crate::feishu_project::store::default_store_path() {
        let store = state.read().await.feishu_project_store.clone();
        let _ = crate::feishu_project::store::save_store(&path, &store);
    }
    let view = state.read().await.feishu_issue_view.clone();
    gui::emit_feishu_project_items(app, &view);
    Ok(())
}

/// Load persisted inbox store into DaemonState on startup.
pub async fn hydrate_store(state: &SharedState) {
    if let Ok(path) = crate::feishu_project::store::default_store_path() {
        if let Ok(store) = crate::feishu_project::store::load_store(&path) {
            let mut d = state.write().await;
            d.feishu_issue_view = store.items.clone();
            d.feishu_project_store = store;
        }
    }
}

/// Auto-start MCP runtime on daemon boot if config is enabled.
pub async fn auto_start(
    state: &SharedState,
    app: &AppHandle,
) -> Option<crate::feishu_project::runtime::FeishuProjectHandle> {
    let cfg = load_cfg();
    if !cfg.enabled || cfg.mcp_user_token.is_empty() {
        return None;
    }
    match crate::feishu_project::runtime::start_mcp_runtime(state.clone(), app.clone(), cfg).await
    {
        Ok(h) => Some(h),
        Err(e) => {
            gui::emit_system_log(app, "warn", &format!("[FeishuProject] auto-start failed: {e}"));
            if let Ok(path) = crate::feishu_project::config::default_config_path() {
                if let Ok(mut saved) = crate::feishu_project::config::load_config(&path) {
                    saved.last_error = Some(e.to_string());
                    let _ = crate::feishu_project::config::save_config(&path, &saved);
                }
            }
            None
        }
    }
}

#[cfg(test)]
#[path = "feishu_project_lifecycle_tests.rs"]
mod tests;
