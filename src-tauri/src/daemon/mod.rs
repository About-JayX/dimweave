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
pub mod feishu_project_task_link;
mod telegram_lifecycle;
pub mod state;
pub mod task_graph;
pub mod task_runtime;
pub mod task_workspace;
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

async fn stop_codex_for_task(
    codex_handles: &mut std::collections::HashMap<String, codex::CodexHandle>,
    port_pool: &mut codex::port_pool::CodexPortPool,
    task_id: &str,
    state: &SharedState,
    app: &AppHandle,
) {
    if let Some(h) = codex_handles.remove(task_id) {
        port_pool.release(h.port, task_id, h.launch_id);
        h.stop().await;
    }
    let tid = state.write().await.invalidate_codex_task_session(task_id);
    if !state.read().await.is_codex_online() {
        gui::emit_agent_status(app, "codex", false, None, None);
    }
    if let Some(tid) = tid {
        emit_task_context_events(state, app, &tid).await;
    }
}

async fn stop_all_codex_sessions(
    codex_handles: &mut std::collections::HashMap<String, codex::CodexHandle>,
    port_pool: &mut codex::port_pool::CodexPortPool,
    state: &SharedState,
    app: &AppHandle,
) {
    let task_ids: Vec<String> = codex_handles.keys().cloned().collect();
    for tid in task_ids {
        stop_codex_for_task(codex_handles, port_pool, &tid, state, app).await;
    }
    // Fallback: clear singleton state if no task handles remain
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
    task_id: &str,
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

    match claude_sdk::launch(opts, task_id.to_string(), state.clone(), app.clone()).await {
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
    codex_handles: &mut std::collections::HashMap<String, codex::CodexHandle>,
    codex_port_pool: &mut codex::port_pool::CodexPortPool,
    claude_sdk_handle: &mut Option<claude_sdk::ClaudeSdkHandle>,
    codex_exit_tx: &tokio::sync::mpsc::UnboundedSender<codex::CodexExitNotice>,
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
            let attach_task_id = state.read().await.active_task_id.clone()
                .unwrap_or_default();
            let (handle, _external_session_id) = launch_claude_sdk(
                &attach_task_id,
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
                let tid = daemon
                    .active_task_id
                    .clone()
                    .ok_or_else(|| "no active task selected".to_string())?;
                crate::daemon::provider::claude::register_on_launch(
                    &mut daemon,
                    &tid,
                    role_id,
                    &cwd,
                    &external_id,
                    &transcript_path,
                );
                tid
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
            let attach_task_id = state.read().await.active_task_id.clone()
                .ok_or_else(|| "no active task selected".to_string())?;
            // Stop only this task's existing Codex session if any
            stop_codex_for_task(codex_handles, codex_port_pool, &attach_task_id, state, app).await;
            let launch_epoch = {
                let mut s = state.write().await;
                s.begin_codex_task_launch(&attach_task_id, 0)
                    .unwrap_or_else(|| s.begin_codex_launch())
            };
            let allocated_port = codex_port_pool
                .reserve(&attach_task_id, launch_epoch)
                .ok_or("no Codex port available in pool")?;
            {
                let mut s = state.write().await;
                if let Some(rt) = s.task_runtimes.get_mut(&attach_task_id) {
                    if let Some(ref mut slot) = rt.codex_slot {
                        slot.port = allocated_port;
                    }
                }
            }
            let handle = codex::resume(
                codex::ResumeOpts {
                    task_id: attach_task_id.clone(),
                    role_id: role_id.clone(),
                    cwd: cwd.clone(),
                    thread_id: external_id.clone(),
                    launch_epoch,
                    codex_port: allocated_port,
                },
                state.clone(),
                app.clone(),
                codex_exit_tx.clone(),
            )
            .await
            .map_err(|err| err.to_string())?;
            codex_handles.insert(attach_task_id.clone(), handle);
            codex_port_pool.promote(allocated_port, &attach_task_id, launch_epoch);
            {
                let mut daemon = state.write().await;
                crate::daemon::provider::codex::register_on_launch(
                    &mut daemon,
                    &attach_task_id,
                    &role_id,
                    &cwd,
                    &external_id,
                );
            }
            Ok(attach_task_id)
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
    let mut codex_port_pool = codex::port_pool::CodexPortPool::new(codex_port);
    let mut codex_handles: std::collections::HashMap<String, codex::CodexHandle> = std::collections::HashMap::new();
    let (codex_exit_tx, mut codex_exit_rx) =
        tokio::sync::mpsc::unbounded_channel::<codex::CodexExitNotice>();
    let mut claude_sdk_handle: Option<claude_sdk::ClaudeSdkHandle> = None;
    let mut telegram_handle: Option<crate::telegram::runtime::TelegramHandle> =
        telegram_lifecycle::auto_start(&state, &app).await;
    loop {
        // Drain Codex natural-exit notices so ports and handles are released promptly
        let cmd = tokio::select! {
            Some(notice) = codex_exit_rx.recv() => {
                // Guard: only act if the current handle matches the notice's launch_id
                let matches = codex_handles.get(&notice.task_id)
                    .map_or(false, |h| h.launch_id == notice.launch_id);
                if matches {
                    codex_handles.remove(&notice.task_id);
                }
                codex_port_pool.release(notice.port, &notice.task_id, notice.launch_id);
                continue;
            }
            cmd = cmd_rx.recv() => match cmd {
                Some(c) => c,
                None => break,
            },
        };
        match cmd {
            DaemonCmd::SendUserInput { content, target, attachments, task_id } => {
                routing::route_user_input(&state, &app, content, target, attachments, task_id).await;
            }
            DaemonCmd::LaunchCodex {
                task_id,
                role_id,
                cwd,
                model,
                reasoning_effort,
                resume_thread_id,
                reply,
            } => {
                let resolved_task_id = if task_id.is_empty() {
                    state.read().await.active_task_id.clone().unwrap_or_default()
                } else {
                    task_id
                };
                // Stop only this task's existing Codex session, not other tasks'
                stop_codex_for_task(&mut codex_handles, &mut codex_port_pool, &resolved_task_id, &state, &app).await;
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
                let launch_epoch = {
                    let mut s = state.write().await;
                    s.begin_codex_task_launch(&resolved_task_id, 0)
                        .unwrap_or_else(|| s.begin_codex_launch())
                };
                let allocated_port = match codex_port_pool.reserve(&resolved_task_id, launch_epoch) {
                    Some(p) => p,
                    None => {
                        gui::emit_system_log(
                            &app,
                            "error",
                            "[Daemon] no Codex port available in pool",
                        );
                        let _ = reply.send(Err("no Codex port available in pool".into()));
                        continue;
                    }
                };
                // Update placeholder port in the task slot
                {
                    let mut s = state.write().await;
                    if let Some(rt) = s.task_runtimes.get_mut(&resolved_task_id) {
                        if let Some(ref mut slot) = rt.codex_slot {
                            slot.port = allocated_port;
                        }
                    }
                }
                let launch_result = match resume_thread_id {
                    Some(thread_id) => {
                        let resumed_thread_id = thread_id.clone();
                        let resume_role = role_id.clone();
                        let resume_cwd = cwd.clone();
                        match codex::resume(
                            codex::ResumeOpts {
                                task_id: resolved_task_id.clone(),
                                role_id,
                                cwd,
                                thread_id,
                                launch_epoch,
                                codex_port: allocated_port,
                            },
                            state.clone(),
                            app.clone(),
                            codex_exit_tx.clone(),
                        )
                        .await
                        {
                            Ok(h) => {
                                codex_handles.insert(resolved_task_id.clone(), h);
                                codex_port_pool.promote(allocated_port, &resolved_task_id, launch_epoch);
                                let task_id = {
                                    let mut daemon = state.write().await;
                                    launch_task_sync::sync_codex_launch_into_task(
                                        &mut daemon,
                                        &resolved_task_id,
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
                                codex_port_pool.release(allocated_port, &resolved_task_id, launch_epoch);
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
                            task_id: resolved_task_id.clone(),
                            role_id,
                            cwd,
                            model,
                            effort: reasoning_effort,
                            launch_epoch,
                            codex_port: allocated_port,
                        },
                        state.clone(),
                        app.clone(),
                        codex_exit_tx.clone(),
                    )
                    .await
                    {
                        Ok(h) => {
                            codex_handles.insert(resolved_task_id.clone(), h);
                            codex_port_pool.promote(allocated_port, &resolved_task_id, launch_epoch);
                            if let Some(task_id) = state.read().await.active_task_id.clone() {
                                emit_task_context_events(&state, &app, &task_id).await;
                            }
                            Ok(())
                        }
                        Err(e) => {
                            codex_port_pool.release(allocated_port, &resolved_task_id, launch_epoch);
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
            DaemonCmd::StopCodex => {
                // StopCodex stops all Codex sessions (user-initiated global stop)
                stop_all_codex_sessions(&mut codex_handles, &mut codex_port_pool, &state, &app).await;
            }
            DaemonCmd::LaunchClaudeSdk {
                task_id,
                role_id,
                cwd,
                model,
                effort,
                resume_session_id,
                reply,
            } => {
                stop_claude_sdk_session(&mut claude_sdk_handle, &state, &app).await;
                // Resolve task_id: explicit param > active_task_id
                let resolved_task_id = if task_id.is_empty() {
                    state.read().await.active_task_id.clone().unwrap_or_default()
                } else {
                    task_id
                };
                let result = launch_claude_sdk(
                    &resolved_task_id,
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
                        let synced_task_id = {
                            let mut daemon = state.write().await;
                            launch_task_sync::sync_claude_launch_into_task(
                                &mut daemon,
                                &resolved_task_id,
                                &role_id,
                                &cwd,
                                &external_session_id,
                                &transcript_path,
                            )
                        };
                        claude_sdk_handle = Some(handle);
                        if let Some(task_id) = synced_task_id {
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
                stop_all_codex_sessions(&mut codex_handles, &mut codex_port_pool, &state, &app).await;
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
                lead_provider,
                coder_provider,
                reply,
            } => {
                // 1. Create task (workspace_root = repo root initially)
                let task = {
                    let mut s = state.write().await;
                    let t = match (lead_provider, coder_provider) {
                        (Some(lp), Some(cp)) => {
                            s.task_graph.create_task_with_config(&workspace, &title, lp, cp)
                        }
                        _ => s.task_graph.create_task(&workspace, &title),
                    };
                    s.active_task_id = Some(t.task_id.clone());
                    t
                };
                let task_id = task.task_id.clone();

                // 2. Create isolated worktree, update workspace_root + init runtime
                let result = match task_workspace::create_task_worktree(
                    std::path::Path::new(&workspace),
                    &task_id,
                ) {
                    Ok(wt_path) => {
                        let mut s = state.write().await;
                        s.task_graph.update_workspace_root(
                            &task_id,
                            &wt_path.to_string_lossy(),
                        );
                        s.init_task_runtime(&task_id, wt_path);
                        let updated = s.task_graph.get_task(&task_id).cloned()
                            .ok_or_else(|| "task disappeared".to_string());
                        updated
                    }
                    Err(e) => Err(e),
                };

                match &result {
                    Ok(_task) => {
                        let save_result = state.read().await.save_task_graph();
                        match save_result {
                            Ok(()) => gui::emit_task_save_status(&app, true, None, &task_id),
                            Err(e) => gui::emit_task_save_status(
                                &app, false, Some(e.to_string()), &task_id,
                            ),
                        }
                        emit_task_context_events(&state, &app, &task_id).await;
                    }
                    Err(e) => {
                        state.write().await.rollback_task_creation(&task_id);
                        gui::emit_system_log(
                            &app,
                            "error",
                            &format!("[Daemon] task worktree creation failed: {e}"),
                        );
                    }
                }
                let _ = reply.send(result);
            }
            DaemonCmd::UpdateTaskConfig {
                task_id,
                lead_provider,
                coder_provider,
                reply,
            } => {
                let mut s = state.write().await;
                let ok = s.task_graph.update_task_providers(
                    &task_id, lead_provider, coder_provider,
                );
                let result = if ok {
                    match s.task_graph.get_task(&task_id).cloned() {
                        Some(t) => Ok(t),
                        None => Err("task disappeared after update".to_string()),
                    }
                } else {
                    Err(format!("task {task_id} not found"))
                };
                drop(s);
                if result.is_ok() {
                    if let Ok(()) = state.read().await.save_task_graph() {
                        gui::emit_task_save_status(&app, true, None, &task_id);
                    }
                    emit_task_context_events(&state, &app, &task_id).await;
                }
                let _ = reply.send(result);
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
                                let resume_task_id = sess.task_id.clone();
                                stop_codex_for_task(&mut codex_handles, &mut codex_port_pool, &resume_task_id, &state, &app).await;
                                let launch_epoch = {
                                    let mut s = state.write().await;
                                    s.begin_codex_task_launch(&resume_task_id, 0)
                                        .unwrap_or_else(|| s.begin_codex_launch())
                                };
                                let alloc = codex_port_pool
                                    .reserve(&resume_task_id, launch_epoch)
                                    .ok_or_else(|| "no Codex port available".to_string());
                                match alloc {
                                    Err(e) => Err(e),
                                    Ok(allocated_port) => {
                                // Update placeholder port in the task slot
                                {
                                    let mut s = state.write().await;
                                    if let Some(rt) = s.task_runtimes.get_mut(&resume_task_id) {
                                        if let Some(ref mut slot) = rt.codex_slot {
                                            slot.port = allocated_port;
                                        }
                                    }
                                }
                                let role_id = match target.role {
                                    crate::daemon::task_graph::types::SessionRole::Lead => "lead",
                                    crate::daemon::task_graph::types::SessionRole::Coder => "coder",
                                }
                                .to_string();
                                match codex::resume(
                                    codex::ResumeOpts {
                                        task_id: resume_task_id.clone(),
                                        role_id,
                                        cwd: target.cwd,
                                        thread_id: target.external_id,
                                        launch_epoch,
                                        codex_port: allocated_port,
                                    },
                                    state.clone(),
                                    app.clone(),
                                    codex_exit_tx.clone(),
                                )
                                .await
                                {
                                    Ok(handle) => {
                                        codex_handles.insert(resume_task_id.clone(), handle);
                                        codex_port_pool.promote(allocated_port, &resume_task_id, launch_epoch);
                                        state.write().await.resume_session(&session_id)
                                    }
                                    Err(err) => {
                                        codex_port_pool.release(allocated_port, &sess.task_id, launch_epoch);
                                        Err(err.to_string())
                                    }
                                }
                                    }
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
                                    let resume_task_id = state.read().await.active_task_id
                                        .clone().unwrap_or_default();
                                    match launch_claude_sdk(
                                        &resume_task_id,
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
                    &mut codex_handles,
                    &mut codex_port_pool,
                    &mut claude_sdk_handle,
                    &codex_exit_tx,
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
            DaemonCmd::FeishuProjectLoadMore { reply } => {
                let result =
                    feishu_project_lifecycle::load_more(&state, &app).await;
                let _ = reply.send(result);
            }
            DaemonCmd::FeishuProjectLoadMoreFiltered { filter, reply } => {
                let result =
                    feishu_project_lifecycle::load_more_filtered(&state, &app, filter).await;
                let _ = reply.send(result);
            }
            DaemonCmd::FeishuProjectFetchFilterOptions { reply } => {
                let result =
                    feishu_project_lifecycle::fetch_filter_options(&state, &app).await;
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
