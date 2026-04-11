use super::{
    config, issue_operator, mcp_client::McpClient, mcp_sync, store,
    types::{FeishuProjectConfig, FeishuSyncMode},
};
use crate::daemon::{gui, SharedState};
use tauri::AppHandle;
use tokio::sync::oneshot;

pub struct FeishuProjectHandle {
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl FeishuProjectHandle {
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Connect MCP client, discover tools, update runtime state.
pub async fn connect_and_discover(
    cfg: &FeishuProjectConfig,
    app: &AppHandle,
) -> Result<McpClient, String> {
    if cfg.mcp_user_token.is_empty() {
        return Err("MCP user token not configured".into());
    }
    let mut client = McpClient::new(&cfg.domain, &cfg.mcp_user_token);
    client.connect().await.map_err(|e| {
        gui::emit_system_log(app, "warn", &format!("[FeishuProject MCP] {e}"));
        e
    })?;
    gui::emit_system_log(
        app,
        "info",
        &format!(
            "[FeishuProject MCP] connected, {} tools discovered",
            client.catalog.tool_count()
        ),
    );
    Ok(client)
}

/// Lightweight connect: only initialize, skip tools/list.
/// For use in load_more and other calls that don't need the catalog.
pub async fn connect_lite(
    cfg: &FeishuProjectConfig,
    app: &AppHandle,
) -> Result<McpClient, String> {
    if cfg.mcp_user_token.is_empty() {
        return Err("MCP user token not configured".into());
    }
    let mut client = McpClient::new(&cfg.domain, &cfg.mcp_user_token);
    client.connect_lite().await.map_err(|e| {
        gui::emit_system_log(app, "warn", &format!("[FeishuProject MCP] {e}"));
        e
    })?;
    Ok(client)
}

/// Fetch the human-readable project name via `search_project_info`.
async fn fetch_project_name(client: &McpClient, workspace_hint: &str) -> Option<String> {
    if workspace_hint.is_empty() {
        return None;
    }
    let args = serde_json::json!({"project_key": workspace_hint});
    let result = client.call_tool("search_project_info", args).await.ok()?;
    let text = result
        .get("content")?
        .as_array()?
        .iter()
        .find(|e| e.get("type").and_then(|t| t.as_str()) == Some("text"))?
        .get("text")?
        .as_str()?;
    let parsed: serde_json::Value = serde_json::from_str(text).ok()?;
    parsed
        .get("name")
        .and_then(|n| n.as_str())
        .map(String::from)
}

/// Run a full MCP sync cycle: connect, discover, fetch items, upsert.
pub async fn run_mcp_sync_cycle(
    cfg: &FeishuProjectConfig,
    state: &SharedState,
    app: &AppHandle,
) -> Result<McpClient, String> {
    let client = connect_and_discover(cfg, app).await?;
    match mcp_sync::run_mcp_sync(&client, &cfg.workspace_hint, cfg.sync_mode).await {
        Ok(mut items) => {
            if cfg.sync_mode == FeishuSyncMode::Issues {
                mcp_sync::enrich_issues_with_operators(
                    &client,
                    &cfg.workspace_hint,
                    &mut items,
                )
                .await;
            }
            let count = items.len();
            let new_item_ids: Vec<String> = {
                let mut daemon = state.write().await;
                daemon.feishu_project_store.sync_replace(items)
            };
            persist_and_emit(state, app).await;
            gui::emit_system_log(
                app,
                "info",
                &format!("[FeishuProject MCP] synced {count} items, {} new", new_item_ids.len()),
            );
        }
        Err(e) => {
            gui::emit_system_log(
                app,
                "warn",
                &format!("[FeishuProject MCP] sync failed: {e}"),
            );
            update_mcp_state(cfg, &client, Some(e.clone()), state, app).await;
            return Err(e);
        }
    }
    Ok(client)
}

/// Start the MCP runtime loop with sync. Returns a handle to stop it.
pub async fn start_mcp_runtime(
    state: SharedState,
    app: AppHandle,
    cfg: FeishuProjectConfig,
) -> Result<FeishuProjectHandle, anyhow::Error> {
    let interval_mins = cfg.refresh_interval_minutes.max(1);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        match run_mcp_sync_cycle(&cfg, &state, &app).await {
            Ok(client) => update_mcp_state(&cfg, &client, None, &state, &app).await,
            Err(e) => update_mcp_state_error(&cfg, &e, &state, &app).await,
        }
        let interval = std::time::Duration::from_secs(interval_mins * 60);
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    gui::emit_system_log(&app, "info", "[FeishuProject MCP] stopped");
                    return;
                }
                _ = tokio::time::sleep(interval) => {
                    match run_mcp_sync_cycle(&cfg, &state, &app).await {
                        Ok(client) => update_mcp_state(&cfg, &client, None, &state, &app).await,
                        Err(e) => update_mcp_state_error(&cfg, &e, &state, &app).await,
                    }
                }
            }
        }
    });

    Ok(FeishuProjectHandle {
        shutdown_tx: Some(shutdown_tx),
    })
}

