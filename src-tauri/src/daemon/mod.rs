pub mod claude_sdk;
mod cmd;
pub mod codex;
pub mod control;
pub mod gui;
pub mod gui_task;
pub mod image_compress;
mod launch_task_sync;
pub mod orchestrator;
mod permission;
pub mod ports;
pub mod provider;
pub mod role_config;
pub mod routing;
pub mod routing_display;
pub mod routing_format;
pub mod routing_target_session;
pub mod routing_user_input;
pub mod session_manager;
pub mod feishu_project_lifecycle;
mod feishu_project_task_link;
mod telegram_lifecycle;
pub mod state;
pub mod task_graph;
pub mod types;
pub mod types_dto;

pub use cmd::{channel, is_valid_agent_role, DaemonCmd};
pub use state::DaemonState;

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
    let task_id = state.write().await.invalidate_codex_session();
    gui::emit_agent_status(app, "codex", false, None, None);
    if let Some(task_id) = task_id {
        emit_task_context_events(state, app, &task_id).await;
    }
}

async fn stop_claude_sdk_session(
    handle: &mut Option<claude_sdk::ClaudeSdkHandle>,
    state: &SharedState,
    app: &AppHandle,
) {
    if let Some(h) = handle.take() {
        h.stop().await;
    }
    let task_id = state.write().await.invalidate_claude_sdk_session();
    gui::emit_agent_status(app, "claude", false, None, None);
    gui::emit_system_log(app, "info", "[Daemon] Claude SDK session stopped");
    if let Some(task_id) = task_id {
        emit_task_context_events(state, app, &task_id).await;
    }
}

async fn launch_claude_sdk(
    role_id: &str,
    cwd: &str,
    model: Option<String>,
    effort: Option<String>,
    resume_session_id: Option<String>,
    state: &SharedState,
    app: &AppHandle,
) -> Result<(claude_sdk::ClaudeSdkHandle, String), String> {
    if let Some(conflict_agent) = {
        let daemon = state.read().await;
        daemon.online_role_conflict("claude", role_id)
    } {
        let err = format!("role '{role_id}' already in use by online {conflict_agent}");
        gui::emit_system_log(app, "error", &format!("[Daemon] Claude SDK failed: {err}"));
        return Err(err);
    }
    // Previous session is already stopped by the daemon loop caller.
    let claude_bin = crate::claude_cli::resolve_claude_bin()?;
    let session_id = uuid::Uuid::new_v4().to_string();
    let external_session_id = resume_session_id
        .clone()
        .unwrap_or_else(|| session_id.clone());
    let launch_nonce = uuid::Uuid::new_v4().to_string();
    let mcp_config = crate::mcp::build_dimweave_mcp_config(cwd, role_id)?;
    gui::emit_system_log(
        app,
        "info",
        &format!("[Claude SDK] strict-mcp-config: {mcp_config}"),
    );

    let opts = claude_sdk::process::ClaudeLaunchOpts {
        claude_bin,
        role: Some(role_id.to_string()),
        cwd: cwd.to_string(),
        session_id: session_id.clone(),
        launch_nonce,
        model,
        effort,
        resume: resume_session_id,
        daemon_port: ports::PortConfig::from_env().daemon,
        mcp_config: Some(mcp_config),
    };

    match claude_sdk::launch(opts, state.clone(), app.clone()).await {
        Ok(handle) => Ok((handle, external_session_id)),
        Err(e) => {
            gui::emit_agent_status(app, "claude", false, None, None);
            gui::emit_system_log(
                app,
                "error",
                &format!("[Daemon] Claude SDK launch failed: {e}"),
            );
            Err(e.to_string())
        }
    }
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
    claude_sdk_handle: &mut Option<claude_sdk::ClaudeSdkHandle>,
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
            stop_claude_sdk_session(claude_sdk_handle, state, app).await;
            let (handle, _external_session_id) = launch_claude_sdk(
                role_id,
                &cwd,
                None,
                None,
                Some(external_id.clone()),
                state,
                app,
            )
            .await?;
            *claude_sdk_handle = Some(handle);
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
                    codex_port: ports::PortConfig::from_env().codex,
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
    gui_task::emit_task_context_events(state, app, task_id).await;
}

