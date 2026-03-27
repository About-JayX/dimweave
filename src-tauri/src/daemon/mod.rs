pub mod codex;
pub mod control;
pub mod gui;
pub mod role_config;
pub mod routing;
pub mod routing_display;
pub mod routing_user_input;
pub mod session_manager;
pub mod state;
pub mod types;
mod window_focus;

pub use state::DaemonState;

use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot, RwLock};

/// Shared daemon state accessible from all submodules.
pub type SharedState = Arc<RwLock<DaemonState>>;

pub enum DaemonCmd {
    SendUserInput { content: String, target: String },
    LaunchCodex {
        role_id: String, cwd: String, model: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    StopCodex,
    Shutdown { reply: oneshot::Sender<()> },
    ReadStatusSnapshot { reply: oneshot::Sender<types::DaemonStatusSnapshot> },
    ReadClaudeRole { reply: oneshot::Sender<String> },
    SetClaudeRole { role: String, reply: oneshot::Sender<Result<(), String>> },
    SetCodexRole { role: String, reply: oneshot::Sender<Result<(), String>> },
    RespondPermission { request_id: String, behavior: types::PermissionBehavior },
    /// Force-disconnect an agent by removing it from attached_agents.
    /// Dropping the sender causes the WS outbound task to end and the connection to close.
    ForceDisconnectAgent { agent_id: String },
}

/// Create the command channel.  Call before spawning to avoid the DaemonSender race.
pub fn channel() -> (mpsc::Sender<DaemonCmd>, mpsc::Receiver<DaemonCmd>) {
    mpsc::channel(64)
}

const AGENT_ROLES: &[&str] = &["lead", "coder", "reviewer"];

pub fn is_valid_agent_role(role: &str) -> bool { AGENT_ROLES.contains(&role) }

async fn set_role(
    state: &SharedState,
    agent: &str,
    field: fn(&mut DaemonState) -> &mut String,
    new: String,
) -> bool {
    if !is_valid_agent_role(&new) { return false; }
    let mut s = state.write().await;
    if s.online_role_conflict(agent, &new).is_some() { return false; }
    let old = std::mem::replace(field(&mut s), new.clone());
    if old != new { s.migrate_buffered_role(&old, &new); }
    true
}

async fn apply_role(
    state: &SharedState, app: &AppHandle, agent: &str, role: String,
    field: fn(&mut DaemonState) -> &mut String,
) -> Result<(), String> {
    if set_role(state, agent, field, role.clone()).await {
        Ok(())
    } else {
        gui::emit_system_log(app, "warn", &format!("[Daemon] {agent} role rejected: {role}"));
        Err(format!("role '{role}' conflict or invalid"))
    }
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
            DaemonCmd::SendUserInput { content, target } => {
                routing::route_user_input(&state, &app, content, target).await;
            }
            DaemonCmd::LaunchCodex {
                role_id,
                cwd,
                model,
                reply,
            } => {
                stop_codex_session(&mut codex_handle, &state, &app).await;
                if let Some(conflict_agent) = {
                    let daemon = state.read().await;
                    daemon.online_role_conflict("codex", &role_id)
                } {
                    let err = format!("role '{role_id}' already in use by online {conflict_agent}");
                    gui::emit_agent_status(&app, "codex", false, None);
                    gui::emit_system_log(
                        &app,
                        "error",
                        &format!("[Daemon] Codex start failed: {err}"),
                    );
                    let _ = reply.send(Err(err));
                    continue;
                }
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
            DaemonCmd::StopCodex => stop_codex_session(&mut codex_handle, &state, &app).await,
            DaemonCmd::Shutdown { reply } => {
                stop_codex_session(&mut codex_handle, &state, &app).await;
                let _ = reply.send(()); break;
            }
            DaemonCmd::ReadClaudeRole { reply } => {
                let _ = reply.send(state.read().await.claude_role.clone());
            }
            DaemonCmd::SetClaudeRole { role: r, reply } => {
                let _ = reply.send(apply_role(&state, &app, "claude", r, |s| &mut s.claude_role).await);
            }
            DaemonCmd::SetCodexRole { role: r, reply } => {
                let _ = reply.send(apply_role(&state, &app, "codex", r, |s| &mut s.codex_role).await);
            }
            DaemonCmd::ReadStatusSnapshot { reply } => {
                let snapshot = state.read().await.status_snapshot();
                let _ = reply.send(snapshot);
            }
            DaemonCmd::RespondPermission { request_id, behavior } => {
                handle_permission_verdict(&state, &app, request_id, behavior).await;
            }
            DaemonCmd::ForceDisconnectAgent { agent_id } => {
                let removed = {
                    let mut daemon = state.write().await;
                    daemon.attached_agents.remove(&agent_id).is_some()
                };
                if removed {
                    if agent_id == "claude" {
                        gui::emit_claude_stream(&app, gui::ClaudeStreamPayload::Reset);
                    }
                    gui::emit_agent_status(&app, &agent_id, false, None);
                    gui::emit_system_log(&app, "info", &format!("[Daemon] force-disconnected {agent_id}"));
                }
            }
        }
    }
}

async fn handle_permission_verdict(
    state: &SharedState,
    app: &AppHandle,
    request_id: String,
    behavior: types::PermissionBehavior,
) {
    let resolved = {
        let mut daemon = state.write().await;
        daemon.resolve_permission(&request_id, behavior, chrono::Utc::now().timestamp_millis() as u64)
    };
    let Some((agent_id, outbound)) = resolved else {
        gui::emit_system_log(app, "warn", &format!("[Daemon] permission {request_id} unknown/expired"));
        return;
    };
    let sender_tx = state.read().await.attached_agents.get(&agent_id).map(|s| s.tx.clone());
    let verdict = match &outbound {
        types::ToAgent::PermissionVerdict { verdict } => Some(verdict.clone()),
        _ => None,
    };
    match sender_tx {
        Some(tx) if tx.send(outbound).await.is_ok() => {
            gui::emit_system_log(app, "info", &format!("[Daemon] verdict delivered to {agent_id}"));
        }
        _ => {
            if let Some(v) = verdict {
                state.write().await.buffer_permission_verdict(&agent_id, v);
            }
            gui::emit_system_log(app, "warn",
                &format!("[Daemon] {agent_id} offline, buffered verdict {request_id}"));
        }
    }
}
