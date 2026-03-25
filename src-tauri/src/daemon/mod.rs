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
use tokio::sync::{mpsc, RwLock};

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
    },
    /// Stop the current Codex session.
    StopCodex,
    /// Update which role Claude is playing (affects routing).
    SetClaudeRole(String),
}

/// Create the command channel.  Call before spawning to avoid the DaemonSender race.
pub fn channel() -> (mpsc::Sender<DaemonCmd>, mpsc::Receiver<DaemonCmd>) {
    mpsc::channel(64)
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

            DaemonCmd::LaunchCodex { role_id, cwd, model } => {
                if let Some(h) = codex_handle.take() {
                    h.stop().await;
                    let mut daemon = state.write().await;
                    daemon.codex_inject_tx = None;
                    drop(daemon);
                    gui::emit_agent_status(&app, "codex", false, None);
                }
                match codex::start(role_id, cwd, model, state.clone(), app.clone(), 4500).await {
                    Ok(h) => {
                        codex_handle = Some(h);
                    }
                    Err(e) => {
                        gui::emit_system_log(
                            &app,
                            "error",
                            &format!("[Daemon] Codex start failed: {e}"),
                        );
                    }
                }
            }

            DaemonCmd::StopCodex => {
                if let Some(h) = codex_handle.take() {
                    h.stop().await;
                }
                let mut daemon = state.write().await;
                daemon.codex_inject_tx = None;
                drop(daemon);
                gui::emit_agent_status(&app, "codex", false, None);
            }

            DaemonCmd::SetClaudeRole(role) => {
                state.write().await.claude_role = role;
            }
        }
    }
}