pub async fn run(app: AppHandle, mut cmd_rx: mpsc::Receiver<DaemonCmd>) {
    let ports = ports::PortConfig::from_env();
    let daemon_port = ports.daemon;
    let codex_port = ports.codex;
    let state: SharedState = Arc::new(RwLock::new(DaemonState::new()));
    // WS control server — bridge processes connect here
    {
        let s = state.clone();
        let a = app.clone();
        tokio::spawn(async move {
            if let Err(e) = control::server::start(daemon_port, s, a).await {
                eprintln!("[Daemon] control server error: {e}");
            }
        });
    }
    // Hydrate persisted Feishu Project inbox store and auto-start polling
    feishu_project_lifecycle::hydrate_store(&state).await;
    let mut feishu_project_handle: Option<crate::feishu_project::runtime::FeishuProjectHandle> =
        feishu_project_lifecycle::auto_start(&state, &app).await;
    let mut codex_handle: Option<codex::CodexHandle> = None;
    let mut claude_sdk_handle: Option<claude_sdk::ClaudeSdkHandle> = None;
    let mut telegram_handle: Option<crate::telegram::runtime::TelegramHandle> =
        telegram_lifecycle::auto_start(&state, &app).await;
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            DaemonCmd::SendUserInput { content, target, attachments } => {
                routing::route_user_input(&state, &app, content, target, attachments).await;
            }
            DaemonCmd::LaunchCodex {
                role_id,
                cwd,
                model,
                reasoning_effort,
                resume_thread_id,
                reply,
            } => {
                stop_codex_session(&mut codex_handle, &state, &app).await;
                if let Some(conflict_agent) = {
                    let daemon = state.read().await;
                    daemon.online_role_conflict("codex", &role_id)
                } {
                    let err = format!("role '{role_id}' already in use by online {conflict_agent}");
                    gui::emit_agent_status(&app, "codex", false, None, None);
                    gui::emit_system_log(
                        &app,
                        "error",
                        &format!("[Daemon] Codex start failed: {err}"),
                    );
                    let _ = reply.send(Err(err));
                    continue;
                }
                let launch_epoch = state.write().await.begin_codex_launch();
                let launch_result = match resume_thread_id {
                    Some(thread_id) => {
                        let resumed_thread_id = thread_id.clone();
                        let resume_role = role_id.clone();
                        let resume_cwd = cwd.clone();
                        match codex::resume(
                            codex::ResumeOpts {
                                role_id,
                                cwd,
                                thread_id,
                                launch_epoch,
                                codex_port,
                            },
                            state.clone(),
                            app.clone(),
                        )
                        .await
                        {
                            Ok(h) => {
                                codex_handle = Some(h);
                                let task_id = {
                                    let mut daemon = state.write().await;
                                    launch_task_sync::sync_codex_launch_into_task(
                                        &mut daemon,
                                        &resume_role,
                                        &resume_cwd,
                                        &resumed_thread_id,
                                    )
                                };
                                if let Some(task_id) = task_id {
                                    emit_task_context_events(&state, &app, &task_id).await;
                                }
                                Ok(())
                            }
                            Err(e) => {
                                gui::emit_agent_status(&app, "codex", false, None, None);
                                gui::emit_system_log(
                                    &app,
                                    "error",
                                    &format!("[Daemon] Codex start failed: {e}"),
                                );
                                Err(e.to_string())
                            }
                        }
                    }
                    None => match codex::start(
                        codex::StartOpts {
                            role_id,
                            cwd,
                            model,
                            effort: reasoning_effort,
                            launch_epoch,
                            codex_port,
                        },
                        state.clone(),
                        app.clone(),
                    )
                    .await
                    {
                        Ok(h) => {
                            codex_handle = Some(h);
                            if let Some(task_id) = state.read().await.active_task_id.clone() {
                                emit_task_context_events(&state, &app, &task_id).await;
                            }
                            Ok(())
                        }
                        Err(e) => {
                            gui::emit_agent_status(&app, "codex", false, None, None);
                            gui::emit_system_log(
                                &app,
                                "error",
                                &format!("[Daemon] Codex start failed: {e}"),
                            );
                            Err(e.to_string())
                        }
                    },
                };
                let _ = reply.send(launch_result);
            }
            DaemonCmd::StopCodex => stop_codex_session(&mut codex_handle, &state, &app).await,
            DaemonCmd::LaunchClaudeSdk {
                role_id,
                cwd,
                model,
                effort,
                resume_session_id,
                reply,
            } => {
                stop_claude_sdk_session(&mut claude_sdk_handle, &state, &app).await;
                let result = launch_claude_sdk(
                    &role_id,
                    &cwd,
                    model,
                    effort,
                    resume_session_id,
                    &state,
                    &app,
                )
                .await;
                match result {
                    Ok((handle, external_session_id)) => {
                        let transcript_path = match crate::daemon::provider::claude::default_transcript_path(
                                &cwd,
                                &external_session_id,
                            ) {
                            Ok(path) => path.to_string_lossy().to_string(),
                            Err(err) => {
                                let mut failed_handle = Some(handle);
                                stop_claude_sdk_session(&mut failed_handle, &state, &app).await;
                                let _ = reply.send(Err(err));
                                continue;
                            }
                        };
                        let task_id = {
                            let mut daemon = state.write().await;
                            launch_task_sync::sync_claude_launch_into_active_task(
                                &mut daemon,
                                &role_id,
                                &cwd,
                                &external_session_id,
                                &transcript_path,
                            )
                        };
                        claude_sdk_handle = Some(handle);
                        if let Some(task_id) = task_id {
                            emit_task_context_events(&state, &app, &task_id).await;
                        }
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            DaemonCmd::StopClaudeSdk => {
                stop_claude_sdk_session(&mut claude_sdk_handle, &state, &app).await;
            }
            DaemonCmd::Shutdown { reply } => {
                if let Some(mut h) = feishu_project_handle.take() {
                    h.stop().await;
                }
                stop_codex_session(&mut codex_handle, &state, &app).await;
                stop_claude_sdk_session(&mut claude_sdk_handle, &state, &app).await;
                let session_mgr = { state.read().await.session_mgr.clone() };
                session_mgr.lock().await.cleanup_all();
                state.write().await.teardown_runtime_handles_for_shutdown();
                let _ = reply.send(());
                break;
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
                    let task_id = if agent_id == "claude" {
                        state.write().await.clear_provider_connection("claude")
                    } else if agent_id == "codex" {
                        state.write().await.clear_provider_connection("codex")
                    } else {
                        None
                    };
                    if agent_id == "claude" {
                        gui::emit_claude_stream(&app, gui::ClaudeStreamPayload::Reset);
                    }
                    gui::emit_agent_status(&app, &agent_id, false, None, None);
                    gui::emit_system_log(
                        &app,
                        "info",
                        &format!("[Daemon] force-disconnected {agent_id}"),
                    );
                    if let Some(task_id) = task_id {
                        emit_task_context_events(&state, &app, &task_id).await;
                    }
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
                // Verify persistence succeeded (auto_save inside create is
                // fire-and-forget; re-save to get a reliable result).
                let save_result = state.read().await.save_task_graph();
                match save_result {
                    Ok(()) => gui::emit_task_save_status(&app, true, None, &task.task_id),
                    Err(e) => gui::emit_task_save_status(
                        &app,
                        false,
                        Some(e.to_string()),
                        &task.task_id,
                    ),
                }
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
            DaemonCmd::ClearActiveTask { reply } => {
                state.write().await.set_active_task(None);
                gui_task::TaskUiEvent::ActiveTaskChanged { task_id: None }.emit(&app);
                let _ = reply.send(Ok(()));
            }
            DaemonCmd::GetTaskSnapshot { reply } => {
                let _ = reply.send(state.read().await.task_snapshot());
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
                                        codex_port,
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
                                    stop_claude_sdk_session(&mut claude_sdk_handle, &state, &app)
                                        .await;
                                    match launch_claude_sdk(
                                        role_id,
                                        &target.cwd,
                                        None,
                                        None,
                                        Some(target.external_id.clone()),
                                        &state,
                                        &app,
                                    )
                                    .await
                                    {
                                        Ok((handle, _external_session_id)) => {
                                            claude_sdk_handle = Some(handle);
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
                                        Err(err) => Err(err.to_string()),
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
                    &mut claude_sdk_handle,
                    &state,
                    &app,
                )
                .await;
                if let Ok(ref task_id) = result {
                    emit_task_context_events(&state, &app, task_id).await;
                }
                let _ = reply.send(result.map(|_| ()));
            }
            // ── Feishu Project ────────────────────────────────────
            DaemonCmd::GetFeishuProjectState { reply } => {
                let rs = feishu_project_lifecycle::get_runtime_state(&state).await;
                let _ = reply.send(rs);
            }
            DaemonCmd::SaveFeishuProjectConfig { config, reply } => {
                let result = feishu_project_lifecycle::save_and_restart(
                    &state,
                    &app,
                    &mut feishu_project_handle,
                    config,
                )
                .await;
                let _ = reply.send(result);
            }
            DaemonCmd::FeishuProjectSyncNow { reply } => {
                let result = feishu_project_lifecycle::sync_now(&state, &app).await;
                let _ = reply.send(result);
            }
            DaemonCmd::FeishuProjectListItems { reply } => {
                let items = feishu_project_lifecycle::list_items(&state).await;
                let _ = reply.send(items);
            }
            DaemonCmd::FeishuProjectStartHandling {
                work_item_id,
                reply,
            } => {
                let result =
                    feishu_project_lifecycle::start_handling(&state, &app, &work_item_id).await;
                let _ = reply.send(result);
            }
            DaemonCmd::FeishuProjectSetIgnored {
                work_item_id,
                ignored,
                reply,
            } => {
                let result =
                    feishu_project_lifecycle::set_ignored(&state, &app, &work_item_id, ignored)
                        .await;
                let _ = reply.send(result);
            }
            // ── Telegram ─────────────────────────────────────────
            DaemonCmd::GetTelegramState { reply } => {
                let tg_state = telegram_lifecycle::get_runtime_state(&state, telegram_handle.is_some()).await;
                let _ = reply.send(tg_state);
            }
            DaemonCmd::SaveTelegramConfig {
                bot_token,
                enabled,
                notifications_enabled,
                reply,
            } => {
                let result = telegram_lifecycle::save_and_restart(
                    &state,
                    &app,
                    &mut telegram_handle,
                    bot_token,
                    enabled,
                    notifications_enabled,
                )
                .await;
                let _ = reply.send(result);
            }
            DaemonCmd::GenerateTelegramPairCode { reply } => {
                let result = telegram_lifecycle::generate_pair(&state, &app, &telegram_handle, telegram_handle.is_some()).await;
                let _ = reply.send(result);
            }
            DaemonCmd::ClearTelegramPairing { reply } => {
                let result = telegram_lifecycle::clear_pair(&state, &app, &telegram_handle, telegram_handle.is_some()).await;
                let _ = reply.send(result);
            }
        }
    }
}
