use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::{
    process::Command,
    sync::{mpsc, oneshot},
    time::{timeout, Duration, Instant},
};

use crate::codex::oauth_helpers::{pump_stream, LoginState, StreamEvent};

/// Where Claude's OAuth verification URL is surfaced on the dialog.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeLoginInfo {
    pub verification_uri: Option<String>,
}

/// `claude auth status` JSON output.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAuthStatus {
    #[serde(default)]
    pub logged_in: bool,
    #[serde(default)]
    pub auth_method: Option<String>,
    #[serde(default)]
    pub api_provider: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub org_name: Option<String>,
    #[serde(default)]
    pub subscription_type: Option<String>,
}

pub struct ClaudeAuthHandle {
    cancel_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl ClaudeAuthHandle {
    pub fn new() -> Self {
        Self {
            cancel_tx: Mutex::new(None),
        }
    }
    pub fn cancel(&self) -> bool {
        if let Some(tx) = self
            .cancel_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = tx.send(());
            true
        } else {
            false
        }
    }
}

fn resolve_claude_bin_path() -> Result<std::path::PathBuf, String> {
    crate::claude_cli::resolve_claude_bin()
}

/// Spawn `claude auth login`, surface the verification URI, and keep the
/// child running until it exits or we're cancelled.
pub async fn start_login(handle: Arc<ClaudeAuthHandle>) -> Result<ClaudeLoginInfo, String> {
    let bin = resolve_claude_bin_path()?;
    let mut child = Command::new(&bin)
        .arg("auth")
        .arg("login")
        .env("PATH", crate::claude_cli::enriched_path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn claude auth login: {e}"))?;

    let stdout = child.stdout.take().ok_or("no stdout pipe")?;
    let stderr = child.stderr.take().ok_or("no stderr pipe")?;
    let (tx, mut rx) = mpsc::unbounded_channel::<StreamEvent>();
    tokio::spawn(pump_stream(stdout, "stdout", tx.clone()));
    tokio::spawn(pump_stream(stderr, "stderr", tx));

    let mut state = LoginState::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    while state.verification_uri.is_none() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match timeout(remaining, rx.recv()).await {
            Ok(Some(event)) => state.apply(event),
            _ => break,
        }
        if state.all_closed() {
            break;
        }
    }

    let info = ClaudeLoginInfo {
        verification_uri: state.verification_uri.clone(),
    };

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    *handle.cancel_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(cancel_tx);

    tokio::spawn(async move {
        tokio::pin!(cancel_rx);
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    let _ = child.kill().await;
                    return;
                }
                maybe_event = rx.recv() => match maybe_event {
                    Some(event) => state.apply(event),
                    None => break,
                }
            }
        }
        let _ = child.wait().await;
    });

    Ok(info)
}

pub async fn do_logout() -> Result<(), String> {
    let bin = resolve_claude_bin_path()?;
    let status = Command::new(&bin)
        .arg("auth")
        .arg("logout")
        .env("PATH", crate::claude_cli::enriched_path())
        .status()
        .await
        .map_err(|e| format!("failed to run claude auth logout: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "claude auth logout exited with code {}",
            status.code().unwrap_or(-1)
        ))
    }
}

pub async fn get_status() -> Result<ClaudeAuthStatus, String> {
    let bin = resolve_claude_bin_path()?;
    let output = Command::new(&bin)
        .arg("auth")
        .arg("status")
        .env("PATH", crate::claude_cli::enriched_path())
        .output()
        .await
        .map_err(|e| format!("failed to run claude auth status: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "claude auth status exited with code {}: {}",
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<ClaudeAuthStatus>(stdout.trim())
        .map_err(|e| format!("parse claude auth status json: {e}; raw={stdout}"))
}
