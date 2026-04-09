use super::{api, config, store, types::FeishuProjectConfig};
use crate::daemon::{gui, SharedState};
use reqwest::Client;
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

/// Run a single poll cycle: fetch all work items, upsert into store, persist, emit.
pub async fn run_poll_cycle(
    client: &Client,
    cfg: &FeishuProjectConfig,
    state: &SharedState,
    app: &AppHandle,
) -> anyhow::Result<usize> {
    let result = api::poll_all_work_items(client, cfg).await?;
    if result.truncated {
        gui::emit_system_log(
            app,
            "warn",
            &format!(
                "[FeishuProject] workspace has {} items but filter API caps at 2000; results truncated",
                result.api_total
            ),
        );
    }
    let count = result.items.len();
    {
        let mut daemon = state.write().await;
        for item in result.items {
            daemon.feishu_project_store.upsert(item);
        }
    }
    persist_and_emit(state, app).await;
    let truncation_warning = if result.truncated {
        Some(format!("truncated: {} items but API cap is 2000", result.api_total))
    } else {
        None
    };
    update_config_after_poll(true, truncation_warning, app).await;
    Ok(count)
}

/// Start a background polling loop. Returns a handle to stop it.
pub async fn start_polling(
    state: SharedState,
    app: AppHandle,
    cfg: FeishuProjectConfig,
) -> anyhow::Result<FeishuProjectHandle> {
    let interval_mins = cfg.poll_interval_minutes.max(1);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();
    let client = Client::new();

    tokio::spawn(async move {
        // Run an initial poll immediately
        match run_poll_cycle(&client, &cfg, &state, &app).await {
            Ok(n) => {
                gui::emit_system_log(
                    &app,
                    "info",
                    &format!("[FeishuProject] initial poll: {n} items"),
                );
            }
            Err(e) => {
                update_config_after_poll(false, Some(e.to_string()), &app).await;
                gui::emit_system_log(
                    &app,
                    "warn",
                    &format!("[FeishuProject] initial poll failed: {e}"),
                );
            }
        }

        let interval = std::time::Duration::from_secs(interval_mins * 60);
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    gui::emit_system_log(&app, "info", "[FeishuProject] polling stopped");
                    return;
                }
                _ = tokio::time::sleep(interval) => {
                    match run_poll_cycle(&client, &cfg, &state, &app).await {
                        Ok(n) => {
                            gui::emit_system_log(
                                &app,
                                "info",
                                &format!("[FeishuProject] poll: {n} items"),
                            );
                        }
                        Err(e) => {
                            update_config_after_poll(false, Some(e.to_string()), &app).await;
                            gui::emit_system_log(
                                &app,
                                "warn",
                                &format!("[FeishuProject] poll failed: {e}"),
                            );
                        }
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

/// Update config timestamps/error after a poll attempt and emit fresh runtime state.
async fn update_config_after_poll(success: bool, error: Option<String>, app: &AppHandle) {
    let now = chrono::Utc::now().timestamp_millis() as u64;
    if let Ok(path) = config::default_config_path() {
        if let Ok(mut saved) = config::load_config(&path) {
            if success {
                saved.last_poll_at = Some(now);
                saved.last_sync_at = Some(now);
                saved.last_error = error; // truncation warning if any
            } else {
                saved.last_error = error;
            }
            let _ = config::save_config(&path, &saved);
            let rs = crate::feishu_project::types::FeishuProjectRuntimeState::from_config(
                &saved,
                crate::daemon::feishu_project_lifecycle::WEBHOOK_PATH,
            );
            gui::emit_feishu_project_state(app, &rs);
        }
    }
}
