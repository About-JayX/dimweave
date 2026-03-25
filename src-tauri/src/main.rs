#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod codex;
mod daemon;
mod mcp;

use codex::auth::CodexProfile;
use codex::models::CodexModel;
use codex::oauth::{OAuthHandle, OAuthLaunchInfo};
use codex::usage::UsageSnapshot;
use daemon::{types::BridgeMessage, DaemonCmd};
use std::sync::Arc;
use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::mpsc;

// ── Daemon command sender ────────────────────────────────────────────────────

struct DaemonSender(mpsc::Sender<DaemonCmd>);

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

// ── Daemon messaging commands ─────────────────────────────────────────────────

#[tauri::command]
async fn daemon_send_message(
    msg: BridgeMessage,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::SendMessage(msg))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn daemon_launch_codex(
    role_id: String,
    cwd: String,
    model: Option<String>,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::LaunchCodex { role_id, cwd, model })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn daemon_stop_codex(sender: State<'_, DaemonSender>) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::StopCodex)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn daemon_set_claude_role(
    role: String,
    sender: State<'_, DaemonSender>,
) -> Result<(), String> {
    sender
        .0
        .send(DaemonCmd::SetClaudeRole(role))
        .await
        .map_err(|e| e.to_string())
}

// ── Auth / OAuth commands ─────────────────────────────────────────────────────

#[tauri::command]
async fn codex_login(app: tauri::AppHandle) -> Result<OAuthLaunchInfo, String> {
    let handle = app.state::<Arc<OAuthHandle>>();
    codex::oauth::start_login(handle.inner().clone()).await
}

#[tauri::command]
fn codex_cancel_login(app: tauri::AppHandle) -> bool {
    app.state::<Arc<OAuthHandle>>().cancel()
}

#[tauri::command]
async fn codex_logout() -> Result<(), String> {
    codex::oauth::do_logout().await
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(OAuthHandle::new()))
        .setup(|app| {
            // Create channel synchronously so DaemonSender is available immediately.
            // If manage() were called inside an async spawn, any command arriving
            // before the spawn completes would panic with "state not managed".
            let (cmd_tx, cmd_rx) = daemon::channel();
            app.handle().manage(DaemonSender(cmd_tx));

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(daemon::run(handle, cmd_rx));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_codex_account,
            refresh_usage,
            list_codex_models,
            pick_directory,
            mcp::register_mcp,
            mcp::check_mcp_registered,
            mcp::launch_claude_terminal,
            codex_login,
            codex_cancel_login,
            codex_logout,
            daemon_send_message,
            daemon_launch_codex,
            daemon_stop_codex,
            daemon_set_claude_role,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
