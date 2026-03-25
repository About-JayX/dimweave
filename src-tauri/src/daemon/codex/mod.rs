pub mod handler;
pub mod lifecycle;
pub mod session;

use crate::daemon::{gui, role_config, session_manager::SessionManager, SharedState};
use session::SessionOpts;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};

pub struct CodexHandle {
    pub inject_tx: mpsc::Sender<String>,
    process: Arc<Mutex<Option<tokio::process::Child>>>,
    session_mgr: Arc<Mutex<SessionManager>>,
    session_id: String,
}

impl CodexHandle {
    /// Inject a text message into the active Codex session.
    pub async fn inject(&self, text: String) {
        self.inject_tx.send(text).await.ok();
    }

    /// Stop the Codex process and clean up session resources.
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
    let mut mgr = SessionManager::new();
    let session_id = format!("{}", chrono::Utc::now().timestamp_millis());
    let codex_home = mgr.create_session(&session_id)?;

    // Spawn the codex process
    let child = lifecycle::start(codex_port, &codex_home, &cwd).await?;
    let child_arc = Arc::new(Mutex::new(Some(child)));
    let mgr_arc = Arc::new(Mutex::new(mgr));

    gui::emit_system_log(
        &app,
        "info",
        &format!("[Codex] started port={codex_port} role={role_id}"),
    );

    // Give app-server a moment to start listening
    tokio::time::sleep(std::time::Duration::from_millis(800)).await;

    // Resolve role config for session opts
    let (sandbox_mode, developer_instructions) =
        if let Some(rc) = role_config::get_role(&role_id) {
            (
                Some(rc.sandbox_mode.to_string()),
                Some(rc.developer_instructions.to_string()),
            )
        } else {
            (None, None)
        };

    let (inject_tx, inject_rx) = mpsc::channel::<String>(64);
    let opts = SessionOpts {
        role_id: role_id.clone(),
        cwd: cwd.clone(),
        model,
        sandbox_mode,
        developer_instructions,
    };

    // Run session in background task
    let state2 = state.clone();
    let app2 = app.clone();
    tokio::spawn(async move {
        session::run(codex_port, opts, state2, app2, inject_rx).await;
    });

    // Update daemon state with codex role
    {
        let mut s = state.write().await;
        s.codex_role = role_id.clone();
    }
    gui::emit_system_log(&app, "info", &format!("[Codex] session wired role={role_id}"));

    Ok(CodexHandle {
        inject_tx,
        process: child_arc,
        session_mgr: mgr_arc,
        session_id,
    })
}
