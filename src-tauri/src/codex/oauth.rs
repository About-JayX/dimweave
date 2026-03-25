use serde::Serialize;
use std::sync::{Arc, Mutex};
use tokio::{
    process::Command,
    sync::{mpsc, oneshot},
    time::{timeout, Duration, Instant},
};

use std::process::Stdio;

use super::oauth_helpers::{find_codex, pump_stream, LoginState, StreamEvent};

// ── Types ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthLaunchInfo {
    pub verification_uri: Option<String>,
}

/// Shared state to allow cancellation from frontend.
pub struct OAuthHandle {
    cancel_tx: Mutex<Option<oneshot::Sender<()>>>,
}

impl OAuthHandle {
    pub fn new() -> Self {
        Self {
            cancel_tx: Mutex::new(None),
        }
    }

    pub fn cancel(&self) -> bool {
        if let Some(tx) = self.cancel_tx.lock().unwrap_or_else(|e| e.into_inner()).take() {
            let _ = tx.send(());
            true
        } else {
            false
        }
    }
}

// ── Public API ───────────────────────────────────────────

/// Launch `codex login` and return verification URL + completion future.
pub async fn start_login(handle: Arc<OAuthHandle>) -> Result<OAuthLaunchInfo, String> {
    let bin = find_codex()?;

    let mut child = Command::new(&bin)
        .arg("login")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn codex login: {e}"))?;

    let stdout = child.stdout.take().ok_or("no stdout pipe")?;
    let stderr = child.stderr.take().ok_or("no stderr pipe")?;
    let (tx, mut rx) = mpsc::unbounded_channel::<StreamEvent>();

    tokio::spawn(pump_stream(stdout, "stdout", tx.clone()));
    tokio::spawn(pump_stream(stderr, "stderr", tx));

    let mut state = LoginState::new();
    let launch_deadline = Instant::now() + Duration::from_secs(3);

    // Wait briefly for verification URI
    while state.verification_uri.is_none() {
        let remaining = launch_deadline.saturating_duration_since(Instant::now());
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

    let info = OAuthLaunchInfo {
        verification_uri: state.verification_uri.clone(),
    };

    // Set up cancellation
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    *handle.cancel_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(cancel_tx);

    // Spawn background task to wait for login completion
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

/// Launch `codex logout`.
pub async fn do_logout() -> Result<(), String> {
    let bin = find_codex()?;
    let status = Command::new(&bin)
        .arg("logout")
        .status()
        .await
        .map_err(|e| format!("failed to run codex logout: {e}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "codex logout exited with code {}",
            status.code().unwrap_or(-1)
        ))
    }
}
