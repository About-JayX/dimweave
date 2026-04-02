//! Claude SDK module — direct `--sdk-url` WebSocket integration.
//!
//! Replaces the old PTY+channel+bridge approach. The daemon spawns Claude
//! as a subprocess with `--sdk-url ws://127.0.0.1:{port}/claude`, and
//! Claude connects back via WS (inbound) and HTTP POST (outbound events).

pub mod event_handler;
pub mod process;
pub mod protocol;
pub mod stdio;

use crate::daemon::{gui, SharedState};
use process::ClaudeLaunchOpts;
use std::sync::Arc;
use serde_json::Value;
use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Handle to a running Claude SDK subprocess.
pub struct ClaudeSdkHandle {
    process: Arc<Mutex<Option<tokio::process::Child>>>,
    cancel: CancellationToken,
}

impl ClaudeSdkHandle {
    /// Stop the Claude subprocess and cancel background tasks.
    pub async fn stop(&self) {
        self.cancel.cancel();
        if let Some(mut child) = self.process.lock().await.take() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
    }
}

/// Launch a Claude subprocess with `--sdk-url` and return a handle.
///
/// Prerequisites: the daemon must have already called `begin_claude_sdk_launch()`
/// and stored a `claude_sdk_ready_tx` oneshot in state. The axum server must
/// have `/claude` WS and `/claude/events` POST routes registered (see
/// `control/claude_sdk_handler.rs`).
pub async fn launch(
    opts: ClaudeLaunchOpts,
    state: SharedState,
    app: AppHandle,
) -> anyhow::Result<ClaudeSdkHandle> {
    let session_id = opts.session_id.clone();
    let role_id = opts.role.clone().unwrap_or_else(|| "lead".into());
    let is_resume = opts.resume.is_some();

    let ready_rx = {
        let (ready_tx, ready_rx) =
            tokio::sync::oneshot::channel::<tokio::sync::mpsc::Sender<String>>();
        let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<Vec<Value>>(256);
        let mut s = state.write().await;
        let _epoch = s.begin_claude_sdk_launch(opts.launch_nonce.clone());
        s.claude_sdk_ready_tx = Some(ready_tx);
        s.claude_sdk_event_tx = Some(event_tx);
        let event_state = state.clone();
        let event_app = app.clone();
        tokio::spawn(async move {
            while let Some(events) = event_rx.recv().await {
                crate::daemon::control::claude_sdk_handler::process_sdk_events(
                    &event_state,
                    &event_app,
                    events,
                )
                .await;
            }
        });
        ready_rx
    };

    let mut child = match process::spawn_claude(&opts) {
        Ok(child) => child,
        Err(err) => {
            state.write().await.invalidate_claude_sdk_session();
            return Err(err);
        }
    };
    stdio::spawn_stdio_drainers(child.stdout.take(), child.stderr.take());
    let child_arc = Arc::new(Mutex::new(Some(child)));
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Claude SDK] spawned session={session_id} role={role_id} resume={is_resume}"),
    );
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Claude Trace] {}", process::format_launch_trace(&opts)),
    );

    let cancel = CancellationToken::new();

    let connected = tokio::select! {
        result = ready_rx => result.is_ok(),
        _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => false,
        _ = poll_child_exit(&child_arc, false) => false,
    };

    if !connected {
        if let Some(mut c) = child_arc.lock().await.take() {
            let _ = c.kill().await;
        }
        state.write().await.invalidate_claude_sdk_session();
        anyhow::bail!("Claude SDK did not connect within 30s");
    }

    // Spawn a background monitor for process exit
    let monitor_child = child_arc.clone();
    let monitor_cancel = cancel.clone();
    let monitor_app = app.clone();
    let monitor_state = state.clone();
    let monitor_role = role_id.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = monitor_cancel.cancelled() => {}
            _ = poll_child_exit(&monitor_child, true) => {
                gui::emit_agent_status(&monitor_app, "claude", false, None, None);
                gui::emit_claude_stream(&monitor_app, gui::ClaudeStreamPayload::Done);
                gui::emit_system_log(
                    &monitor_app,
                    "info",
                    &format!("[Claude SDK] process exited, role={monitor_role}"),
                );
                monitor_state.write().await.invalidate_claude_sdk_session();
            }
        }
    });

    // Emit provider connection state
    let provider_session = crate::daemon::types::ProviderConnectionState {
        provider: crate::daemon::task_graph::types::Provider::Claude,
        external_session_id: session_id.clone(),
        cwd: opts.cwd.clone(),
        connection_mode: if is_resume {
            crate::daemon::types::ProviderConnectionMode::Resumed
        } else {
            crate::daemon::types::ProviderConnectionMode::New
        },
    };
    {
        let mut s = state.write().await;
        s.set_provider_connection("claude", provider_session.clone());
        s.claude_role = role_id.clone();
    }
    gui::emit_agent_status(&app, "claude", true, None, Some(provider_session.clone()));
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Claude SDK] ready session={session_id}"),
    );
    gui::emit_system_log(
        &app,
        "info",
        &format!(
            "[Claude Trace] chain=ready session={} provider_session={{provider=claude,external_session_id={},cwd={},connection_mode={}}}",
            session_id,
            provider_session.external_session_id,
            provider_session.cwd,
            provider_session.connection_mode.as_str(),
        ),
    );

    Ok(ClaudeSdkHandle {
        process: child_arc,
        cancel,
    })
}

/// Poll until the child process has exited. If `take` is true, takes
/// the child out of the Option on exit (used for the background monitor).
async fn poll_child_exit(child: &Arc<Mutex<Option<tokio::process::Child>>>, take: bool) {
    let interval = if take { 500 } else { 200 };
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(interval)).await;
        let mut guard = child.lock().await;
        if let Some(ref mut c) = *guard {
            match c.try_wait() {
                Ok(Some(_)) | Err(_) => {
                    if take {
                        *guard = None;
                    }
                    return;
                }
                Ok(None) => {}
            }
        } else {
            return;
        }
    }
}