pub(crate) async fn persist_and_emit(state: &SharedState, app: &AppHandle) {
    let store = state.read().await.feishu_project_store.clone();
    if let Ok(path) = store::default_store_path() {
        let _ = store::save_store(&path, &store);
    }
    gui::emit_feishu_project_items(app, &store.items);
}

/// Carry forward existing filter options into a freshly rebuilt runtime state.
/// Called before replacing `feishu_project_runtime` so that options fetched by
/// `fetch_filter_options` survive sync/error rebuilds.
pub(crate) fn preserve_filter_options(
    prev: Option<&super::types::FeishuProjectRuntimeState>,
    target: &mut super::types::FeishuProjectRuntimeState,
) {
    if let Some(prev) = prev {
        target.status_options.clone_from(&prev.status_options);
        target.assignee_options.clone_from(&prev.assignee_options);
    }
}

pub(crate) async fn update_mcp_state(
    cfg: &FeishuProjectConfig,
    client: &McpClient,
    error: Option<String>,
    state: &SharedState,
    app: &AppHandle,
) {
    let project_name = fetch_project_name(client, &cfg.workspace_hint).await;
    let team_members = {
        let daemon = state.read().await;
        issue_operator::derive_team_members(&daemon.feishu_project_store.items)
    };
    let now = chrono::Utc::now().timestamp_millis() as u64;
    if let Ok(path) = config::default_config_path() {
        if let Ok(mut saved) = config::load_config(&path) {
            saved.last_sync_at = Some(now);
            saved.last_error = error.clone();
            let _ = config::save_config(&path, &saved);
        }
    }
    let mut rs = super::types::FeishuProjectRuntimeState::from_config(cfg);
    rs.project_name = project_name;
    rs.team_members = team_members;
    rs.mcp_status = client.status;
    rs.discovered_tool_count = client.catalog.tool_count();
    rs.last_sync_at = Some(now);
    rs.last_error = error;
    {
        let mut d = state.write().await;
        preserve_filter_options(d.feishu_project_runtime.as_ref(), &mut rs);
        d.feishu_project_runtime = Some(rs.clone());
    }
    gui::emit_feishu_project_state(app, &rs);
}

async fn update_mcp_state_error(
    cfg: &FeishuProjectConfig,
    error: &str,
    state: &SharedState,
    app: &AppHandle,
) {
    if let Ok(path) = config::default_config_path() {
        if let Ok(mut saved) = config::load_config(&path) {
            saved.last_error = Some(error.to_string());
            let _ = config::save_config(&path, &saved);
        }
    }
    let mut rs = super::types::FeishuProjectRuntimeState::from_config(cfg);
    rs.last_error = Some(error.to_string());
    rs.mcp_status = super::types::McpConnectionStatus::Error;
    {
        let mut d = state.write().await;
        preserve_filter_options(d.feishu_project_runtime.as_ref(), &mut rs);
        d.feishu_project_runtime = Some(rs.clone());
    }
    gui::emit_feishu_project_state(app, &rs);
}
