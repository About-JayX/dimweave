#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod codex;
mod daemon;

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

// ── Misc commands ─────────────────────────────────────────────────────────────

#[tauri::command]
fn register_mcp() -> Result<bool, String> {
    let bridge_cmd = if cfg!(debug_assertions) {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let project_root =
            std::path::Path::new(manifest_dir).parent().unwrap_or(std::path::Path::new("."));
        let bridge_bin = project_root.join("target").join("debug").join("agent-bridge-bridge");
        bridge_bin.to_string_lossy().to_string()
    } else {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        exe.parent()
            .unwrap_or(std::path::Path::new("."))
            .join("../Resources/agent-bridge-bridge")
            .to_string_lossy()
            .to_string()
    };
    write_mcp_config(&bridge_cmd, &[])
}

fn write_mcp_config(command: &str, args: &[&str]) -> Result<bool, String> {
    let home = dirs::home_dir().ok_or("cannot resolve home")?;
    let mcp_path = home.join(".claude").join("mcp.json");

    let mut config: serde_json::Value = if mcp_path.exists() {
        let raw = std::fs::read_to_string(&mcp_path).map_err(|e| format!("read error: {e}"))?;
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        std::fs::create_dir_all(mcp_path.parent().unwrap()).ok();
        serde_json::json!({})
    };

    let servers = config
        .as_object_mut()
        .ok_or("invalid mcp.json")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    let mut entry = serde_json::json!({ "command": command });
    if !args.is_empty() {
        entry["args"] = serde_json::json!(args);
    }

    servers
        .as_object_mut()
        .ok_or("invalid mcpServers")?
        .insert("agentbridge".to_string(), entry);

    let json =
        serde_json::to_string_pretty(&config).map_err(|e| format!("serialize error: {e}"))?;
    std::fs::write(&mcp_path, json).map_err(|e| format!("write error: {e}"))?;
    Ok(true)
}

#[tauri::command]
fn check_mcp_registered() -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };
    let raw = match std::fs::read_to_string(home.join(".claude").join("mcp.json")) {
        Ok(r) => r,
        Err(_) => return false,
    };
    let config: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(c) => c,
        Err(_) => return false,
    };
    config.pointer("/mcpServers/agentbridge").is_some()
}

#[tauri::command]
fn launch_claude_terminal(cwd: Option<String>) -> Result<(), String> {
    let dir = cwd.unwrap_or_else(|| ".".to_string());

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            r#"tell application "Terminal"
                activate
                do script "cd '{}' && claude"
            end tell"#,
            dir.replace("'", "'\\''")
        );
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(&script)
            .spawn()
            .map_err(|e| format!("failed: {e}"))?;
    }

    #[cfg(not(target_os = "macos"))]
    {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("cd '{}' && claude", dir))
            .spawn()
            .map_err(|e| format!("failed: {e}"))?;
    }

    Ok(())
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(OAuthHandle::new()))
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let sender = daemon::start(handle.clone()).await;
                handle.manage(DaemonSender(sender));
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_codex_account,
            refresh_usage,
            list_codex_models,
            pick_directory,
            register_mcp,
            check_mcp_registered,
            launch_claude_terminal,
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
