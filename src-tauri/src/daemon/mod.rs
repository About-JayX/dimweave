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

/// Start the daemon.  Returns an mpsc sender for driving it from Tauri commands.
///
/// Spawns:
/// - WS control server on :4502 (bridge ↔ daemon)
/// - Command handler task (processes `DaemonCmd`)
pub async fn start(app: AppHandle) -> mpsc::Sender<DaemonCmd> {
    let state: SharedState = Arc::new(RwLock::new(DaemonState::new()));

    // WS control server — bridge processes connect here
    {
        let s = state.clone();
        let a = app.clone();
        tokio::spawn(async move {
            control::server::start(4502, s, a).await;
        });
    }

    let (cmd_tx, mut cmd_rx) = mpsc::channel::<DaemonCmd>(64);

    {
        let s = state.clone();
        let a = app.clone();
        tokio::spawn(async move {
            let mut codex_handle: Option<codex::CodexHandle> = None;

            while let Some(cmd) = cmd_rx.recv().await {
                match cmd {
                    DaemonCmd::SendMessage(msg) => {
                        routing::route_message(&s, &a, msg).await;
                    }

                    DaemonCmd::LaunchCodex { role_id, cwd, model } => {
                        // Stop existing session first
                        if let Some(h) = codex_handle.take() {
                            h.stop().await;
                        }
                        match codex::start(role_id, cwd, model, s.clone(), a.clone(), 4500).await {
                            Ok(h) => {
                                codex_handle = Some(h);
                            }
                            Err(e) => {
                                gui::emit_system_log(
                                    &a,
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
                    }

                    DaemonCmd::SetClaudeRole(role) => {
                        s.write().await.claude_role = role;
                    }
                }
            }
        });
    }

    cmd_tx
}
