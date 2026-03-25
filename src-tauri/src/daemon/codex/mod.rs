pub mod handler;
pub mod lifecycle;
pub mod session;

use crate::daemon::{gui, role_config, SharedState};
use session::SessionOpts;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};

pub struct CodexHandle {
    process: Arc<Mutex<Option<tokio::process::Child>>>,
    session_mgr: Arc<Mutex<crate::daemon::session_manager::SessionManager>>,
    session_id: String,
}

impl CodexHandle {
    pub async fn stop(&self) {
        if let Some(ref mut child) = *self.process.lock().await {
            lifecycle::stop(child).await;
        }
        self.session_mgr.lock().await.cleanup_session(&self.session_id);
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

    let child = lifecycle::start(codex_port, &codex_home, &cwd, &sandbox_mode, &approval_policy).await?;
    let child_arc = Arc::new(Mutex::new(Some(child)));

    gui::emit_system_log(
        &app,
        "info",
        &format!("[Codex] started port={codex_port} role={role_id}"),
    );

    // Poll until Codex app-server is accepting connections (up to 10 s)
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        if tokio::net::TcpStream::connect(format!("127.0.0.1:{codex_port}")).await.is_ok() {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("Codex app-server did not start within 10 s");
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let (inject_tx, inject_rx) = mpsc::channel::<String>(64);
    let opts = SessionOpts {
        role_id: role_id.clone(),
        cwd: cwd.clone(),
        model,
        sandbox_mode: Some(sandbox_mode),
        developer_instructions,
    };

    let state2 = state.clone();
    let app2 = app.clone();
    tokio::spawn(async move {
        session::run(codex_port, opts, state2, app2, inject_rx).await;
    });

    {
        let mut s = state.write().await;
        s.codex_role = role_id.clone();
        s.codex_inject_tx = Some(inject_tx.clone());
    }
    let buffered = state.write().await.take_buffered_for(&role_id);
    for msg in buffered {
        inject_tx
            .send(crate::daemon::routing::format_codex_input(&msg))
            .await
            .ok();
    }
    gui::emit_agent_status(&app, "codex", true, None);
    gui::emit_system_log(&app, "info", &format!("[Codex] session wired role={role_id}"));

    Ok(CodexHandle {
        process: child_arc,
        session_mgr,
        session_id,
    })
}
