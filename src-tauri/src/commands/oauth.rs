use crate::codex::oauth::{OAuthHandle, OAuthLaunchInfo};
use std::sync::Arc;
use tauri::Manager;

#[tauri::command]
pub async fn codex_login(app: tauri::AppHandle) -> Result<OAuthLaunchInfo, String> {
    let handle = app.state::<Arc<OAuthHandle>>();
    crate::codex::oauth::start_login(handle.inner().clone()).await
}

#[tauri::command]
pub fn codex_cancel_login(app: tauri::AppHandle) -> bool {
    app.state::<Arc<OAuthHandle>>().cancel()
}

#[tauri::command]
pub async fn codex_logout() -> Result<(), String> {
    crate::codex::oauth::do_logout().await
}

#[tauri::command]
pub async fn codex_login_with_api_key(api_key: String) -> Result<(), String> {
    crate::codex::oauth::login_with_api_key(api_key).await
}
