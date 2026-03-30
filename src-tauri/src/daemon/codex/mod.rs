pub(crate) mod handshake;
pub mod handler;
pub mod lifecycle;
mod runtime;
pub mod session;
mod structured_output;
pub(crate) mod ws_client;

use crate::daemon::{gui, role_config, SharedState};
use runtime::{ensure_port_available, spawn_health_monitor};
use session::SessionOpts;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;

pub struct CodexHandle {
    process: Arc<Mutex<Option<tokio::process::Child>>>,
    session_mgr: Arc<Mutex<crate::daemon::session_manager::SessionManager>>,
    session_id: String,
    cancel: CancellationToken,
    port: u16,
}

pub struct StartOpts {
    pub role_id: String,
    pub cwd: String,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub launch_epoch: u64,
    pub codex_port: u16,
}

impl CodexHandle {
    pub async fn stop(&self) {
        self.cancel.cancel();
        if let Some(mut child) = self.process.lock().await.take() {
            lifecycle::stop(&mut child, self.port).await;
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
) -> anyhow::Result<CodexHandle> {
    let StartOpts {
        role_id,
        cwd,
        model,
        effort,
        launch_epoch,
        codex_port,
    } = opts;
    let (sandbox_mode, approval_policy, network_access, base_instructions) = if let Some(rc) =
        role_config::get_role(&role_id)
    {
        (
            rc.sandbox_mode.to_string(),
            rc.approval_policy.to_string(),
            rc.network_access,
            Some(rc.base_instructions.to_string()),
        )
    } else {
        ("workspace-write".into(), "never".into(), false, None)
    };

    let session_mgr = state.read().await.session_mgr.clone();
    let session_id = session_mgr.lock().await.next_session_id();
    let codex_home = session_mgr.lock().await
        .create_session(&session_id, &sandbox_mode, &approval_policy)?;

    // If a previous Codex session crashed, a port-holder orphan may survive.
    // Proactively clean it before giving up on launch.
    ensure_port_available(codex_port, std::time::Duration::from_secs(5), |port| {
        lifecycle::kill_port_holder(port)
    })
    .await?;

    let child = lifecycle::start(codex_port, &codex_home, &cwd, &sandbox_mode, &approval_policy)
        .await?;
    let child_arc = Arc::new(Mutex::new(Some(child)));
    gui::emit_system_log(&app, "info", &format!("[Codex] spawned port={codex_port} role={role_id}"));

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

    let (inject_tx, inject_rx) = mpsc::channel::<(String, bool)>(64);
    let opts = SessionOpts {
        role_id: role_id.clone(),
        cwd: cwd.clone(),
        model,
        effort,
        sandbox_mode: Some(sandbox_mode),
        network_access,
        base_instructions,
    };

    let cancel = CancellationToken::new();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<String>();

    let state2 = state.clone();
    let app2 = app.clone();
    let cancel_session = cancel.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = cancel_session.cancelled() => {}
            _ = session::run(codex_port, launch_epoch, opts, state2, app2, inject_rx, ready_tx) => {}
        }
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

    let (attached, mut buffered) = {
        let mut s = state.write().await;
        s.codex_role = role_id.clone();
        let attached = s.attach_codex_session_if_current(launch_epoch, inject_tx.clone());
        let buffered = if attached {
            s.take_buffered_for(&role_id)
        } else {
            Vec::new()
        };
        (attached, buffered)
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
            let from_user = buffered[i].from == "user";
            let text = crate::daemon::routing::format_codex_input(&buffered[i]);
            if inject_tx.send((text, from_user)).await.is_err() {
                let mut s = state.write().await;
                for m in buffered.drain(i..) { s.buffer_message(m); }
                eprintln!("[Codex] inject replay failed, re-buffered tail");
                break;
            }
            i += 1;
        }
    }
    gui::emit_agent_status(&app, "codex", true, None);
    gui::emit_system_log(&app, "info", &format!("[Codex] ready role={role_id} thread={thread_id}"));

    spawn_health_monitor(child_arc.clone(), launch_epoch, state.clone(), app.clone(), cancel.clone());

    Ok(CodexHandle {
        process: child_arc,
        session_mgr,
        session_id,
        cancel,
        port: codex_port,
    })
}

#[cfg(test)]
#[path = "start_tests.rs"]
mod start_tests;
