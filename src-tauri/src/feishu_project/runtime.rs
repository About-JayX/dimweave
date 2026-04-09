use super::{config, mcp_client::McpClient, mcp_sync, store, types::FeishuProjectConfig};
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

/// Run a full MCP sync cycle: connect, discover, fetch items, upsert.
pub async fn run_mcp_sync_cycle(
    cfg: &FeishuProjectConfig,
    state: &SharedState,
    app: &AppHandle,
) -> Result<McpClient, String> {
    let client = connect_and_discover(cfg, app).await?;
    match mcp_sync::run_mcp_sync(&client, &cfg.workspace_hint).await {
        Ok(items) => {
            let count = items.len();
            {
                let mut daemon = state.write().await;
                for item in items {
                    daemon.feishu_project_store.upsert(item);
                }
            }
            persist_and_emit(state, app).await;
            gui::emit_system_log(
                app,
                "info",
                &format!("[FeishuProject MCP] synced {count} items"),
            );
        }
        Err(e) => {
            gui::emit_system_log(
                app,
                "warn",
                &format!("[FeishuProject MCP] sync failed: {e}"),
            );
            update_mcp_state(cfg, &client, Some(e.clone()), app).await;
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
            Ok(client) => update_mcp_state(&cfg, &client, None, &app).await,
            Err(e) => update_mcp_state_error(&cfg, &e, &app).await,
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
                        Ok(client) => update_mcp_state(&cfg, &client, None, &app).await,
                        Err(e) => update_mcp_state_error(&cfg, &e, &app).await,
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

async fn update_mcp_state(
    cfg: &FeishuProjectConfig,
    client: &McpClient,
    error: Option<String>,
    app: &AppHandle,
) {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    if let Ok(path) = config::default_config_path() {
        if let Ok(mut saved) = config::load_config(&path) {
            saved.last_sync_at = Some(now);
            saved.last_error = error.clone();
            let _ = config::save_config(&path, &saved);
        }
    }
    let mut rs = super::types::FeishuProjectRuntimeState::from_config(cfg);
    rs.mcp_status = client.status;
    rs.discovered_tool_count = client.catalog.tool_count();
    rs.last_sync_at = Some(now);
    rs.last_error = error;
    gui::emit_feishu_project_state(app, &rs);
}

async fn update_mcp_state_error(cfg: &FeishuProjectConfig, error: &str, app: &AppHandle) {
    if let Ok(path) = config::default_config_path() {
        if let Ok(mut saved) = config::load_config(&path) {
            saved.last_error = Some(error.to_string());
            let _ = config::save_config(&path, &saved);
        }
    }
    let mut rs = super::types::FeishuProjectRuntimeState::from_config(cfg);
    rs.last_error = Some(error.to_string());
    rs.mcp_status = super::types::McpConnectionStatus::Error;
    gui::emit_feishu_project_state(app, &rs);
}
