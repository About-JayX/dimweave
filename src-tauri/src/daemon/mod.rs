pub mod codex;
pub mod control;
pub mod gui;
pub mod role_config;
pub mod routing;
pub mod session_manager;
pub mod state;
pub mod types;

pub use state::DaemonState;

use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot, RwLock};

/// Shared daemon state accessible from all submodules.
pub type SharedState = Arc<RwLock<DaemonState>>;

/// Commands sent from Tauri commands/frontend to the daemon task.
pub enum DaemonCmd {
    /// Route a message (e.g. user input) to its target agent.
    SendMessage(types::BridgeMessage),
    /// Launch a Codex session for the given role.
    LaunchCodex {
        role_id: String,
        cwd: String,
        model: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    /// Stop the current Codex session.
    StopCodex,
    /// Stop active agents and let the app exit without orphaned child processes.
    Shutdown { reply: oneshot::Sender<()> },
    /// Read the current runtime status snapshot for frontend hydration.
    ReadStatusSnapshot {
        reply: oneshot::Sender<types::DaemonStatusSnapshot>,
    },
    /// Update which role Claude is playing (affects routing).
    SetClaudeRole(String),
    /// Send a permission verdict back to the bridge for Claude Code.
    RespondPermission {
        request_id: String,
        behavior: types::PermissionBehavior,
    },
}

/// Create the command channel.  Call before spawning to avoid the DaemonSender race.
pub fn channel() -> (mpsc::Sender<DaemonCmd>, mpsc::Receiver<DaemonCmd>) {
    mpsc::channel(64)
}

async fn stop_codex_session(
    codex_handle: &mut Option<codex::CodexHandle>,
    state: &SharedState,
    app: &AppHandle,
) {
    if let Some(h) = codex_handle.take() {
        h.stop().await;
    }
    let mut daemon = state.write().await;
    daemon.codex_inject_tx = None;
    drop(daemon);
    gui::emit_agent_status(app, "codex", false, None);
}

/// Run the daemon.  Consumes `cmd_rx`; should be spawned via `tauri::async_runtime::spawn`.
pub async fn run(app: AppHandle, mut cmd_rx: mpsc::Receiver<DaemonCmd>) {
    let state: SharedState = Arc::new(RwLock::new(DaemonState::new()));

    // WS control server — bridge processes connect here
    {
        let s = state.clone();
        let a = app.clone();
        tokio::spawn(async move {
            if let Err(e) = control::server::start(4502, s, a).await {
                eprintln!("[Daemon] control server error: {e}");
            }
        });
    }

    let mut codex_handle: Option<codex::CodexHandle> = None;

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            DaemonCmd::SendMessage(msg) => {
                routing::route_message(&state, &app, msg).await;
            }

            DaemonCmd::LaunchCodex {
                role_id,
                cwd,
                model,
                reply,
            } => {
                stop_codex_session(&mut codex_handle, &state, &app).await;
                let launch_result =
                    match codex::start(role_id, cwd, model, state.clone(), app.clone(), 4500).await
                    {
                        Ok(h) => {
                            codex_handle = Some(h);
                            Ok(())
                        }
                        Err(e) => {
                            gui::emit_agent_status(&app, "codex", false, None);
                            gui::emit_system_log(
                                &app,
                                "error",
                                &format!("[Daemon] Codex start failed: {e}"),
                            );
                            Err(e.to_string())
                        }
                    };
                let _ = reply.send(launch_result);
            }

            DaemonCmd::StopCodex => {
                stop_codex_session(&mut codex_handle, &state, &app).await;
            }

            DaemonCmd::Shutdown { reply } => {
                stop_codex_session(&mut codex_handle, &state, &app).await;
                let _ = reply.send(());
                break;
            }

            DaemonCmd::SetClaudeRole(role) => {
                state.write().await.claude_role = role;
            }

            DaemonCmd::ReadStatusSnapshot { reply } => {
                let snapshot = state.read().await.status_snapshot();
                let _ = reply.send(snapshot);
            }

            DaemonCmd::RespondPermission {
                request_id,
                behavior,
            } => {
                let resolved = {
                    let mut daemon = state.write().await;
                    daemon.resolve_permission(
                        &request_id,
                        behavior,
                        chrono::Utc::now().timestamp_millis() as u64,
                    )
                };

                let Some((agent_id, outbound)) = resolved else {
                    gui::emit_system_log(
                        &app,
                        "warn",
                        &format!("[Daemon] permission request {request_id} is unknown or expired"),
                    );
                    continue;
                };

                let sender = { state.read().await.attached_agents.get(&agent_id).cloned() };
                let verdict = match &outbound {
                    types::ToAgent::PermissionVerdict { verdict } => Some(verdict.clone()),
                    _ => None,
                };

                match sender {
                    Some(tx) if tx.send(outbound).await.is_ok() => {
                        gui::emit_system_log(
                            &app,
                            "info",
                            &format!("[Daemon] permission verdict delivered to {agent_id}"),
                        );
                    }
                    _ => {
                        if let Some(verdict) = verdict {
                            state
                                .write()
                                .await
                                .buffer_permission_verdict(&agent_id, verdict);
                        }
                        gui::emit_system_log(
                            &app,
                            "warn",
                            &format!(
                                "[Daemon] {agent_id} offline, buffered permission verdict for {request_id}"
                            ),
                        );
                    }
                }
            }
        }
    }
}
