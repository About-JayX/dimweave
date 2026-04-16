pub mod handler;
pub(crate) mod handshake;
pub mod lifecycle;
pub mod port_pool;
mod runtime;
pub mod session;
mod structured_output;
pub(crate) mod ws_client;
mod ws_helpers;

use crate::daemon::{gui, role_config, SharedState};
use runtime::{ensure_port_available, spawn_health_monitor};
use session::SessionOpts;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

/// Notification sent from session/runtime tasks when a Codex session
/// exits naturally (process death or session loop end). The daemon loop
/// uses this to release the port lease and remove the stale task handle.
#[derive(Debug)]
pub struct CodexExitNotice {
    pub task_id: String,
    pub agent_id: String,
    pub port: u16,
    pub launch_id: u64,
}

pub struct CodexHandle {
    process: Arc<Mutex<Option<tokio::process::Child>>>,
    session_mgr: Arc<Mutex<crate::daemon::session_manager::SessionManager>>,
    session_id: String,
    cancel: CancellationToken,
    pub port: u16,
    pub launch_id: u64,
    pub task_id: String,
    pub agent_id: String,
}

pub struct StartOpts {
    pub task_id: String,
    pub agent_id: String,
    pub role_id: String,
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub launch_epoch: u64,
    pub codex_port: u16,
}

pub struct ResumeOpts {
    pub task_id: String,
    pub agent_id: String,
    pub role_id: String,
    pub cwd: String,
    pub thread_id: String,
    pub launch_epoch: u64,
    pub codex_port: u16,
}

enum LaunchMode {
    New(SessionOpts),
    Resume { role_id: String, thread_id: String },
}

impl CodexHandle {
    pub async fn stop(&self) {
        self.cancel.cancel();
        if let Some(mut c) = self.process.lock().await.take() {
            lifecycle::stop(&mut c, self.port).await;
        }
        self.session_mgr
            .lock()
            .await
            .cleanup_session(&self.session_id);
    }
}

/// Start a Codex app-server for the given role and wire it up to the daemon state.
pub async fn start(
    opts: StartOpts,
    state: SharedState,
    app: AppHandle,
    exit_tx: tokio::sync::mpsc::UnboundedSender<CodexExitNotice>,
) -> anyhow::Result<CodexHandle> {
    let StartOpts {
        task_id,
        agent_id,
        role_id,
        cwd,
        model,
        effort,
        launch_epoch,
        codex_port,
    } = opts;
    let (sandbox_mode, approval_policy, network_access, base_instructions) =
        resolve_role_launch_config(&role_id);
    let session_opts = SessionOpts {
        role_id: role_id.clone(),
        cwd: cwd.clone(),
        model,
        effort,
        sandbox_mode: Some(sandbox_mode.clone()),
        network_access,
        base_instructions,
    };
    launch(
        LaunchMode::New(session_opts),
        task_id,
        agent_id,
        role_id,
        cwd,
        launch_epoch,
        codex_port,
        sandbox_mode,
        approval_policy,
        state,
        app,
        exit_tx,
    )
    .await
}

pub async fn resume(
    opts: ResumeOpts,
    state: SharedState,
    app: AppHandle,
    exit_tx: tokio::sync::mpsc::UnboundedSender<CodexExitNotice>,
) -> anyhow::Result<CodexHandle> {
    let ResumeOpts {
        task_id,
        agent_id,
        role_id,
        cwd,
        thread_id,
        launch_epoch,
        codex_port,
    } = opts;
    let (sandbox_mode, approval_policy, _, _) = resolve_role_launch_config(&role_id);
    launch(
        LaunchMode::Resume {
            role_id: role_id.clone(),
            thread_id,
        },
        task_id,
        agent_id,
        role_id,
        cwd,
        launch_epoch,
        codex_port,
        sandbox_mode,
        approval_policy,
        state,
        app,
        exit_tx,
    )
    .await
}

fn resolve_role_launch_config(role_id: &str) -> (String, String, bool, Option<String>) {
    if let Some(rc) = role_config::get_role(role_id) {
        (
            rc.sandbox_mode.to_string(),
            rc.approval_policy.to_string(),
            rc.network_access,
            Some(rc.base_instructions.to_string()),
        )
    } else {
        ("workspace-write".into(), "never".into(), false, None)
    }
}

