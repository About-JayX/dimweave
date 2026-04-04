//! Claude SDK module — direct `--sdk-url` WebSocket integration.
//!
//! Replaces the old PTY+channel+bridge approach. The daemon spawns Claude
//! as a subprocess with `--sdk-url ws://127.0.0.1:{port}/claude`, and
//! Claude connects back via WS (inbound) and HTTP POST (outbound events).

pub mod event_handler;
pub mod process;
pub mod protocol;
pub mod stdio;
mod reconnect;
mod runtime;

use crate::daemon::{gui, SharedState};
use process::ClaudeLaunchOpts;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use reconnect::{recover_ws_connection, wait_for_ws_disconnect};
use runtime::{emit_runtime_health, poll_child_exit, spawn_runtime};

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
    let cancel = CancellationToken::new();
    let (child_arc, epoch) = spawn_runtime(&opts, state.clone(), app.clone()).await?;
    let monitor_cancel = cancel.clone();
    let monitor_app = app.clone();
    let monitor_state = state.clone();
    let monitor_role = opts.role.clone().unwrap_or_else(|| "lead".into());
    let monitor_opts = opts.clone();
    let monitor_child = child_arc.clone();
    tokio::spawn(async move {
        let mut current_child = monitor_child;
        let mut current_epoch = epoch;
        loop {
            tokio::select! {
                _ = monitor_cancel.cancelled() => return,
                ws_lost = wait_for_ws_disconnect(&monitor_state, current_epoch) => {
                    if !ws_lost {
                        return;
                    }
                    if !recover_ws_connection(
                        &mut current_child,
                        &mut current_epoch,
                        &monitor_opts,
                        &monitor_state,
                        &monitor_app,
                        &monitor_cancel,
                    ).await {
                        return;
                    }
                }
                _ = poll_child_exit(&current_child, true) => {
                    let cleared = monitor_state
                        .write()
                        .await
                        .invalidate_claude_sdk_session_if_current(current_epoch);
                    if !cleared {
                        return;
                    }
                    emit_runtime_health(
                        &monitor_state,
                        &monitor_app,
                        crate::daemon::types::RuntimeHealthLevel::Error,
                        format!("Claude runtime exited for role={monitor_role}"),
                    ).await;
                    gui::emit_agent_status(&monitor_app, "claude", false, None, None);
                    gui::emit_claude_stream(&monitor_app, gui::ClaudeStreamPayload::Done);
                    gui::emit_system_log(
                        &monitor_app,
                        "info",
                        &format!("[Claude SDK] process exited, role={monitor_role}"),
                    );
                    return;
                }
            }
        }
    });

    Ok(ClaudeSdkHandle {
        process: child_arc,
        cancel,
    })
}
