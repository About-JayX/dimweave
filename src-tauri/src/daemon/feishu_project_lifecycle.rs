use super::gui;
use super::SharedState;
use tauri::AppHandle;

fn load_cfg() -> crate::feishu_project::types::FeishuProjectConfig {
    crate::feishu_project::config::default_config_path()
        .and_then(|p| crate::feishu_project::config::load_config(&p))
        .unwrap_or_default()
}

pub async fn get_runtime_state(
    _state: &SharedState,
) -> crate::feishu_project::types::FeishuProjectRuntimeState {
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
    let config_path =
        crate::feishu_project::config::default_config_path().map_err(|e| e.to_string())?;
    crate::feishu_project::config::save_config(&config_path, &incoming)
        .map_err(|e| e.to_string())?;
    // Start MCP runtime if enabled and token is configured
    if incoming.enabled && !incoming.mcp_user_token.is_empty() {
        match crate::feishu_project::runtime::start_mcp_runtime(
            state.clone(),
            app.clone(),
            incoming.clone(),
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
    state.read().await.feishu_project_store.items.clone()
}

/// Trigger an immediate MCP sync cycle (manual "Sync now").
pub async fn sync_now(
    state: &SharedState,
    app: &AppHandle,
) -> Result<(), String> {
    let cfg = load_cfg();
    if cfg.mcp_user_token.is_empty() {
        return Err("MCP user token not configured".into());
    }
    crate::feishu_project::runtime::run_mcp_sync_cycle(&cfg, state, app)
        .await
        .map(|_| ())
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
        daemon.feishu_project_store.set_ignored(work_item_id, ignored)
    };
    if !ok {
        return Err(format!("work item not found: {work_item_id}"));
    }
    if let Ok(path) = crate::feishu_project::store::default_store_path() {
        let store = state.read().await.feishu_project_store.clone();
        let _ = crate::feishu_project::store::save_store(&path, &store);
    }
    let items = state.read().await.feishu_project_store.items.clone();
    gui::emit_feishu_project_items(app, &items);
    Ok(())
}

/// Load persisted inbox store into DaemonState on startup.
pub async fn hydrate_store(state: &SharedState) {
    if let Ok(path) = crate::feishu_project::store::default_store_path() {
        if let Ok(store) = crate::feishu_project::store::load_store(&path) {
            state.write().await.feishu_project_store = store;
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
