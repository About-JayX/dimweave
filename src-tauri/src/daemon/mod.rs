mod cmd;
pub mod codex;
pub mod control;
pub mod gui;
pub mod gui_task;
pub mod orchestrator;
mod permission;
pub mod provider;
pub mod role_config;
pub mod routing;
pub mod routing_display;
pub mod routing_user_input;
pub mod session_manager;
pub mod state;
pub mod task_graph;
pub mod types;
mod window_focus;

pub use cmd::{channel, is_valid_agent_role, DaemonCmd};
pub use state::DaemonState;

use crate::claude_session::ClaudeSessionManager;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, RwLock};

/// Shared daemon state accessible from all submodules.
pub type SharedState = Arc<RwLock<DaemonState>>;

async fn set_role(
    state: &SharedState,
    agent: &str,
    field: fn(&mut DaemonState) -> &mut String,
    new: String,
) -> bool {
    if !is_valid_agent_role(&new) {
        return false;
    }
    let mut s = state.write().await;
    if s.online_role_conflict(agent, &new).is_some() {
        return false;
    }
    let old = std::mem::replace(field(&mut s), new.clone());
    if old != new {
        s.migrate_buffered_role(&old, &new);
    }
    true
}

async fn apply_role(
    state: &SharedState,
    app: &AppHandle,
    agent: &str,
    role: String,
    field: fn(&mut DaemonState) -> &mut String,
) -> Result<(), String> {
    if set_role(state, agent, field, role.clone()).await {
        Ok(())
    } else {
        gui::emit_system_log(
            app,
            "warn",
            &format!("[Daemon] {agent} role rejected: {role}"),
        );
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
    state.write().await.invalidate_codex_session();
    gui::emit_agent_status(app, "codex", false, None);
}

async fn stop_claude_session(claude_manager: &ClaudeSessionManager) {
    crate::claude_session::stop_if_running(claude_manager).await;
}

fn session_role_name(role: crate::daemon::task_graph::types::SessionRole) -> &'static str {
    match role {
        crate::daemon::task_graph::types::SessionRole::Lead => "lead",
        crate::daemon::task_graph::types::SessionRole::Coder => "coder",
    }
}

async fn attach_provider_history(
    provider: crate::daemon::task_graph::types::Provider,
    external_id: String,
    cwd: String,
    role: crate::daemon::task_graph::types::SessionRole,
    codex_handle: &mut Option<codex::CodexHandle>,
    claude_manager: Arc<ClaudeSessionManager>,
    state: &SharedState,
    app: &AppHandle,
) -> Result<String, String> {
    if let Some(existing_session_id) = {
        let daemon = state.read().await;
        daemon
            .task_graph
            .find_session_by_external_id(provider, &external_id)
            .map(|session| session.session_id.clone())
    } {
        return state.write().await.resume_session(&existing_session_id);
    }

    if state.read().await.active_task_id.is_none() {
        return Err("no active task selected".into());
    }

    match provider {
        crate::daemon::task_graph::types::Provider::Claude => {
            let role_id = session_role_name(role);
            if let Some(conflict_agent) = {
                let daemon = state.read().await;
                daemon.online_role_conflict("claude", role_id)
            } {
                return Err(format!(
                    "role '{role_id}' already in use by online {conflict_agent}"
                ));
            }
            stop_claude_session(claude_manager.as_ref()).await;
            crate::claude_launch::resume(
                &cwd,
                None,
                None,
                role_id,
                &external_id,
                None,
                None,
                claude_manager,
                app.clone(),
            )
            .await?;
            let transcript_path =
                crate::daemon::provider::claude::default_transcript_path(&cwd, &external_id)?
                    .to_string_lossy()
                    .to_string();
            let task_id = {
                let mut daemon = state.write().await;
                crate::daemon::provider::claude::register_on_launch(
                    &mut daemon,
                    role_id,
                    &cwd,
                    &external_id,
                    &transcript_path,
                );
                daemon
                    .active_task_id
                    .clone()
                    .ok_or_else(|| "no active task selected".to_string())?
            };
            Ok(task_id)
        }
        crate::daemon::task_graph::types::Provider::Codex => {
            let role_id = session_role_name(role).to_string();
            if let Some(conflict_agent) = {
                let daemon = state.read().await;
                daemon.online_role_conflict("codex", &role_id)
            } {
                return Err(format!(
                    "role '{role_id}' already in use by online {conflict_agent}"
                ));
            }
            stop_codex_session(codex_handle, state, app).await;
            let launch_epoch = state.write().await.begin_codex_launch();
            let handle = codex::resume(
                codex::ResumeOpts {
                    role_id: role_id.clone(),
                    cwd: cwd.clone(),
                    thread_id: external_id.clone(),
                    launch_epoch,
                    codex_port: 4500,
                },
                state.clone(),
                app.clone(),
            )
            .await
            .map_err(|err| err.to_string())?;
            *codex_handle = Some(handle);
            let task_id = {
                let mut daemon = state.write().await;
                crate::daemon::provider::codex::register_on_launch(
                    &mut daemon,
                    &role_id,
                    &cwd,
                    &external_id,
                );
                daemon
                    .active_task_id
                    .clone()
                    .ok_or_else(|| "no active task selected".to_string())?
            };
            Ok(task_id)
        }
    }
}

