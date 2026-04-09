use super::gui;
use super::SharedState;
use tauri::AppHandle;

pub const WEBHOOK_PATH: &str = "/integrations/feishu-project/webhook";

pub async fn get_runtime_state(
    state: &SharedState,
) -> crate::feishu_project::types::FeishuProjectRuntimeState {
    let cfg = match crate::feishu_project::config::default_config_path()
        .and_then(|p| crate::feishu_project::config::load_config(&p))
    {
        Ok(cfg) => cfg,
        Err(_) => crate::feishu_project::types::FeishuProjectConfig::default(),
    };
    let rs = crate::feishu_project::types::FeishuProjectRuntimeState::from_config(
        &cfg,
        WEBHOOK_PATH,
    );
    rs
}

pub async fn save_config(
    state: &SharedState,
    app: &AppHandle,
    incoming: crate::feishu_project::types::FeishuProjectConfig,
) -> Result<crate::feishu_project::types::FeishuProjectRuntimeState, String> {
    let config_path =
        crate::feishu_project::config::default_config_path().map_err(|e| e.to_string())?;
    crate::feishu_project::config::save_config(&config_path, &incoming)
        .map_err(|e| e.to_string())?;

    let rs = get_runtime_state(state).await;
    gui::emit_feishu_project_state(app, &rs);
    Ok(rs)
}

pub async fn list_items(
    state: &SharedState,
) -> Vec<crate::feishu_project::types::FeishuProjectInboxItem> {
    state.read().await.feishu_project_store.items.clone()
}

/// Stub: real polling will be implemented in Task 2.
pub async fn sync_now(
    _state: &SharedState,
    _app: &AppHandle,
) -> Result<(), String> {
    // Task 2 will implement actual Feishu API polling here.
    Err("sync not yet implemented".into())
}

/// Stub: real task linking will be implemented in Task 4.
pub async fn start_handling(
    _state: &SharedState,
    _app: &AppHandle,
    _work_item_id: &str,
) -> Result<String, String> {
    // Task 4 will implement idempotent task creation/reuse here.
    Err("start_handling not yet implemented".into())
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
    // Persist store
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