async fn launch(
    mode: LaunchMode,
    task_id: String,
    agent_id: String,
    role_id: String,
    cwd: String,
    launch_epoch: u64,
    codex_port: u16,
    sandbox_mode: String,
    approval_policy: String,
    state: SharedState,
    app: AppHandle,
    exit_tx: tokio::sync::mpsc::UnboundedSender<CodexExitNotice>,
) -> anyhow::Result<CodexHandle> {
    let session_mgr = state.read().await.session_mgr.clone();
    let session_id = session_mgr.lock().await.next_session_id();
    let codex_home =
        session_mgr
            .lock()
            .await
            .create_session(&session_id, &sandbox_mode, &approval_policy)?;

    // If a previous Codex session crashed, a port-holder orphan may survive.
    // Proactively clean it before giving up on launch.
    ensure_port_available(codex_port, std::time::Duration::from_secs(5), |port| {
        lifecycle::kill_port_holder(port)
    })
    .await?;

    let child = lifecycle::start(
        codex_port,
        &codex_home,
        &cwd,
        &sandbox_mode,
        &approval_policy,
    )
    .await?;
    let child_arc = Arc::new(Mutex::new(Some(child)));
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Codex] spawned port={codex_port} role={role_id}"),
    );

    // Poll until Codex app-server is accepting connections (up to 10 s)
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut poll_delay = std::time::Duration::from_millis(50);
    loop {
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{codex_port}"))
            .await
            .is_ok()
        {
            break;
        }
        // Check if child process exited prematurely
        if let Some(ref mut child) = *child_arc.lock().await {
            if let Ok(Some(status)) = child.try_wait() {
                anyhow::bail!("Codex process exited prematurely with status: {status}");
            }
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("Codex app-server did not start within 10 s");
        }
        tokio::time::sleep(poll_delay).await;
        poll_delay = (poll_delay * 2).min(std::time::Duration::from_millis(500));
    }

    let (inject_tx, inject_rx) = mpsc::channel::<(Vec<serde_json::Value>, bool)>(64);
    let cancel = CancellationToken::new();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<String>();

    let state2 = state.clone();
    let app2 = app.clone();
    let cancel_session = cancel.clone();
    let is_new_session = matches!(&mode, LaunchMode::New(_));
    let task_id_session = task_id.clone();
    let agent_id_session = agent_id.clone();
    let session_exit_tx = exit_tx.clone();
    let session_exit_port = codex_port;
    let session_exit_task_id = task_id.clone();
    let session_exit_agent_id = agent_id.clone();
    let session_exit_launch_id = launch_epoch;
    tokio::spawn(async move {
        tokio::select! {
            _ = cancel_session.cancelled() => {}
            _ = async {
                match mode {
                    LaunchMode::New(opts) => {
                        session::run(codex_port, launch_epoch, task_id_session, agent_id_session, opts, state2, app2, inject_rx, ready_tx).await;
                    }
                    LaunchMode::Resume { role_id, thread_id } => {
                        session::resume(codex_port, launch_epoch, task_id_session, agent_id_session, role_id, thread_id, state2, app2, inject_rx, ready_tx).await;
                    }
                }
            } => {}
        }
        // Notify daemon loop so it can release port lease and remove handle
        let _ = session_exit_tx.send(CodexExitNotice {
            task_id: session_exit_task_id,
            agent_id: session_exit_agent_id,
            port: session_exit_port,
            launch_id: session_exit_launch_id,
        });
    });

    // Wait for session handshake to complete before declaring connected
    let thread_id = match ready_rx.await {
        Ok(tid) if !tid.is_empty() => tid,
        _ => {
            cancel.cancel();
            if let Some(mut child) = child_arc.lock().await.take() {
                lifecycle::stop(&mut child, codex_port).await;
            }
            session_mgr.lock().await.cleanup_session(&session_id);
            anyhow::bail!("Codex session handshake failed");
        }
    };

    let (attached, mut buffered, provider_session) = {
        let mut s = state.write().await;
        s.codex_role = role_id.clone();
        let provider_session = crate::daemon::types::ProviderConnectionState {
            provider: crate::daemon::task_graph::types::Provider::Codex,
            external_session_id: thread_id.clone(),
            cwd: cwd.clone(),
            connection_mode: if is_new_session {
                crate::daemon::types::ProviderConnectionMode::New
            } else {
                crate::daemon::types::ProviderConnectionMode::Resumed
            },
        };
        let attached = s.attach_codex_task_session_for_agent(
            &task_id, &agent_id, launch_epoch, inject_tx.clone(), Some(provider_session.clone()),
        );
        let buffered = if attached {
            if is_new_session {
                crate::daemon::provider::codex::register_on_launch(
                    &mut s, &task_id, &role_id, &cwd, &thread_id, Some(&agent_id),
                );
            } else if let Some(existing_session_id) = s
                .task_graph
                .find_session_by_external_id(
                    crate::daemon::task_graph::types::Provider::Codex,
                    &thread_id,
                )
                .map(|session| session.session_id.clone())
            {
                let _ = s.resume_session(&existing_session_id);
            }
            s.take_buffered_for(&role_id)
        } else {
            Vec::new()
        };
        (attached, buffered, provider_session)
    };
    if !attached {
        cancel.cancel();
        if let Some(mut child) = child_arc.lock().await.take() {
            lifecycle::stop(&mut child, codex_port).await;
        }
        session_mgr.lock().await.cleanup_session(&session_id);
        anyhow::bail!("Codex session was superseded before it became active");
    }
    {
        let mut i = 0;
        while i < buffered.len() {
            let from_user = buffered[i].is_from_user();
            let text = crate::daemon::routing::format_codex_input(&buffered[i]);
            if inject_tx.send((vec![serde_json::json!({"type":"text","text":text})], from_user)).await.is_err() {
                let mut s = state.write().await;
                for m in buffered.drain(i..) {
                    s.buffer_message(m);
                }
                eprintln!("[Codex] inject replay failed, re-buffered tail");
                break;
            }
            i += 1;
        }
    }
    gui::emit_agent_status_online(&app, "codex", Some(provider_session), role_id.clone());
    gui::emit_system_log(
        &app,
        "info",
        &format!("[Codex] ready role={role_id} thread={thread_id}"),
    );

    spawn_health_monitor(
        child_arc.clone(),
        task_id.clone(),
        agent_id.clone(),
        launch_epoch,
        codex_port,
        exit_tx,
        state.clone(),
        app.clone(),
        cancel.clone(),
    );

    Ok(CodexHandle {
        process: child_arc,
        session_mgr,
        session_id,
        cancel,
        port: codex_port,
        launch_id: launch_epoch,
        task_id,
        agent_id,
    })
}

#[cfg(test)]
#[path = "start_tests.rs"]
mod start_tests;
