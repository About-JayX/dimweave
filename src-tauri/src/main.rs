#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod claude;
#[allow(dead_code)]
mod claude_cli;
mod codex;
mod commands_artifact;
mod commands_history;
mod commands;
mod commands_task;
// TODO(audit-wave-2): pay down the pre-existing daemon lint debt and remove
// these daemon-scoped allow attributes once the legacy warnings are fixed.
#[allow(
    dead_code,
    clippy::items_after_test_module,
    clippy::large_enum_variant,
    clippy::needless_option_as_deref,
    clippy::too_many_arguments
)]
mod daemon;
#[allow(dead_code)]
mod mcp;
mod paste_attachments;
mod telegram;
mod commands_telegram;
mod feishu_project;
mod commands_feishu_project;

use codex::auth::CodexProfile;
use codex::models::CodexModel;
use codex::oauth::OAuthHandle;
use codex::usage::UsageSnapshot;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Manager, WindowEvent};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::mpsc;

// ── Daemon command sender ────────────────────────────────────────────────────

pub struct DaemonSender(mpsc::Sender<daemon::DaemonCmd>);

#[derive(Default)]
struct ExitState(AtomicBool);

fn request_app_shutdown(app: tauri::AppHandle) {
    if app.state::<ExitState>().0.swap(true, Ordering::SeqCst) {
        return;
    }
    let sender = app.state::<DaemonSender>().0.clone();
    tauri::async_runtime::spawn(async move {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        if sender
            .send(daemon::DaemonCmd::Shutdown { reply: reply_tx })
            .await
            .is_ok()
        {
            let _ = reply_rx.await;
        }
        app.exit(0);
    });
}

// ── Codex / account commands ─────────────────────────────────────────────────

#[tauri::command]
fn get_codex_account() -> Result<CodexProfile, String> {
    codex::auth::read_profile()
}

#[tauri::command]
async fn refresh_usage() -> Result<UsageSnapshot, String> {
    codex::usage::get_snapshot().await
}

#[tauri::command]
fn list_codex_models() -> Result<Vec<CodexModel>, String> {
    codex::models::list_models()
}

#[tauri::command]
async fn list_claude_models() -> Result<Vec<claude::models::ClaudeModel>, String> {
    claude::models::list_models().await
}

#[tauri::command]
async fn get_claude_profile() -> Result<claude::profile::ClaudeProfile, String> {
    claude::profile::get_profile().await
}

#[tauri::command]
async fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path.map(|p| p.to_string()));
    });
    rx.await.map_err(|_| "dialog cancelled".to_string())
}

#[tauri::command]
async fn pick_files(app: tauri::AppHandle) -> Result<Option<Vec<String>>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<Vec<String>>>();
    app.dialog().file().pick_files(move |paths| {
        let _ = tx.send(paths.map(|ps| ps.into_iter().map(|p| p.to_string()).collect()));
    });
    rx.await.map_err(|_| "dialog cancelled".to_string())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard::init())
        .manage(Arc::new(OAuthHandle::new()))
        .manage(ExitState::default())
        .setup(|app| {
            let (cmd_tx, cmd_rx) = daemon::channel();
            app.handle().manage(DaemonSender(cmd_tx));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(daemon::run(handle, cmd_rx));
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                if window
                    .app_handle()
                    .state::<ExitState>()
                    .0
                    .load(Ordering::SeqCst)
                {
                    return;
                }
                api.prevent_close();
                request_app_shutdown(window.app_handle().clone());
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_codex_account,
            refresh_usage,
            list_codex_models,
            list_claude_models,
            get_claude_profile,
            pick_directory,
            pick_files,
            mcp::register_mcp,
            mcp::check_mcp_registered,
            commands_artifact::daemon_get_artifact_detail,
            commands::oauth::codex_login,
            commands::oauth::codex_cancel_login,
            commands::oauth::codex_logout,
            commands::daemon_send_user_input,
            commands::daemon_launch_codex,
            commands::daemon_stop_codex,
            commands::daemon_set_claude_role,
            commands::daemon_set_codex_role,
            commands::daemon_respond_permission,
            commands::daemon_get_status_snapshot,
            commands_task::daemon_create_task,
            commands_task::daemon_update_task_config,
            commands_task::daemon_list_tasks,
            commands_task::daemon_select_task,
            commands_task::daemon_delete_task,
            commands_task::daemon_clear_active_task,
            commands_task::daemon_get_task_snapshot,
            commands_task::daemon_list_session_tree,
            commands_task::daemon_add_task_agent,
            commands_task::daemon_remove_task_agent,
            commands_task::daemon_update_task_agent,
            commands_task::daemon_reorder_task_agents,
            commands_history::daemon_list_history,
            commands_history::daemon_list_provider_history,
            commands_history::daemon_resume_session,
            commands_history::daemon_attach_provider_history,
            commands::stop_claude,
            commands::daemon_launch_claude_sdk,
            commands::daemon_stop_claude_sdk,
            commands_telegram::telegram_get_state,
            commands_telegram::telegram_save_config,
            commands_telegram::telegram_generate_pair_code,
            commands_telegram::telegram_clear_pairing,
            commands_feishu_project::feishu_project_get_state,
            commands_feishu_project::feishu_project_save_config,
            commands_feishu_project::feishu_project_sync_now,
            commands_feishu_project::feishu_project_list_items,
            commands_feishu_project::feishu_project_load_more,
            commands_feishu_project::feishu_project_load_more_filtered,
            commands_feishu_project::feishu_project_fetch_filter_options,
            commands_feishu_project::feishu_project_start_handling,
            commands_feishu_project::feishu_project_set_ignored,
            paste_attachments::read_paste_attachments,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if app_handle.state::<ExitState>().0.load(Ordering::SeqCst) {
                return;
            }
            api.prevent_exit();
            request_app_shutdown(app_handle.clone());
        }
    });
}