/// Emit a full task context sync for the selected task.
async fn emit_task_context_events(state: &SharedState, app: &AppHandle, task_id: &str) {
    let s = state.read().await;
    let sess: Vec<_> = s
        .task_graph
        .sessions_for_task(task_id)
        .into_iter()
        .cloned()
        .collect();
    let arts: Vec<_> = s
        .task_graph
        .artifacts_for_task(task_id)
        .into_iter()
        .cloned()
        .collect();
    let events =
        gui_task::build_task_context_events(s.task_graph.get_task(task_id), task_id, &sess, &arts);
    drop(s);
    for event in events {
        event.emit(app);
    }
}

pub async fn run(
    app: AppHandle,
    claude_manager: Arc<ClaudeSessionManager>,
    mut cmd_rx: mpsc::Receiver<DaemonCmd>,
) {
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
                reasoning_effort,
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
                let launch_epoch = state.write().await.begin_codex_launch();
                let launch_result = match codex::start(
                    codex::StartOpts {
                        role_id,
                        cwd,
                        model,
                        effort: reasoning_effort,
                        launch_epoch,
                        codex_port: 4500,
                    },
                    state.clone(),
                    app.clone(),
                )
                .await
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
                let _ = reply.send(());
                break;
            }
            DaemonCmd::RegisterClaudeLaunch {
                role_id,
                cwd,
                external_id,
                transcript_path,
                reply,
            } => {
                let task_id = {
                    let mut daemon = state.write().await;
                    crate::daemon::provider::claude::register_on_launch(
                        &mut daemon,
                        &role_id,
                        &cwd,
                        &external_id,
                        &transcript_path,
                    );
                    daemon.active_task_id.clone()
                };
                if let Some(task_id) = task_id {
                    emit_task_context_events(&state, &app, &task_id).await;
                }
                let _ = reply.send(Ok(()));
            }
            DaemonCmd::ReadClaudeRole { reply } => {
                let _ = reply.send(state.read().await.claude_role.clone());
            }
            DaemonCmd::SetClaudeRole { role: r, reply } => {
                let _ =
                    reply.send(apply_role(&state, &app, "claude", r, |s| &mut s.claude_role).await);
            }
            DaemonCmd::SetCodexRole { role: r, reply } => {
                let _ =
                    reply.send(apply_role(&state, &app, "codex", r, |s| &mut s.codex_role).await);
            }
            DaemonCmd::ReadStatusSnapshot { reply } => {
                let _ = reply.send(state.read().await.status_snapshot());
            }
            DaemonCmd::RespondPermission {
                request_id,
                behavior,
            } => {
                permission::handle_permission_verdict(&state, &app, request_id, behavior).await;
            }
            DaemonCmd::ForceDisconnectAgent { agent_id } => {
                let removed = state
                    .write()
                    .await
                    .attached_agents
                    .remove(&agent_id)
                    .is_some();
                if removed {
                    if agent_id == "claude" {
                        gui::emit_claude_stream(&app, gui::ClaudeStreamPayload::Reset);
                    }
                    gui::emit_agent_status(&app, &agent_id, false, None);
                    gui::emit_system_log(
                        &app,
                        "info",
                        &format!("[Daemon] force-disconnected {agent_id}"),
                    );
                }
            }
            DaemonCmd::CreateTask {
                workspace,
                title,
                reply,
            } => {
                let task = state
                    .write()
                    .await
                    .create_and_select_task(&workspace, &title);
                emit_task_context_events(&state, &app, &task.task_id).await;
                let _ = reply.send(task);
            }
            DaemonCmd::ListTasks { workspace, reply } => {
                let _ = reply.send(state.read().await.task_list(workspace.as_deref()));
            }
            DaemonCmd::SelectTask { task_id, reply } => {
                let result = state.write().await.select_task(&task_id);
                if result.is_ok() {
                    emit_task_context_events(&state, &app, &task_id).await;
                }
                let _ = reply.send(result.map(|_| ()));
            }
            DaemonCmd::GetTaskSnapshot { reply } => {
                let _ = reply.send(state.read().await.task_snapshot());
            }
            DaemonCmd::ApproveReview { reply } => {
                let effects = state.write().await.lead_approve_review_effects();
                for event in effects.ui_events {
                    event.emit(&app);
                }
                for msg in effects.released {
                    routing::route_message(&state, &app, msg).await;
                }
                let _ = reply.send(Ok(()));
            }
            DaemonCmd::ListSessionTree { task_id, reply } => {
                let _ = reply.send(state.read().await.session_tree(&task_id));
            }
            DaemonCmd::ListHistory { workspace, reply } => {
                let _ = reply.send(state.read().await.task_history(workspace.as_deref()));
            }
            DaemonCmd::ListProviderHistory { workspace, reply } => {
                let workspace = match workspace {
                    Some(workspace) => Some(workspace),
                    None => {
                        let daemon = state.read().await;
                        daemon
                            .active_task_id
                            .as_ref()
                            .and_then(|task_id| daemon.task_graph.get_task(task_id))
                            .map(|task| task.workspace_root.clone())
                    }
                };
                let entries = match workspace {
                    Some(workspace) => {
                        crate::daemon::provider::history::list_workspace_provider_history(
                            &state, &workspace, &app,
                        )
                        .await
                    }
                    None => Vec::new(),
                };
                let _ = reply.send(entries);
            }
            DaemonCmd::ResumeSession { session_id, reply } => {
                let session = state
                    .read()
                    .await
                    .task_graph
                    .get_session(&session_id)
                    .cloned();
                let result = match session {
                    Some(sess)
                        if sess.provider == crate::daemon::task_graph::types::Provider::Codex =>
                    {
                        let target = crate::daemon::provider::codex::build_resume_target(&sess);
                        match target {
                            Ok(target) => {
                                stop_codex_session(&mut codex_handle, &state, &app).await;
                                let launch_epoch = state.write().await.begin_codex_launch();
                                let role_id = match target.role {
                                    crate::daemon::task_graph::types::SessionRole::Lead => "lead",
                                    crate::daemon::task_graph::types::SessionRole::Coder => "coder",
                                }
                                .to_string();
                                match codex::resume(
                                    codex::ResumeOpts {
                                        role_id,
                                        cwd: target.cwd,
                                        thread_id: target.external_id,
                                        launch_epoch,
                                        codex_port: 4500,
                                    },
                                    state.clone(),
                                    app.clone(),
                                )
                                .await
                                {
                                    Ok(handle) => {
                                        codex_handle = Some(handle);
                                        state.write().await.resume_session(&session_id)
                                    }
                                    Err(err) => Err(err.to_string()),
                                }
                            }
                            Err(err) => Err(err),
                        }
                    }
                    Some(sess)
                        if sess.provider == crate::daemon::task_graph::types::Provider::Claude =>
                    {
                        let target = crate::daemon::provider::claude::build_resume_target(&sess);
                        match target {
                            Ok(target) => {
                                let role_id = match target.role {
                                    crate::daemon::task_graph::types::SessionRole::Lead => "lead",
                                    crate::daemon::task_graph::types::SessionRole::Coder => "coder",
                                };
                                if let Some(conflict_agent) = {
                                    let daemon = state.read().await;
                                    daemon.online_role_conflict("claude", role_id)
                                } {
                                    Err(format!(
                                        "role '{role_id}' already in use by online {conflict_agent}"
                                    ))
                                } else {
                                    stop_claude_session(claude_manager.as_ref()).await;
                                    match crate::claude_launch::resume(
                                        &target.cwd,
                                        None,
                                        None,
                                        role_id,
                                        &target.external_id,
                                        None,
                                        None,
                                        claude_manager.clone(),
                                        app.clone(),
                                    )
                                    .await
                                    {
                                        Ok(()) => {
                                            let mut daemon = state.write().await;
                                            if let Ok(path) =
                                                crate::daemon::provider::claude::default_transcript_path(
                                                    &target.cwd,
                                                    &target.external_id,
                                                )
                                            {
                                                let _ = daemon.task_graph.set_transcript_path(
                                                    &session_id,
                                                    &path.to_string_lossy(),
                                                );
                                            }
                                            daemon.resume_session(&session_id)
                                        }
                                        Err(err) => Err(err),
                                    }
                                }
                            }
                            Err(err) => Err(err),
                        }
                    }
                    Some(_) => state.write().await.resume_session(&session_id),
                    None => Err(format!("session not found: {session_id}")),
                };
                if let Ok(ref task_id) = result {
                    emit_task_context_events(&state, &app, task_id).await;
                }
                let _ = reply.send(result.map(|_| ()));
            }
            DaemonCmd::AttachProviderHistory {
                provider,
                external_id,
                cwd,
                role,
                reply,
            } => {
                let result = attach_provider_history(
                    provider,
                    external_id,
                    cwd,
                    role,
                    &mut codex_handle,
                    claude_manager.clone(),
                    &state,
                    &app,
                )
                .await;
                if let Ok(ref task_id) = result {
                    emit_task_context_events(&state, &app, task_id).await;
                }
                let _ = reply.send(result.map(|_| ()));
            }
        }
    }
}
