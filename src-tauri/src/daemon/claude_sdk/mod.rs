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
    task_id: String,
    agent_id: String,
    state: SharedState,
    app: AppHandle,
) -> anyhow::Result<ClaudeSdkHandle> {
    let cancel = CancellationToken::new();
    let (child_arc, epoch) = spawn_runtime(&opts, &task_id, &agent_id, state.clone(), app.clone()).await?;
    let monitor_cancel = cancel.clone();
    let monitor_app = app.clone();
    let monitor_state = state.clone();
    let monitor_role = opts.role.clone().unwrap_or_else(|| "lead".into());
    let monitor_opts = opts.clone();
    let monitor_child = child_arc.clone();
    let monitor_task_id = task_id;
    let monitor_agent_id = agent_id;
    tokio::spawn(async move {
        let mut current_child = monitor_child;
        let mut current_epoch = epoch;
        loop {
            tokio::select! {
                _ = monitor_cancel.cancelled() => return,
                ws_lost = wait_for_ws_disconnect(&monitor_state, &monitor_task_id, &monitor_agent_id, current_epoch) => {
                    if !ws_lost {
                        return;
                    }
                    if !recover_ws_connection(
                        &mut current_child,
                        &mut current_epoch,
                        &monitor_opts,
                        &monitor_task_id,
                        &monitor_agent_id,
                        &monitor_state,
                        &monitor_app,
                        &monitor_cancel,
                    ).await {
                        return;
                    }
                }
                _ = poll_child_exit(&current_child, true) => {
                    let (is_current, affected_task_id) = {
                        let mut s = monitor_state.write().await;
                        let is_current = s.claude_task_epoch_for_agent(&monitor_task_id, &monitor_agent_id)
                            == Some(current_epoch);
                        let tid = s.invalidate_claude_agent_session_if_current(
                            &monitor_task_id, &monitor_agent_id, current_epoch,
                        );
                        (is_current, tid)
                    };
                    if !is_current {
                        return;
                    }
                    emit_runtime_health(
                        &monitor_state,
                        &monitor_app,
                        crate::daemon::types::RuntimeHealthLevel::Error,
                        format!("Claude runtime exited for role={monitor_role}"),
                    ).await;
                    gui::emit_agent_status(&monitor_app, "claude", false, None, None);
                    gui::emit_claude_stream(&monitor_app, None, None, gui::ClaudeStreamPayload::Done);
                    gui::emit_system_log(
                        &monitor_app,
                        "info",
                        &format!("[Claude SDK] process exited, role={monitor_role}"),
                    );
                    if let Some(tid) = affected_task_id {
                        crate::daemon::gui_task::emit_task_context_events(
                            &monitor_state, &monitor_app, &tid,
                        ).await;
                    }
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
