#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod claude_cli;
mod claude_launch;
mod claude_session;
mod codex;
mod commands;
mod commands_task;
mod daemon;
mod mcp;

use codex::auth::CodexProfile;
use codex::models::CodexModel;
use codex::oauth::OAuthHandle;
use codex::usage::UsageSnapshot;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
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
    let claude_session = app
        .state::<Arc<claude_session::ClaudeSessionManager>>()
        .inner()
        .clone();
    tauri::async_runtime::spawn(async move {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        if sender
            .send(daemon::DaemonCmd::Shutdown { reply: reply_tx })
            .await
            .is_ok()
        {
            let _ = reply_rx.await;
        }
        claude_session::stop_if_running(claude_session.as_ref()).await;
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
async fn pick_directory(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<String>>();
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path.map(|p| p.to_string()));
    });
    rx.await.map_err(|_| "dialog cancelled".to_string())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(OAuthHandle::new()))
        .manage(Arc::new(claude_session::ClaudeSessionManager::default()))
        .manage(ExitState::default())
        .setup(|app| {
            let (cmd_tx, cmd_rx) = daemon::channel();
            app.handle().manage(DaemonSender(cmd_tx));

            let handle = app.handle().clone();
            let claude_manager = app
                .handle()
                .state::<Arc<claude_session::ClaudeSessionManager>>()
                .inner()
                .clone();
            tauri::async_runtime::spawn(daemon::run(handle, claude_manager, cmd_rx));
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
            pick_directory,
            mcp::register_mcp,
            mcp::check_mcp_registered,
            mcp::launch_claude_terminal,
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
            commands_task::daemon_list_tasks,
            commands_task::daemon_select_task,
            commands_task::daemon_get_task_snapshot,
            commands_task::daemon_approve_review,
            commands_task::daemon_list_session_tree,
            commands_task::daemon_list_history,
            commands_task::daemon_list_provider_history,
            commands_task::daemon_resume_session,
            commands_task::daemon_attach_provider_history,
            commands::stop_claude,
            commands::claude_terminal_input,
            commands::claude_terminal_resize,
            commands::daemon_launch_claude_sdk,
            commands::daemon_stop_claude_sdk,
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
