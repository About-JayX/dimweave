use crate::daemon::{gui, SharedState};
use axum::{http::StatusCode, Json, response::IntoResponse};
use tauri::AppHandle;

#[derive(Debug, Default, serde::Deserialize)]
pub(crate) struct LaunchNonceQuery {
    pub(crate) launch_nonce: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LaunchNonceError {
    Missing,
    Stale,
}

impl LaunchNonceError {
    fn status_code(self) -> StatusCode {
        match self {
            Self::Missing => StatusCode::BAD_REQUEST,
            Self::Stale => StatusCode::FORBIDDEN,
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Missing => "missing launch_nonce",
            Self::Stale => "invalid launch_nonce",
        }
    }
}

pub(crate) async fn current_launch_nonce(
    state: &SharedState,
    query: &LaunchNonceQuery,
) -> Result<String, LaunchNonceError> {
    let launch_nonce = query
        .launch_nonce
        .clone()
        .ok_or(LaunchNonceError::Missing)?;
    if state
        .read()
        .await
        .claude_sdk_accepts_launch_nonce(&launch_nonce)
    {
        Ok(launch_nonce)
    } else {
        Err(LaunchNonceError::Stale)
    }
}

pub(crate) fn launch_nonce_error_response(
    app: &AppHandle,
    transport: &str,
    query: &LaunchNonceQuery,
    err: LaunchNonceError,
) -> axum::response::Response {
    gui::emit_system_log(
        app,
        "warn",
        &format!(
            "[Claude Trace] chain={}_rejected reason={} launch_nonce={}",
            transport.to_lowercase(),
            err.message(),
            query
                .launch_nonce
                .as_deref()
                .map(crate::daemon::claude_sdk::process::redact_launch_nonce)
                .unwrap_or_else(|| "-".to_string()),
        ),
    );
    (
        err.status_code(),
        Json(serde_json::json!({"ok": false, "error": err.message()})),
    )
        .into_response()
}
