pub mod handler;
pub mod lifecycle;
pub mod session;

use crate::daemon::{gui, role_config, SharedState};
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
    role_id: String,
    cwd: String,
    model: Option<String>,
    state: SharedState,
    app: AppHandle,
    codex_port: u16,
) -> anyhow::Result<CodexHandle> {
    // Resolve role config before touching session resources
    let (sandbox_mode, approval_policy, developer_instructions) =
        if let Some(rc) = role_config::get_role(&role_id) {
            (
                rc.sandbox_mode.to_string(),
                rc.approval_policy.to_string(),
                Some(rc.developer_instructions.to_string()),
            )
        } else {
            ("workspace-write".to_string(), "never".to_string(), None)
        };

    // Acquire the singleton session manager from daemon state
    let session_mgr = state.read().await.session_mgr.clone();

    let session_id = session_mgr.lock().await.next_session_id();
    let codex_home = session_mgr
        .lock()
        .await
        .create_session(&session_id, &sandbox_mode, &approval_policy)?;

    // Wait for port to be free before spawning (previous process may still hold it)
    let port_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
    while tokio::net::TcpStream::connect(format!("127.0.0.1:{codex_port}"))
        .await
        .is_ok()
    {
        if tokio::time::Instant::now() >= port_deadline {
            anyhow::bail!("Port {codex_port} still in use after 5s");
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }

    let child =
        lifecycle::start(codex_port, &codex_home, &cwd, &sandbox_mode, &approval_policy).await?;
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

    let (inject_tx, inject_rx) = mpsc::channel::<String>(64);
    let opts = SessionOpts {
        role_id: role_id.clone(),
        cwd: cwd.clone(),
        model,
        sandbox_mode: Some(sandbox_mode),
        developer_instructions,
    };

    let cancel = CancellationToken::new();
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<String>();

    let state2 = state.clone();
    let app2 = app.clone();
    let cancel_session = cancel.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = cancel_session.cancelled() => {}
            _ = session::run(codex_port, opts, state2, app2, inject_rx, ready_tx) => {}
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

    let buffered = {
        let mut s = state.write().await;
        s.codex_role = role_id.clone();
        s.codex_inject_tx = Some(inject_tx.clone());
        s.take_buffered_for(&role_id)
    };
    for msg in buffered {
        inject_tx
            .send(crate::daemon::routing::format_codex_input(&msg))
            .await
            .ok();
    }
    gui::emit_agent_status(&app, "codex", true, None);
    gui::emit_system_log(&app, "info", &format!("[Codex] ready role={role_id} thread={thread_id}"));

    // Health monitor: detect unexpected Codex process death
    let child_health = child_arc.clone();
    let state_health = state.clone();
    let app_health = app.clone();
    let cancel_health = cancel.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_health.cancelled() => return,
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
            }
            let mut guard = child_health.lock().await;
            if let Some(ref mut child) = *guard {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        eprintln!("[Codex] process exited: {status}");
                        state_health.write().await.codex_inject_tx = None;
                        gui::emit_agent_status(&app_health, "codex", false, None);
                        gui::emit_system_log(
                            &app_health,
                            "warn",
                            &format!("[Codex] process exited: {status}"),
                        );
                        return;
                    }
                    Ok(None) => {} // still running
                    Err(_) => return,
                }
            } else {
                return;
            }
        }
    });

    Ok(CodexHandle {
        process: child_arc,
        session_mgr,
        session_id,
        cancel,
        port: codex_port,
    })
}
